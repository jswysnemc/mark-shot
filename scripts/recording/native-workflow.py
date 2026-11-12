#!/usr/bin/env python3
"""【录制验证】【私有桌面】运行真实进程内录制测试，不连接用户显示或音频会话"""

import argparse
import importlib.util
import json
import math
from pathlib import Path
import struct
import subprocess
import sys
import time
import wave

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from wayland_headless_runtime import HeadlessRuntime


class Processes:
    """【录制验证】【进程资源】仅管理当前测试启动的服务、日志与明确环境"""

    def __init__(self, root, environment):
        """【录制验证】【资源创建】按 root 与 environment 保存隔离路径，不启动任何服务"""
        self.root = root
        self.environment = environment
        self.processes = []
        self.logs = []

    def launch(self, arguments, name, stdout=None):
        """【录制验证】【服务启动】按 arguments 和 name 启动测试服务，返回子进程句柄"""
        log = (self.root / f"{name}.log").open("w")
        self.logs.append(log)
        process = subprocess.Popen(arguments, env=self.environment, stdout=stdout or log, stderr=log)
        self.processes.append(process)
        return process

    def audio(self, pulse):
        """【录制验证】【音频隔离】按 pulse 启动私有服务和虚拟声卡，播放固定正弦波，不使用实际设备"""
        self.environment.pop("PIPEWIRE_REMOTE", None)
        self.environment["PIPEWIRE_RUNTIME_DIR"] = self.environment["XDG_RUNTIME_DIR"]
        bus = self.launch(["dbus-daemon", "--session", "--nofork", "--print-address=1"], "dbus", subprocess.PIPE)
        self.environment["DBUS_SESSION_BUS_ADDRESS"] = bus.stdout.readline().decode().strip()
        self.launch(["pipewire"], "pipewire")
        time.sleep(0.4)
        config = Path(self.environment["XDG_CONFIG_HOME"]) / "wireplumber/wireplumber.conf.d"
        config.mkdir(parents=True)
        config.joinpath("private.conf").write_text(
            "wireplumber.profiles = { main = { monitor.alsa = disabled monitor.bluez = disabled "
            "monitor.bluez-midi = disabled monitor.libcamera = disabled monitor.v4l2 = disabled } }")
        self.launch(["wireplumber"], "wireplumber")
        time.sleep(0.8)
        subprocess.run(["pw-cli", "create-node", "adapter", "{ factory.name=support.null-audio-sink "
                        "node.name=mark-shot-test-sink media.class=Audio/Sink object.linger=true audio.position=[FL FR] }"],
                       env=self.environment, check=True, stdout=subprocess.DEVNULL, timeout=5)
        time.sleep(0.5)
        nodes = json.loads(subprocess.check_output(["pw-dump"], env=self.environment, timeout=5))
        sink = next(node["id"] for node in nodes if node.get("info", {}).get("props", {}).get("node.name") == "mark-shot-test-sink")
        subprocess.run(["wpctl", "set-default", str(sink)], env=self.environment, check=True, timeout=5)
        tone = self.root / "tone.wav"
        with wave.open(str(tone), "wb") as output:
            output.setnchannels(2)
            output.setsampwidth(2)
            output.setframerate(48000)
            output.writeframes(b"".join(struct.pack("<hh", *([int(math.sin(index * math.tau * 440 / 48000) * 8000)] * 2)) for index in range(48000 * 12)))
        self.launch(["pw-play", "--target=mark-shot-test-sink", str(tone)], "tone")
        if pulse:
            self.launch(["pipewire-pulse"], "pulse")
            time.sleep(0.5)
            self.environment.update(PIPEWIRE_REMOTE="missing-test-server", PULSE_SERVER="unix:" + str(Path(self.environment["XDG_RUNTIME_DIR"]) / "pulse/native"))
        self.environment["MARK_SHOT_TEST_AUDIO"] = "system"

    def close(self):
        """【录制验证】【服务回收】无参数，仅关闭本实例拥有的服务，不按名称终止其他进程"""
        for process in reversed(self.processes):
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=3)
        for log in self.logs:
            log.close()


def verify_audio(path):
    """【录制验证】【内容校验】独立解码 path 的中后段，必须包含测试音而非空轨或连续静音"""
    import array
    raw = subprocess.check_output(["ffmpeg", "-v", "error", "-i", str(path), "-f", "f32le", "-ac", "1", "-ar", "48000", "-"], timeout=30)
    samples = array.array("f")
    samples.frombytes(raw)
    for second in (1.0, 2.5, 3.5):
        block = samples[int(second * 48000):int((second + 0.25) * 48000)]
        assert len(block) == 12000
        rms = math.sqrt(sum(value * value for value in block) / len(block))
        assert 0.18 < rms < 0.30, (second, rms)


def run_test(options, environment, directory):
    """【录制验证】【真实会话】按 options 的测试程序录制，保存证据并独立验证容器时长、颜色格式和完整解码"""
    evidence = directory / "evidence"
    environment.update(MARK_SHOT_PRIVATE_RECORDING_TEST="1", MARK_SHOT_MEDIA_LIBRARY=str(options.media_library.resolve()), MARK_SHOT_MEDIA_EVIDENCE=str(evidence))
    with (directory / "result.log").open("w") as log:
        subprocess.run([str(options.test_binary.resolve()), "private_native_recording_pause_resume", "--ignored", "--nocapture"], env=environment, stdout=log, stderr=log, timeout=30, check=True)
    path = evidence / "desktop.mp4"
    probe = json.loads(subprocess.check_output(["ffprobe", "-v", "error", "-show_format", "-show_streams", "-of", "json", str(path)], timeout=10))
    assert 3.5 < float(probe["format"]["duration"]) < 4.8, probe
    video = next(stream for stream in probe["streams"] if stream["codec_type"] == "video")
    assert (video["width"], video["height"], video["codec_name"]) == (320, 240, "h264"), video
    subprocess.run(["ffmpeg", "-v", "error", "-xerror", "-i", str(path), "-enc_time_base", "demux", "-fps_mode", "passthrough", "-f", "null", "-"], check=True, timeout=30)
    if options.mode in ("audio", "pulse"):
        verify_audio(path)
    (evidence / "probe.json").write_text(json.dumps(probe, indent=2))
    print(directory)


def main():
    """【录制验证】【命令入口】无参数，读取模式、测试程序、库与全新目录，隔离执行后释放全部桌面服务"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=("x11", "wayland", "gpu", "audio", "pulse"), required=True)
    parser.add_argument("--test-binary", type=Path, required=True)
    parser.add_argument("--media-library", type=Path, required=True)
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--sway", type=Path, default=Path("target/test-tools/sway/usr/bin/sway"))
    options = parser.parse_args()
    directory = options.directory.resolve()
    if options.mode in ("wayland", "gpu"):
        with HeadlessRuntime(directory, options.sway, renderer="gles2" if options.mode == "gpu" else "pixman") as environment:
            environment.update(MARK_SHOT_TEST_OUTPUT="HEADLESS-2", MARK_SHOT_TEST_ENCODER="hardware" if options.mode == "gpu" else "software")
            if options.mode == "gpu":
                environment["MARK_SHOT_TEST_CAPTURE_PATH"] = "DMA-BUF"
            run_test(options, environment, directory)
    else:
        spec = importlib.util.spec_from_file_location("x11_runtime", Path(__file__).resolve().parents[1] / "x11-desktop-workflow.py")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        directory.mkdir(parents=True, exist_ok=False, mode=0o700)
        runtime = module.Runtime(directory, Path("/nonexistent"))
        services = Processes(directory, runtime.environment)
        try:
            runtime.start()
            if options.mode in ("audio", "pulse"):
                services.audio(options.mode == "pulse")
            run_test(options, runtime.environment, directory)
        finally:
            services.close()
            runtime.close()


if __name__ == "__main__":
    main()
