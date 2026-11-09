#!/usr/bin/env python3
"""【X11回归】【隔离工作流】在独立Xvfb验证实际捕获、编辑输入、剪贴板及普通窗口"""

import argparse
import io
import json
import os
from pathlib import Path
import select
import subprocess
import tempfile
import time

from PIL import Image, ImageChops, ImageGrab


class Runtime:
    """【X11回归】【会话资源】仅持有本次启动的进程与私有显示环境"""

    def __init__(self, root, binary):
        """【X11回归】【状态构造】按root和binary记录测试目录与应用路径，无返回值"""
        self.root = root
        self.binary = binary
        self.processes = []
        self.logs = []
        self.environment = dict(os.environ)

    def start(self):
        """【X11回归】【独立显示】无参数，启动Xvfb并读取自动分配编号，无返回值"""
        for key in ("WAYLAND_DISPLAY", "WAYLAND_SOCKET", "NIRI_SOCKET", "SWAYSOCK", "MARK_SHOT_CAPTURE", "XAUTHORITY", "XMODIFIERS", "DBUS_SESSION_BUS_ADDRESS"):
            self.environment.pop(key, None)
        self.environment.update(XDG_SESSION_TYPE="x11", MARK_SHOT_BACKEND="auto", XDG_RUNTIME_DIR=str(self.root), XDG_CONFIG_HOME=str(self.root / "config"), XDG_STATE_HOME=str(self.root / "state"), XDG_CACHE_HOME=str(self.root / "cache"), XDG_DATA_HOME=str(self.root / "data"))
        (self.root / "test.toml").write_text('[startup]\ninclude_cursor = false\n[notifications]\nenabled = false\n[history]\nenabled = false\n')
        reader, writer = os.pipe()
        try:
            self.launch(["Xvfb", "-displayfd", str(writer), "-screen", "0", "1024x768x24", "-nolisten", "tcp", "-noreset"], "xvfb", pass_fds=(writer,))
            os.close(writer)
            writer = None
            if not select.select([reader], [], [], 8)[0]:
                raise AssertionError("private Xvfb did not become ready")
            number = os.read(reader, 64).decode().strip()
            if not number.isdecimal():
                raise AssertionError("invalid private Xvfb display")
            self.environment["DISPLAY"] = f":{number}"
            self.environment["MARK_SHOT_TEST_X11_DISPLAY"] = f":{number}"
        finally:
            os.close(reader)
            if writer is not None:
                os.close(writer)

    def launch(self, arguments, name, **kwargs):
        """【X11回归】【进程启动】按arguments和name启动独立进程并保存日志，返回进程句柄"""
        if arguments[0] == str(self.binary):
            arguments = [arguments[0], "--config", str(self.root / "test.toml"), *arguments[1:]]
        log = (self.root / f"{name}.log").open("wb")
        self.logs.append(log)
        process = subprocess.Popen(arguments, env=self.environment, stdout=kwargs.pop("stdout", log), stderr=log, **kwargs)
        self.processes.append(process)
        return process

    def command(self, *arguments, **kwargs):
        """【X11回归】【有界命令】按arguments执行私有显示命令，返回标准输出，失败保留诊断"""
        if arguments[0] == str(self.binary):
            arguments = (arguments[0], "--config", str(self.root / "test.toml"), *arguments[1:])
        return subprocess.run(arguments, env=self.environment, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=15, **kwargs).stdout

    def window(self, process):
        """【X11回归】【窗口确认】按process等待实际可见窗口，返回唯一窗口编号"""
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            if process.poll() is not None:
                raise AssertionError(f"application exited before mapping: {process.returncode}")
            result = subprocess.run(["xdotool", "search", "--onlyvisible", "--pid", str(process.pid)], env=self.environment, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=2)
            if result.returncode == 0:
                windows = result.stdout.decode().splitlines()
                if len(windows) == 1:
                    return windows[0]
            time.sleep(0.03)
        raise AssertionError("application did not map a unique window")

    def close(self):
        """【X11回归】【资源释放】无参数，仅关闭本次进程句柄与日志，不操作其他桌面"""
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


def start_window_manager(runtime, executable):
    """【X11回归】【管理器就绪】按runtime及executable启动管理器，校验进程与根窗口协议后返回句柄"""
    process = runtime.launch([str(executable.resolve()), "--sm-disable"], "window-manager")
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise AssertionError("window manager exited; see window-manager.log and its data/theme paths")
        if b"window id" in runtime.command("xprop", "-root", "_NET_SUPPORTING_WM_CHECK"):
            return process
        time.sleep(0.05)
    raise AssertionError("window manager did not publish its EWMH identity")


def capture(runtime, name, *arguments):
    """【X11回归】【无界面捕获】按runtime、name和arguments执行捕获，返回PNG内容并校验JSON报告"""
    path = runtime.root / f"{name}.png"
    report = json.loads(runtime.command(str(runtime.binary), "--capture-to", str(path), "--nonotify", *arguments))
    if report.get("error"):
        raise AssertionError(report)
    return Image.open(path).convert("RGB")


def verify(runtime):
    """【X11回归】【功能校验】按runtime验证真实协议工作流，返回通过项目列表"""
    binary = str(runtime.binary)
    runtime.launch(["xmessage", "-geometry", "320x180+70+90", "-bw", "0", "-bg", "#357ac2", "-fg", "white", "X11 capture fixture"], "fixture")
    deadline = time.monotonic() + 5
    fixture_window = None
    while time.monotonic() < deadline:
        result = subprocess.run(["xdotool", "search", "--onlyvisible", "--class", "Xmessage"], env=runtime.environment, capture_output=True, timeout=2)
        if result.returncode == 0:
            fixture_window = result.stdout.decode().splitlines()[0]
            break
        time.sleep(0.03)
    if fixture_window is None:
        raise AssertionError("fixture window did not map")
    time.sleep(0.1)
    displays = json.loads(runtime.command(binary, "--list-displays"))
    if not displays:
        raise AssertionError("no X11 outputs discovered")
    expected = ImageGrab.grab(xdisplay=runtime.environment["DISPLAY"]).convert("RGB")
    expected.save(runtime.root / "expected.png")
    actual = capture(runtime, "desktop", "--all-outputs")
    if actual.size != (1024, 768) or ImageChops.difference(expected, actual).getbbox():
        raise AssertionError("X11 capture differs from the actual framebuffer")
    region = capture(runtime, "region", "--region", "31,47,173,91")
    if region.size != (173, 91) or ImageChops.difference(expected.crop((31, 47, 204, 138)), region).getbbox():
        raise AssertionError("X11 region crop differs from the framebuffer")
    with_cursor = capture(runtime, "cursor", "--include-cursor")
    if with_cursor.size != actual.size or not ImageChops.difference(with_cursor, actual).getbbox():
        raise AssertionError("cursor capture did not preserve size and add a pointer")
    runtime.command("xdotool", "windowfocus", fixture_window)
    time.sleep(0.1)
    active_expected = ImageGrab.grab(xdisplay=runtime.environment["DISPLAY"]).convert("RGB")
    geometry = dict(line.split("=", 1) for line in runtime.command("xdotool", "getwindowgeometry", "--shell", fixture_window).decode().splitlines())
    active = Image.open(io.BytesIO(runtime.command(binary, "--window", "--to", "o", "--nonotify"))).convert("RGB")
    left, top, width, height = (int(geometry[key]) for key in ("X", "Y", "WIDTH", "HEIGHT"))
    active.save(runtime.root / "active.png")
    active_expected.crop((left, top, left + width, top + height)).save(runtime.root / "active-expected.png")
    if active.size != (width, height) or ImageChops.difference(active, active_expected.crop((left, top, left + width, top + height))).getbbox():
        raise AssertionError(f"active window capture differs from its visible client pixels: {active.size}, {geometry}")

    for text, payload in [(True, "X11 中文 selection\n".encode() * 9000), (False, (runtime.root / "desktop.png").read_bytes())]:
        arguments = [binary, "--clipboard-daemon"] + (["text"] if text else [])
        process = runtime.launch(arguments, f"clipboard-{text}", stdin=subprocess.PIPE, stdout=subprocess.PIPE)
        process.stdin.write(payload)
        process.stdin.close()
        if not select.select([process.stdout], [], [], 5)[0] or process.stdout.readline() != b"\n":
            raise AssertionError("clipboard ownership not confirmed")
        target = "UTF8_STRING" if text else "image/png"
        copied = runtime.command("xclip", "-selection", "clipboard", "-target", target, "-out")
        if copied != payload:
            raise AssertionError("clipboard byte roundtrip differs")
        if text:
            previous = process
        else:
            previous.wait(timeout=5)
            if previous.returncode != 0:
                raise AssertionError("clipboard owner did not terminate on replacement")

    pin = runtime.launch([binary, "--pin-clipboard"], "pin")
    pin_window = runtime.window(pin)
    runtime.command("xdotool", "windowfocus", pin_window)
    runtime.command("xdotool", "key", "Escape")
    pin.wait(timeout=5)
    if pin.returncode != 0:
        raise AssertionError("pin failed to close cleanly")

    settings = runtime.launch([binary, "--settings"], "settings")
    settings_window = runtime.window(settings)
    runtime.command("xdotool", "windowfocus", settings_window)
    runtime.command("xdotool", "key", "Escape")
    settings.wait(timeout=5)
    if settings.returncode != 0:
        raise AssertionError("settings failed to close cleanly")

    editor = runtime.launch([binary, "--fullscreen", "--nonotify"], "editor")
    editor_window = runtime.window(editor)
    runtime.command("xdotool", "windowfocus", editor_window)
    time.sleep(0.2)
    runtime.command("xdotool", "key", "ctrl+c")
    editor.wait(timeout=10)
    if editor.returncode != 0:
        raise AssertionError("editor copy failed")
    copied = runtime.command("xclip", "-selection", "clipboard", "-target", "image/png", "-out")
    if Image.open(io.BytesIO(copied)).size != (1024, 768):
        raise AssertionError("editor copy changed image dimensions")
    annotated = runtime.launch([binary, "--fullscreen", "--nonotify"], "annotation")
    annotation_window = runtime.window(annotated)
    runtime.command("xdotool", "windowfocus", annotation_window)
    time.sleep(0.2)
    runtime.command("xdotool", "key", "r")
    runtime.command("xdotool", "mousemove", "200", "200", "mousedown", "1", "mousemove", "410", "330", "mouseup", "1")
    runtime.command("xdotool", "key", "ctrl+c")
    annotated.wait(timeout=10)
    pixels = Image.open(io.BytesIO(runtime.command("xclip", "-selection", "clipboard", "-target", "image/png", "-out"))).convert("RGB")
    if pixels.size != actual.size or not ImageChops.difference(pixels, actual).getbbox():
        raise AssertionError("rectangle annotation did not change exported pixels")
    return ["output discovery", "framebuffer pixels", "region pixels", "cursor capture", "active window pixels", "UTF-8 INCR clipboard", "PNG clipboard", "selection owner lifecycle", "pin clipboard", "settings window", "editor copy shortcut", "rectangle annotation export"]


def main():
    """【X11回归】【命令入口】无参数，解析程序与输出目录，验证后写入实际结果，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/debug/mark-shot"))
    parser.add_argument("--root", type=Path)
    parser.add_argument("--backend-tests", type=Path)
    parser.add_argument("--input-tests", type=Path)
    parser.add_argument("--wm", type=Path)
    options = parser.parse_args()
    root = options.root or Path(tempfile.mkdtemp(prefix="mark-shot-x11-"))
    root.mkdir(parents=True, exist_ok=True, mode=0o700)
    root.chmod(0o700)
    (root / "result.json").unlink(missing_ok=True)
    runtime = Runtime(root.resolve(), options.binary.resolve())
    try:
        runtime.start()
        manager = start_window_manager(runtime, options.wm) if options.wm else None
        checks = verify(runtime)
        if options.backend_tests:
            output = runtime.command(str(options.backend_tests.resolve()), "--ignored", "--test-threads=1")
            (root / "backend-tests.log").write_bytes(output)
            checks.extend(["continuous capture", "scroll focus and pixel isolation", "event timeout and wake"])
        if options.input_tests:
            runtime.environment["XMODIFIERS"] = "@im=mark-shot-test"
            output = runtime.command(str(options.input_tests.resolve()), "--ignored", "--test-threads=1")
            (root / "input-tests.log").write_bytes(output)
            checks.append("XIM preedit, commit, fallback and stale target isolation")
        assert manager is None or manager.poll() is None, "window manager exited during verification"
        result = {"checks": checks, "display": runtime.environment["DISPLAY"]}
        (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result))
    finally:
        runtime.close()


if __name__ == "__main__":
    main()
