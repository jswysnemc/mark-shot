"""【录制界面】【真实流程验证】私有 Niri 中验证设置、倒计时和截图内控制，保存截图与实际录制成品。"""

import argparse
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

from PIL import Image

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
sys.path.insert(0, str(SCRIPTS / "performance"))
from capture_interaction import key
from compare_capture_memory import output_size
from wayland_test_runtime import TestRuntime
from wayland_virtual_pointer import VirtualPointer, transform_for_pointer


@contextmanager
def private_bus(root, environment):
    """【录制界面】【消息总线隔离】按 root 与 environment 创建私有总线并返回环境，退出或失败后关闭本实例服务。"""
    with (root / "dbus.log").open("w") as log:
        process = subprocess.Popen(["dbus-daemon", "--session", "--nofork", "--print-address=1"],
                                   env=environment, stdout=subprocess.PIPE, stderr=log)
        try:
            environment = dict(environment, DBUS_SESSION_BUS_ADDRESS=process.stdout.readline().decode().strip())
            if not environment["DBUS_SESSION_BUS_ADDRESS"]:
                raise RuntimeError("private message bus did not start")
            yield environment
        finally:
            process.terminate()
            process.wait(timeout=5)


def wait_for(read, accept, description, timeout=10):
    """【录制界面】【条件等待】按 read 获取状态直到 accept 通过，description 用于超时错误，返回最后状态。"""
    deadline = time.monotonic() + timeout
    value = None
    while time.monotonic() < deadline:
        value = read()
        if accept(value):
            return value
        time.sleep(.05)
    raise RuntimeError(f"{description}: {value}")


class Workflow:
    """【录制界面】【私有流程】持有当前验证创建的进程与输入设备，不连接其他桌面或配置。"""

    def __init__(self, binary, media, root, environment, size, pointer):
        """【录制界面】【流程构造】按 binary、media、root、environment、size 和 pointer 建立验证状态，不启动程序。"""
        self.binary, self.root, self.size, self.pointer = binary, root, size, pointer
        self.environment = dict(environment, MARK_SHOT_MEDIA_LIBRARY=str(media), MARK_SHOT_LOG="info")
        self.config = root / "app.toml"
        self.config.write_text('[desktop]\ntray_enabled=false\nshortcuts_enabled=false\n'
                               '[history]\nenabled=false\n[ui]\nlanguage="zh-CN"\n'
                               '[startup]\nselection_loupe=false\n'
                               '[recording]\nencoder="software"\naudio="off"\nfps=12\n'
                               'countdown=3\ndirectory=' + json.dumps(str(root / "videos")) + '\n')
        self.processes, self.logs = [], []

    def launch(self, name, arguments):
        """【录制界面】【程序启动】按 name 与 arguments 启动当前配置的程序，返回进程，日志只写入 root。"""
        log = (self.root / f"{name}.log").open("w")
        self.logs.append(log)
        process = subprocess.Popen([str(self.binary), "--config", str(self.config), *arguments],
                                   env=self.environment, stdout=log, stderr=log)
        self.processes.append(process)
        return process

    def windows(self):
        """【录制界面】【窗口查询】无参数，返回私有合成器中当前录制设置窗口，截图与倒计时使用独立图层。"""
        values = json.loads(subprocess.check_output(["niri", "msg", "-j", "windows"], env=self.environment))
        return [value for value in values if value.get("app_id") == "mark-shot-recording"]

    def status(self):
        """【录制界面】【会话查询】无参数，通过真实命令入口读取当前配置的录制状态，返回 JSON。"""
        return json.loads(subprocess.check_output([str(self.binary), "--config", str(self.config), "--recording-status"],
                                                env=self.environment, stderr=subprocess.DEVNULL, timeout=5))

    def phase(self, wanted):
        """【录制界面】【阶段等待】按 wanted 等待真实后台阶段，返回状态快照，失败直接终止验证。"""
        return wait_for(self.status, lambda value: value["phase"] == wanted, f"expected recording phase {wanted}")

    def screenshot(self, name):
        """【录制界面】【证据截图】按 name 保存私有输出实际像素，返回图片路径。"""
        path = self.root / f"{name}.png"
        subprocess.run(["grim", "-o", "winit", str(path)], env=self.environment, check=True)
        return path

    def click(self, x, y):
        """【录制界面】【点击控件】按逻辑 x、y 向已验证私有指针发送完整点击，无返回值。"""
        self.pointer.move_abs(*transform_for_pointer(x, y, *self.size, "flipped180"))
        self.pointer.click()

    def setup(self, name):
        """【录制界面】【准备窗口】按 name 打开录制设置并浮动定位，返回进程，位置与大小由私有合成器确认。"""
        process = self.launch(name, ["--record"])
        window = wait_for(self.windows, bool, "recording setup did not open")[0]
        if not window.get("is_floating"):
            subprocess.run(["niri", "msg", "action", "toggle-window-floating", "--id", str(window["id"])],
                           env=self.environment, check=True)
        subprocess.run(["niri", "msg", "action", "move-floating-window", "--id", str(window["id"]), "--x", "40", "--y", "60"],
                       env=self.environment, check=True)
        wait_for(self.windows, lambda values: bool(values) and values[0]["layout"]["window_size"] == [400, 300],
                 "setup did not apply its compact content size")
        time.sleep(.25)
        return process

    def capture(self, name):
        """【录制界面】【截图启动】按 name 启动真实截图界面，等待原生首帧，返回进程。"""
        process = self.launch(name, ["--speed"])
        path = self.root / f"{name}.log"
        wait_for(lambda: path.read_text(), lambda text: "frames staged + flushed" in text,
                 "screenshot overlay did not present")
        time.sleep(1.1)
        return process

    def recording_control_y(self):
        """【录制界面】【控制位置】无参数，等待真实布局日志确认录制栏避开编辑工具栏，返回按钮中心的逻辑纵坐标。"""
        path = self.root / "recording-capture.log"
        values = wait_for(lambda: re.findall(r"【截图录制】【控制布局】monitor=0 top=([\d.]+)", path.read_text()),
                          lambda values: bool(values) and float(values[-1]) > 12,
                          "recording controls still overlap the editing toolbar")
        return float(values[-1]) + 19

    def run(self):
        """【录制界面】【完整验证】无参数，执行设置、倒计时取消和后台录制控制，返回含成品与像素证据的结果。"""
        # 1. 【录制界面】【设置布局】展开高级参数后高度变化，收起后恢复紧凑尺寸
        recorder = self.setup("cancel-setup")
        self.screenshot("setup-compact")
        self.click(240, 60 + 44 + 4 * 32 + 16)
        expanded = wait_for(self.windows, lambda values: bool(values) and values[0]["layout"]["window_size"][1] == 396,
                            "advanced options did not resize")
        self.screenshot("setup-expanded")
        subprocess.run(["niri", "msg", "action", "set-window-height", "--id", str(expanded[0]["id"]), "350"],
                       env=self.environment, check=True)
        wait_for(self.windows, lambda values: bool(values) and values[0]["layout"]["window_size"][1] == 350,
                 "externally resized setup did not shrink")
        time.sleep(.2)
        self.click(240, 60 + 44 + 4 * 32 + 16)
        wait_for(self.windows, lambda values: bool(values) and values[0]["layout"]["window_size"][1] == 396,
                 "shrinking setup did not collapse advanced options")
        self.click(240, 60 + 44 + 4 * 32 + 16)
        wait_for(self.windows, lambda values: bool(values) and values[0]["layout"]["window_size"][1] == 300,
                 "compact options did not restore size")
        # 2. 【录制界面】【倒计时取消】GIF 准备期间设置窗口已关闭，截图顶部按钮取消倒计时，不创建任何成品
        self.click(40 + 336, 60 + 92)
        key(self.environment, "End")
        key(self.environment, "Return")
        key(self.environment, "Return")
        countdown = self.phase("countdown")
        assert countdown["format"] == "gif", countdown
        assert not self.windows()
        self.screenshot("countdown-widget")
        capture = self.capture("countdown-capture")
        self.screenshot("countdown-status")
        self.click(self.size[0] / 2 + 186, 31)
        self.phase("idle")
        wait_for(self.windows, bool, "cancelled countdown did not return to setup")
        assert not list((self.root / "videos").glob("*"))
        capture.terminate()
        capture.wait(timeout=5)
        recorder.terminate()
        recorder.wait(timeout=5)
        # 3. 【录制界面】【后台采集】确认开始后仅短暂显示倒计时，采集阶段没有准备窗口或控制窗口
        recorder = self.setup("video-setup")
        self.click(40 + 136, 60 + 92)
        key(self.environment, "Home")
        key(self.environment, "Return")
        key(self.environment, "Return")
        recording = self.phase("recording")
        assert recording["format"] == "mp4", recording
        assert not self.windows() and recorder.poll() is None
        time.sleep(.5)
        capture = self.capture("recording-capture")
        self.screenshot("recording-intro")
        # 4. 【录制界面】【选区后控制】全屏截图选区隐藏起始提示，紧凑录制栏仍允许暂停、继续和停止保存
        self.click((self.size[0] - 484) / 2 + 105, 31)
        time.sleep(.3)
        control_y = self.recording_control_y()
        self.screenshot("recording-selected")
        self.click(self.size[0] / 2 + 34, control_y)
        paused = self.phase("paused")
        time.sleep(1.1)
        self.screenshot("recording-paused")
        assert self.status()["elapsed_ms"] == paused["elapsed_ms"]
        self.click(self.size[0] / 2 + 34, control_y)
        self.phase("recording")
        time.sleep(1.1)
        self.click(self.size[0] / 2 + 124, control_y)
        finished = self.phase("finished")
        path = Path(finished["output"])
        assert finished["format"] == "mp4" and path.suffix == ".mp4", finished
        assert path.is_file() and path.stat().st_size > 1000
        capture.terminate()
        capture.wait(timeout=5)
        recorder.wait(timeout=8)
        # 5. 【录制界面】【成品校验】独立解码完整视频，首帧不能含白色倒计时小组件
        decoded = subprocess.run(["ffmpeg", "-v", "error", "-xerror", "-i", str(path), "-fps_mode", "passthrough",
                                  "-enc_time_base", "demux", "-f", "null", "-"],
                                 check=True, timeout=30, capture_output=True, text=True)
        assert not decoded.stderr, decoded.stderr
        frames = json.loads(subprocess.check_output(["ffprobe", "-v", "error", "-select_streams", "v:0",
                                                    "-show_frames", "-show_entries", "frame=best_effort_timestamp_time",
                                                    "-of", "json", str(path)], timeout=30))["frames"]
        timestamps = [float(frame["best_effort_timestamp_time"]) for frame in frames]
        assert timestamps and all(left < right for left, right in zip(timestamps, timestamps[1:])), timestamps
        first = self.root / "first-recorded-frame.png"
        subprocess.run(["ffmpeg", "-v", "error", "-i", str(path), "-frames:v", "1", str(first)], check=True, timeout=10)
        with Image.open(first) as image:
            pixels = image.convert("RGB").tobytes()
            white = sum(min(pixels[index:index + 3]) > 230 for index in range(0, len(pixels), 3))
        assert white < 500, f"first recorded frame still contains countdown UI: {white} white pixels"
        return {"complete": True, "logical_size": list(self.size), "expanded_size": expanded[0]["layout"]["window_size"],
                "countdown_format": countdown["format"], "pause_elapsed_ms": paused["elapsed_ms"],
                "recording_controls_y": control_y,
                "decoded_frames": len(frames),
                "recording_format": finished["format"], "output": str(path), "first_frame_white_pixels": white}

    def close(self):
        """【录制界面】【进程回收】无参数，只关闭本实例创建的程序与日志，保持用户桌面和录制不变。"""
        for process in reversed(self.processes):
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=5)
        for log in self.logs:
            log.close()


def main():
    """【录制界面】【命令入口】读取 binary、media、root 和 scale，在全新私有桌面执行验证并保存结果。"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--media", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--scale", type=float, default=1.0)
    options = parser.parse_args()
    root = options.root.resolve()
    root.mkdir(parents=True, exist_ok=False, mode=0o700)
    original = dict(os.environ)
    floating = '\nwindow-rule {\n    match app-id="^mark-shot-recording$"\n    open-floating true\n}\n'
    with TestRuntime(root / "desktop", options.scale, extra_config=floating) as environment, private_bus(root, environment) as environment:
        if options.scale == 1.0:
            output_size(environment, (1270, 720))
        outputs = json.loads(subprocess.check_output(["niri", "msg", "-j", "outputs"], env=environment))
        logical = outputs["winit"]["logical"]
        size = (logical["width"], logical["height"])
        os.environ.update(environment)
        try:
            with VirtualPointer() as pointer:
                workflow = Workflow(options.binary.resolve(), options.media.resolve(), root, environment, size, pointer)
                try:
                    result = workflow.run()
                    result.update(scale=options.scale, binary=str(options.binary.resolve()),
                                  sha256=hashlib.sha256(options.binary.read_bytes()).hexdigest())
                    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
                    print(json.dumps(result, ensure_ascii=False), flush=True)
                finally:
                    workflow.close()
        finally:
            os.environ.clear()
            os.environ.update(original)


if __name__ == "__main__":
    main()
