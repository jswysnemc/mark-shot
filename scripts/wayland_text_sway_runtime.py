#!/usr/bin/env python3
"""【文字回归】【无界面运行器】仅启动本次单输出 Sway，与嵌套 niri 对照原生菜单和编辑输入"""

import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import time


CONFIG = '''
xwayland disable
output HEADLESS-1 mode 1280x800 position 0 0 scale 1.2 transform flipped-180
seat seat0 attach *
focus_follows_mouse yes
default_border none
'''


class TextSwayRuntime:
    """【文字回归】【会话生命周期】管理 root 下配置与日志，binary 只属于本次测试进程"""

    def __init__(self, root, binary, scale=1.2, width=1280, height=800, transform="flipped-180"):
        """【文字回归】【状态构造】保存 root、binary、比例、物理宽高和方向；不启动服务，无返回值"""
        if not 0.5 <= scale <= 4.0:
            raise ValueError("private output scale must be between 0.5 and 4")
        self.scale = scale
        if not 320 <= width <= 7680 or not 240 <= height <= 4320:
            raise ValueError("private output dimensions are outside the test limits")
        if transform not in ("normal", "90", "180", "270", "flipped", "flipped-90", "flipped-180", "flipped-270"):
            raise ValueError("invalid private output transform")
        self.width, self.height, self.transform = width, height, transform
        self.root, self.binary = root.resolve(), binary.resolve()
        self.process = None
        self.log = None
        self.runtime = None

    def __enter__(self):
        """【文字回归】【服务启动】创建私有套接字与单输出合成器，返回经过输出校验的环境；失败释放已创建资源"""
        try:
            self.root.mkdir(parents=True, exist_ok=False, mode=0o700)
            config = self.root / "config.conf"
            config.write_text(CONFIG.replace("scale 1.2", f"scale {self.scale}")
                              .replace("1280x800", f"{self.width}x{self.height}")
                              .replace("flipped-180", self.transform))
            self.runtime = tempfile.TemporaryDirectory(prefix="mark-shot-text-wl-")
            root = Path(self.runtime.name)
            environment = dict(os.environ)
            for name in ("DISPLAY", "WAYLAND_DISPLAY", "NIRI_SOCKET", "SWAYSOCK", "MARK_SHOT_TEST_X11_DISPLAY"):
                environment.pop(name, None)
            environment.update(XDG_RUNTIME_DIR=str(root), XDG_CONFIG_HOME=str(self.root / "config"),
                               XDG_DATA_HOME=str(self.root / "data"), XDG_STATE_HOME=str(self.root / "state"),
                               XDG_CACHE_HOME=str(self.root / "cache"),
                               WLR_BACKENDS="headless", WLR_RENDERER="pixman", WLR_HEADLESS_OUTPUTS="1", WLR_LIBINPUT_NO_DEVICES="1",
                               LD_LIBRARY_PATH=str(self.binary.parent.parent / "lib"))
            self.log = (self.root / "sway.log").open("w")
            self.process = subprocess.Popen([str(self.binary), "-c", str(config)], env=environment, stdout=subprocess.DEVNULL, stderr=self.log, start_new_session=True)
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline:
                if self.process.poll() is not None:
                    raise RuntimeError(f"private text Sway exited: {self.process.returncode}")
                sockets = list(root.glob(f"sway-ipc.*.{self.process.pid}.sock"))
                wayland = [path for path in root.glob("wayland-*") if path.is_socket()]
                if len(sockets) == 1 and len(wayland) == 1:
                    environment.update(SWAYSOCK=str(sockets[0]), WAYLAND_DISPLAY=wayland[0].name)
                    break
                time.sleep(0.04)
            else:
                raise RuntimeError("private text Sway did not create its sockets")
            outputs = json.loads(subprocess.check_output([str(self.binary.parent / "swaymsg"), "-r", "-t", "get_outputs"], env=environment))
            rotated = self.transform in ("90", "270", "flipped-90", "flipped-270")
            width, height = (self.height, self.width) if rotated else (self.width, self.height)
            if len(outputs) != 1 or outputs[0]["name"] != "HEADLESS-1" or outputs[0]["rect"]["width"] != int(width / self.scale) or outputs[0]["rect"]["height"] != int(height / self.scale):
                raise RuntimeError(f"unexpected private text output: {outputs}")
            output = outputs[0]
            logical = dict(output["rect"], scale=output["scale"], transform=output["transform"])
            metadata = {"compositor": "sway", "sway_pid": self.process.pid, "sway_socket": environment["SWAYSOCK"],
                        "wayland_display": environment["WAYLAND_DISPLAY"],
                        "outputs": {output["name"]: {"logical": logical, "modes": [output["current_mode"]], "current_mode": 0}}}
            for destination in (root / "runtime.json", self.root / "runtime.json"):
                destination.write_text(json.dumps(metadata, indent=2) + "\n")
            return environment
        except BaseException:
            self.close()
            raise

    def close(self):
        """【文字回归】【服务释放】只终止本实例进程组，随后关闭日志和临时套接字，无参数、无返回值"""
        if self.process and self.process.poll() is None:
            os.killpg(self.process.pid, signal.SIGTERM)
            try:
                self.process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                os.killpg(self.process.pid, signal.SIGKILL)
                self.process.wait(timeout=3)
        if self.log:
            self.log.close()
        if self.runtime:
            self.runtime.cleanup()

    def __exit__(self, *_arguments):
        """【文字回归】【上下文退出】关闭本次资源，不抑制 _arguments 中的异常，无返回值"""
        self.close()


def main():
    """【文字回归】【运行入口】解析私有 root、sway 和命令，执行后关闭合成器，返回命令状态"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--sway", type=Path, default=Path("target/test-tools/sway/usr/bin/sway"))
    parser.add_argument("--scale", type=float, default=1.2)
    parser.add_argument("--width", type=int, default=1280)
    parser.add_argument("--height", type=int, default=800)
    parser.add_argument("--transform", default="flipped-180")
    parser.add_argument("command", nargs=argparse.REMAINDER)
    options = parser.parse_args()
    command = options.command
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        parser.error("a test command is required after --")
    with TextSwayRuntime(options.root, options.sway, options.scale, options.width, options.height, options.transform) as environment:
        return subprocess.run(command, env=environment).returncode


if __name__ == "__main__":
    raise SystemExit(main())
