"""【单实例原生回归】【进程范围】在已经验证的私有会话中管理截图进程、配置和独立输出"""

from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
import re
import subprocess
import time

import wayland_text_format_session as session


@dataclass
class Capture:
    """【单实例原生回归】【进程证据】保存本次子进程、日志和标准输出位置，不管理其他会话"""

    process: subprocess.Popen
    log: Path
    output: Path
    directory: Path

    def wait_present(self, timeout=5):
        """【单实例原生回归】【首帧等待】按 timeout 等待全尺寸表面的编辑帧与键盘焦点，返回确认时间，排除启动占位缓冲"""
        deadline = time.monotonic() + timeout
        while True:
            assert self.process.poll() is None, self.log.read_text()
            text = self.log.read_text()
            dimensions = re.findall(r"wl_surface@(\d+)\.damage_buffer\(0, 0, (\d+), (\d+)\)", text)
            if any(int(width) > 10 and int(height) > 10
                   and f"wl_surface@{surface}.frame(" in text
                   and re.search(rf"wl_keyboard@\d+\.enter, \(\d+, wl_surface@{surface},", text)
                   for surface, width, height in dimensions):
                return time.monotonic()
            assert time.monotonic() < deadline, "capture did not present"
            time.sleep(0.01)

    def wait_text(self, text, timeout=4):
        """【单实例原生回归】【协议等待】按 text 和 timeout 等待当前日志内容，无返回值，提前退出或超时视为失败"""
        deadline = time.monotonic() + timeout
        while text not in self.log.read_text():
            assert self.process.poll() is None, self.log.read_text()
            assert time.monotonic() < deadline, f"missing client evidence: {text}"
            time.sleep(0.01)

    def cancel(self):
        """【单实例原生回归】【正常取消】发送本次私有 Escape，确认无交付且进程回收，无参数、无返回值"""
        session.key("Escape")
        assert self.process.wait(timeout=5) == 0
        assert self.output.stat().st_size == 0


@contextmanager
def launch(binary, root, name, flags=()):
    """【单实例原生回归】【进程启动】按 binary、root、name 与 flags 启动普通截图，返回本次进程证据，退出只回收当前子进程"""
    directory = root / name
    directory.mkdir(mode=0o700)
    config = directory / "config.toml"
    config.write_text('[general]\noverlay="layer"\nto="o"\n[region_detection]\nenabled=false\n[window_detection]\nenabled=false\n[history]\nenabled=false\n[notifications]\nenabled=false\n')
    environment = dict(session.environment(), WAYLAND_DEBUG="client", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/single-instance-test",
                       XDG_CONFIG_HOME=str(directory / "config"), XDG_DATA_HOME=str(directory / "data"), XDG_STATE_HOME=str(directory / "state"))
    if "NIRI_SOCKET" in environment:
        environment["NIRI_CONFIG"] = str(Path(environment["XDG_RUNTIME_DIR"]) / "niri-child/config.kdl")
    log, output = directory / "client.log", directory / "output.png"
    with log.open("w") as errors, output.open("wb") as stdout:
        process = subprocess.Popen([str(binary), "--capture", "--to", "o", "--output", str(directory / "images"), "--config", str(config), "--nonotify", "--debug", *flags],
                                   env=environment, stdout=stdout, stderr=errors)
        try:
            yield Capture(process, log, output, directory)
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)


def endpoint():
    """【单实例原生回归】【私有路径】按当前已验证运行记录返回本次合成器的控制路径，无参数，不读取用户会话"""
    import json
    private = session.environment()
    root = Path(private["XDG_RUNTIME_DIR"])
    metadata = json.loads((root / "runtime.json").read_text())
    pid = metadata["sway_pid"] if metadata.get("compositor") == "sway" else metadata["niri_pid"]
    return root / "mark-shot-instance" / f"capture-{pid}.sock"


def wait_endpoint(capture):
    """【单实例原生回归】【服务就绪】按 capture 等待本次监听路径，返回路径；此阶段允许截图尚未呈现"""
    path = endpoint()
    deadline = time.monotonic() + 3
    while not path.exists():
        assert capture.process.poll() is None, capture.log.read_text()
        assert time.monotonic() < deadline, "private control endpoint did not appear"
        time.sleep(0.01)
    return path
