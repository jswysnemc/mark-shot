#!/usr/bin/env python3
"""【GNOME回归】【独立会话】在私有D-Bus与无头GNOME验证门户、编辑器和剪贴板桥接"""

import argparse
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import tempfile
import time


def wait(predicate, message, timeout=12):
    """【GNOME回归】【有界等待】按predicate等待有效结果，超过timeout时报告message，返回有效值"""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = predicate()
        if result:
            return result
        time.sleep(0.05)
    raise AssertionError(message)


def portal_action(name, all_matches=False):
    """【GNOME回归】【授权按钮】按name在当前私有辅助功能树执行真实按钮，返回是否找到并执行"""
    import gi
    gi.require_version("Atspi", "2.0")
    from gi.repository import Atspi

    matches = []

    def visit(node, depth=0):
        """【GNOME回归】【节点遍历】按node和depth查找指定按钮，返回执行结果，限制遍历深度"""
        if depth > 24:
            return False
        try:
            action = node.get_action_iface()
            if node.get_name() == name and action and action.get_n_actions() > 0:
                matches.append(action)
            for index in range(node.get_child_count()):
                visit(node.get_child_at_index(index), depth + 1)
        except Exception:
            return False
        return False

    visit(Atspi.get_desktop(0))
    actions = matches if all_matches else matches[:1]
    return bool(actions) and all(bool(action.do_action(0)) for action in actions)


class Session:
    """【GNOME回归】【进程资源】只管理当前私有会话中启动的进程与日志"""

    def __init__(self, root):
        """【GNOME回归】【资源构造】按root记录测试目录，无返回值"""
        self.root = root
        self.processes = []
        self.logs = []

    def spawn(self, arguments, name, environment=None, **kwargs):
        """【GNOME回归】【启动进程】按arguments、name和environment创建子进程，返回句柄并保存日志"""
        log = (self.root / f"{name}.log").open("wb")
        self.logs.append(log)
        process = subprocess.Popen(arguments, env=environment, stdout=kwargs.pop("stdout", log), stderr=log, **kwargs)
        self.processes.append(process)
        return process

    def close(self):
        """【GNOME回归】【关闭资源】无参数，仅回收当前会话拥有的进程句柄与日志，无返回值"""
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


def verify(options, session):
    """【GNOME回归】【真实协议】按options与session验证授权、导出与跨协议剪贴板，返回检查记录"""
    import dbus
    from PIL import Image

    root = options.root
    shell = session.spawn([str(options.shell), "--headless", "--wayland", *(["--virtual-monitor=1024x768"] * options.monitors), "--wayland-display=mark-shot-gnome", "--debug-control"], "gnome")
    bus = dbus.SessionBus()
    wait(lambda: bus.name_has_owner("org.gnome.Shell") or shell.poll() is not None, "GNOME did not start")
    if shell.poll() is not None:
        raise AssertionError("GNOME exited during initialization; see gnome.log")
    time.sleep(2)
    remote = bus.get_object("org.gnome.Mutter.RemoteDesktop", "/org/gnome/Mutter/RemoteDesktop")
    path = remote.CreateSession(dbus_interface="org.gnome.Mutter.RemoteDesktop")
    remote_session = bus.get_object("org.gnome.Mutter.RemoteDesktop", path)
    remote_session.Start(dbus_interface="org.gnome.Mutter.RemoteDesktop.Session")
    shell_api = bus.get_object("org.gnome.Shell", "/org/gnome/Shell")
    shell_api.Set("org.gnome.Shell", "OverviewActive", dbus.Boolean(False), dbus_interface="org.freedesktop.DBus.Properties")

    def key(symbol, pressed):
        """【GNOME回归】【按键投递】按symbol和pressed向私有远程输入会话发送按键，无返回值"""
        remote_session.NotifyKeyboardKeysym(dbus.UInt32(symbol), pressed, dbus_interface="org.gnome.Mutter.RemoteDesktop.Session")
        time.sleep(0.08)

    def tap(symbol):
        """【GNOME回归】【完整按键】按symbol投递按下及释放，无返回值"""
        key(symbol, True)
        key(symbol, False)

    def evaluate(code):
        """【GNOME回归】【私有观察】按code读取私有Shell状态，返回JSON结果，拒绝执行失败"""
        success, result = shell_api.Eval(code, dbus_interface="org.gnome.Shell")
        if not success:
            raise AssertionError(f"private GNOME evaluation failed: {result}")
        return json.loads(result) if result else None

    time.sleep(1)
    tap(0xFF1B)
    key(0xFFE9, True)
    tap(0xFFBF)
    key(0xFFE9, False)
    time.sleep(0.5)
    for character in "lg":
        tap(ord(character))
    tap(0xFF0D)
    time.sleep(2)
    for character in "global.context.unsafe_mode = true":
        tap(ord(character))
    tap(0xFF0D)
    time.sleep(0.5)
    tap(0xFF1B)
    environment = dict(os.environ, WAYLAND_DISPLAY="mark-shot-gnome", MARK_SHOT_BACKEND="wayland")
    for name in ("DISPLAY", "XAUTHORITY"):
        value = evaluate(f'GLib.getenv("{name}")')
        if value:
            environment[name] = value
    session.spawn(["pipewire"], "pipewire", environment)
    session.spawn(["wireplumber"], "wireplumber", environment)
    accessibility = bus.get_object("org.a11y.Bus", "/org/a11y/bus").GetAddress(dbus_interface="org.a11y.Bus")
    session.spawn(["/usr/lib/at-spi2-registryd"], "atspi", dict(environment, AT_SPI_BUS_ADDRESS=str(accessibility)))
    binary = [str(options.binary), "--config", str(root / "config.toml"), "--nonotify"]
    displays = json.loads(subprocess.check_output([*binary, "--list-displays"], env=environment, timeout=10))
    assert len(displays) == options.monitors and all(display["width"] == 1024 and display["height"] == 768 for display in displays)
    bounds = (max(display["x"] + display["width"] for display in displays) - min(display["x"] for display in displays), max(display["y"] + display["height"] for display in displays) - min(display["y"] for display in displays))

    colors = [(48, 96, 160), (144, 64, 96)]
    for index, display in enumerate(displays):
        color = ",".join(map(str, colors[index]))
        evaluate(f'import("gi://St").then(({{default:St}}) => {{ const actor = new St.Widget({{style: "background-color: rgb({color});"}}); actor.set_position({display["x"]}, {display["y"]}); actor.set_size({display["width"]}, {display["height"]}); global.window_group.add_child(actor); return true; }})')

    def verify_pixels(path):
        """【GNOME回归】【逐屏像素】按path核对各屏已知色块，拒绝黑帧、屏幕错配及残留授权遮罩，无返回值"""
        image = Image.open(path).convert("RGB")
        assert image.size == bounds
        origin = (min(display["x"] for display in displays), min(display["y"] for display in displays))
        for index, display in enumerate(displays):
            for horizontal, vertical in ((128, 128), (512, 384), (896, 640)):
                point = (display["x"] - origin[0] + horizontal, display["y"] - origin[1] + vertical)
                assert image.getpixel(point) == colors[index], f"unexpected monitor pixels at {point}: {image.getpixel(point)}"

    def windows():
        """【GNOME回归】【窗口列表】无参数，返回当前私有Shell实际管理窗口的标题及进程编号"""
        return evaluate("global.get_window_actors().map(actor => ({title:actor.meta_window.get_title(),pid:actor.meta_window.get_pid()}))")

    settings = session.spawn([*binary, "--settings"], "settings", environment)
    wait(lambda: any(window["pid"] == settings.pid for window in windows()), "settings window did not map")
    settings.terminate()
    settings.wait(timeout=3)
    payload = "GNOME clipboard 中文".encode()
    clipboard = session.spawn([*binary, "--clipboard-daemon", "text"], "clipboard", environment, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    clipboard.stdin.write(payload)
    clipboard.stdin.close()
    assert select.select([clipboard.stdout], [], [], 5)[0] and clipboard.stdout.readline() == b"\n"
    expression = "import('gi://St').then(({default:St}) => new Promise(resolve => St.Clipboard.get_default().get_text(St.ClipboardType.CLIPBOARD, (_,text) => resolve(text))))"
    wait(lambda: evaluate(expression) == payload.decode(), "Xwayland text did not reach the Wayland clipboard")
    image = root / "capture.png"
    capture = session.spawn([*binary, "--capture-to", str(image), "--all-outputs", "--debug"], "capture", environment)
    wait(lambda: any(window["title"] == "Share Screen" for window in windows()), "GNOME permission window did not map")
    wait(lambda: portal_action("MetaVendor", all_matches=True), "GNOME monitor option did not activate")
    time.sleep(0.5)
    if not portal_action("Share"):
        for symbol in (0xFF09, 0x20, 0xFF09, 0xFF09, 0xFF0D):
            tap(symbol)
    capture.wait(timeout=12)
    assert capture.returncode == 0 and Image.open(image).size == bounds
    verify_pixels(image)
    assert not any(window["title"] == "Share Screen" for window in windows())
    editor = session.spawn([*binary, "--fullscreen", "--all-outputs", "--to", "c", "--debug"], "editor", environment)
    wait(lambda: any(window["pid"] == editor.pid for window in windows()), "GNOME capture editor did not map")
    key(0xFFE3, True)
    tap(ord("c"))
    key(0xFFE3, False)
    editor.wait(timeout=10)
    assert editor.returncode == 0
    copied = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment, timeout=5)
    (root / "editor-copy.png").write_bytes(copied)
    verify_pixels(root / "editor-copy.png")
    pin = session.spawn([*binary, "--pin-clipboard"], "pin", environment)
    wait(lambda: any(window["pid"] == pin.pid for window in windows()), "GNOME clipboard pin did not map")
    wait(lambda: evaluate(f"global.display.focus_window?.get_pid() === {pin.pid}"), "GNOME clipboard pin did not receive focus")
    key(0xFFE3, True)
    tap(ord("c"))
    key(0xFFE3, False)
    pinned = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment, timeout=5)
    (root / "pin-copy.png").write_bytes(pinned)
    verify_pixels(root / "pin-copy.png")
    tap(0xFF1B)
    pin.wait(timeout=5)
    assert pin.returncode == 0
    return {"checks": ["GNOME output discovery", "settings window mapping", "Xwayland to Wayland UTF-8 clipboard", "GNOME permission selection", "PipeWire PNG capture", "restored permission", "fullscreen editor", "copy shortcut", "Wayland PNG clipboard", "per-monitor source pixels", "permission transition pixels", "clipboard pin", "pinned original PNG"], "displays": displays}


def main():
    """【GNOME回归】【命令入口】无参数，建立私有D-Bus环境或验证标记后执行内部工作流，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/mark-shot"))
    parser.add_argument("--shell", type=Path, default=Path("/usr/bin/gnome-shell"))
    parser.add_argument("--monitors", type=int, choices=(1, 2), default=1)
    parser.add_argument("--inside", action="store_true", help=argparse.SUPPRESS)
    options = parser.parse_args()
    options.binary = options.binary.resolve()
    options.shell = options.shell.resolve()
    options.root = (options.root or Path(tempfile.mkdtemp(prefix="mark-shot-gnome-"))).resolve()
    root = options.root
    if not options.inside:
        root.mkdir(parents=True, exist_ok=True, mode=0o700)
        root.chmod(0o700)
        (root / "result.json").unlink(missing_ok=True)
        state = Path(tempfile.mkdtemp(prefix="session-", dir=root))
        environment = dict(os.environ)
        for name in ("DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET", "NIRI_SOCKET", "SWAYSOCK", "DBUS_SESSION_BUS_ADDRESS", "XAUTHORITY"):
            environment.pop(name, None)
        environment.update(MARK_SHOT_TEST_GNOME=str(root), XDG_RUNTIME_DIR=str(root), XDG_CONFIG_HOME=str(state / "config"), XDG_DATA_HOME=str(state / "data"), XDG_STATE_HOME=str(state / "state"), XDG_CACHE_HOME=str(state / "cache"), XDG_CURRENT_DESKTOP="GNOME", XDG_SESSION_TYPE="wayland", GSETTINGS_BACKEND="memory", NO_AT_BRIDGE="1")
        (root / "config.toml").write_text('[startup]\ninclude_cursor=false\n[notifications]\nenabled=false\n[history]\nenabled=false\n')
        command = ["dbus-run-session", "--", sys.executable, str(Path(__file__).resolve()), "--inside", "--root", str(root), "--binary", str(options.binary), "--shell", str(options.shell), "--monitors", str(options.monitors)]
        raise SystemExit(subprocess.run(command, env=environment, timeout=90).returncode)
    if os.environ.get("MARK_SHOT_TEST_GNOME") != str(root) or os.environ.get("XDG_RUNTIME_DIR") != str(root) or os.environ.get("WAYLAND_DISPLAY") or os.environ.get("DISPLAY"):
        raise RuntimeError("GNOME tests require a fresh private session")
    session = Session(root)
    try:
        result = verify(options, session)
        (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result))
    finally:
        session.close()


if __name__ == "__main__":
    main()
