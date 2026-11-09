#!/usr/bin/env python3
"""【桌面集成回归】【隔离流程】验证真实X11按键、SNI注册、菜单、配置热更新与注销"""

import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time


def load(filename):
    """【桌面集成回归】【模块导入】按filename加载相邻测试模块，返回模块对象"""
    spec = importlib.util.spec_from_file_location(filename, Path(__file__).with_name(filename))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def wait(predicate, message, timeout=12):
    """【桌面集成回归】【有界等待】按predicate等待结果，超过timeout报告message，返回匹配值"""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        value = predicate()
        if value:
            return value
        time.sleep(0.05)
    raise AssertionError(message)


def configure(path, shortcut="F8", keys=True, tray=True):
    """【桌面集成回归】【独立配置】按path及开关写入私有配置，使用原子替换触发生产目录监听，无返回值"""
    pending = path.with_suffix(".new")
    pending.write_text(f'[ui]\nlanguage="chinese"\n[notifications]\nenabled=false\n[history]\nenabled=false\n[desktop]\ntray_enabled={str(tray).lower()}\nshortcuts_enabled={str(keys).lower()}\nscreenshot_shortcut="{shortcut}"\n')
    pending.replace(path)


def verify(runtime, watcher):
    """【桌面集成回归】【真实操作】按runtime与watcher验证应用行为，返回检查记录"""
    import dbus
    import fcntl

    path = runtime.root / "test.toml"
    configure(path)
    service = runtime.launch([str(runtime.binary), "--background"], "desktop")
    states = runtime.root / "cache/mark-shot/desktop"

    def status():
        """【桌面集成回归】【实际状态】无参数，读取服务实际发布的唯一状态文件"""
        files = list(states.glob("*/status.json"))
        return json.loads(files[0].read_text()) if files else {}

    def windows():
        """【桌面集成回归】【可见窗口】无参数，返回私有显示上的应用窗口编号"""
        result = subprocess.run(["xdotool", "search", "--onlyvisible", "--name", "^Mark Shot"], env=runtime.environment, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=3)
        return result.stdout.decode().splitlines()

    def capture(key=None, activate=None):
        """【桌面集成回归】【真实触发】按key或activate触发截图并关闭，验证新窗口实际映射，无返回值"""
        assert not windows()
        if key:
            runtime.command("xdotool", "key", key)
        else:
            activate()
        window = wait(windows, "desktop action did not open the capture editor")[0]
        runtime.command("xdotool", "windowfocus", window, "key", "Escape")
        wait(lambda: not windows(), "capture editor did not close")

    wait(lambda: status().get("shortcuts", {}).get("state") == "registered", "global key was not registered")
    wait(lambda: watcher.items, "tray did not register with StatusNotifierWatcher")
    wait(lambda: status().get("tray", {}).get("state") == "registered", "tray registration status was not published")
    duplicate = runtime.launch([str(runtime.binary), "--background"], "duplicate")
    duplicate.wait(timeout=4)
    assert duplicate.returncode == 0 and service.poll() is None
    capture("F8")
    runtime.command("xdotool", "key", "Caps_Lock", "Num_Lock")
    capture("F8")
    runtime.command("xdotool", "key", "Caps_Lock", "Num_Lock")
    configure(path, "Ctrl+F9")
    wait(lambda: status().get("shortcuts", {}).get("detail") == "Ctrl+F9", "new shortcut was not registered")
    runtime.command("xdotool", "key", "F8")
    time.sleep(0.3)
    assert not windows(), "old shortcut remained active"
    capture("ctrl+F9")
    assert len(watcher.items) == 1, "shortcut changes unnecessarily replaced the tray"
    occupied = load("desktop-test-hotkey.py").OccupiedKey(runtime.environment["DISPLAY"], 0xFFC7)
    try:
        configure(path, "F10")
        wait(lambda: status().get("shortcuts", {}).get("state") == "failed", "occupied global key did not report failure")
    finally:
        occupied.close()
    configure(path, "Ctrl+F9")
    wait(lambda: status().get("shortcuts", {}).get("state") == "registered", "shortcut did not recover after conflict")
    recording = next(states.glob("*/status.json")).with_name("shortcut-recording.lock")
    with recording.open("w+b") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        lock.write(b"1")
        lock.flush()
        wait(lambda: status().get("shortcuts", {}).get("state") == "paused", "recording did not suspend the old global grab")
        runtime.command("xdotool", "key", "ctrl+F9")
        time.sleep(0.3)
        assert not windows(), "global key fired while settings was recording"
    wait(lambda: status().get("shortcuts", {}).get("state") == "registered", "shortcut did not resume when recording closed")
    capture("ctrl+F9")
    address, object_path = watcher.items[-1]
    item = watcher.bus.get_object(address, object_path)
    icon = item.Get("org.kde.StatusNotifierItem", "IconPixmap", dbus_interface="org.freedesktop.DBus.Properties")
    assert int(icon[0][0]) == 32 and len(icon[0][2]) == 32 * 32 * 4
    assert len(set(bytes(icon[0][2]))) > 10, "tray icon is blank"
    capture(activate=lambda: item.Activate(0, 0, dbus_interface="org.kde.StatusNotifierItem"))
    menu_path = item.Get("org.kde.StatusNotifierItem", "Menu", dbus_interface="org.freedesktop.DBus.Properties")
    menu = watcher.bus.get_object(address, menu_path)
    _, layout = menu.GetLayout(0, -1, [], dbus_interface="com.canonical.dbusmenu")
    items = {str(child[1]["label"]): int(child[0]) for child in layout[2] if "label" in child[1]}
    settings = next(number for label, number in items.items() if "设置" in label or "Settings" in label)
    quit_item = next(number for label, number in items.items() if "退出" in label or "Quit" in label)
    menu.Event(settings, "clicked", dbus.Int32(0), dbus.UInt32(0), dbus_interface="com.canonical.dbusmenu")
    window = wait(windows, "tray settings action did not map a window")[0]
    runtime.command("xdotool", "windowfocus", window, "key", "Escape")
    wait(lambda: not windows(), "settings window did not close")
    configure(path, "Ctrl+F9", keys=False)
    wait(lambda: status().get("shortcuts", {}).get("state") == "disabled", "shortcut disable was not applied")
    runtime.command("xdotool", "key", "ctrl+F9")
    time.sleep(0.3)
    assert not windows(), "disabled shortcut still opened a capture"
    address, object_path = watcher.items[-1]
    item = watcher.bus.get_object(address, object_path)
    menu_path = item.Get("org.kde.StatusNotifierItem", "Menu", dbus_interface="org.freedesktop.DBus.Properties")
    menu = watcher.bus.get_object(address, menu_path)
    menu.Event(quit_item, "clicked", dbus.Int32(0), dbus.UInt32(0), dbus_interface="com.canonical.dbusmenu")
    service.wait(timeout=8)
    assert service.returncode == 0
    wait(lambda: not watcher.bus.name_has_owner(address), "tray bus name remained after quit")
    assert status()["shortcuts"]["state"] == "stopped"
    return {"checks": ["single desktop service", "global screenshot key", "Caps Lock and Num Lock", "live shortcut replacement", "old key release", "independent tray registration", "occupied shortcut error and recovery", "recording pause and resume", "StatusNotifierItem registration", "actual tray icon pixels", "tray capture action", "tray settings action", "shortcut disable", "tray quit and unregister"]}


def main():
    """【桌面集成回归】【命令入口】无参数，建立私有总线和显示会话并执行回归，退出回收资源"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/mark-shot"))
    parser.add_argument("--inside", action="store_true")
    options = parser.parse_args()
    root = (options.root or Path(tempfile.mkdtemp(prefix="mark-shot-desktop-"))).resolve()
    root.mkdir(parents=True, exist_ok=True, mode=0o700)
    root.chmod(0o700)
    if not options.inside:
        environment = dict(os.environ, MARK_SHOT_DESKTOP_TEST_ROOT=str(root))
        environment.pop("DBUS_SESSION_BUS_ADDRESS", None)
        raise SystemExit(subprocess.run(["dbus-run-session", "--", sys.executable, str(Path(__file__).resolve()), "--inside", "--root", str(root), "--binary", str(options.binary.resolve())], env=environment, timeout=90).returncode)
    assert os.environ.get("MARK_SHOT_DESKTOP_TEST_ROOT") == str(root)
    (root / "result.json").unlink(missing_ok=True)
    runtime = load("x11-desktop-workflow.py").Runtime(root, options.binary.resolve())
    watcher = None
    try:
        runtime.start()
        runtime.environment["DBUS_SESSION_BUS_ADDRESS"] = os.environ["DBUS_SESSION_BUS_ADDRESS"]
        watcher = load("desktop-test-watcher.py").Watcher()
        result = verify(runtime, watcher)
        (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result))
    finally:
        runtime.close()
        if watcher:
            watcher.close()


if __name__ == "__main__":
    main()
