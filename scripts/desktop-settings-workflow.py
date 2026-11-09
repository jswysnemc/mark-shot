#!/usr/bin/env python3
"""【桌面设置回归】【原生控件】在独立X11会话操作共用颜色组件并保存可视证据"""

import argparse
import importlib.util
from pathlib import Path
import tempfile
import time
import tomllib
from PIL import ImageGrab


def verify(runtime):
    """【桌面设置回归】【颜色事务】按runtime执行真实点击、输入、确认、取消和恢复，返回验证记录"""
    runtime.environment["MARK_SHOT_LANG"] = "chinese"
    path = runtime.root / "test.toml"
    path.write_text('[pinned]\nborder_color="#ee334488"\n')
    process = runtime.launch([str(runtime.binary), "--settings"], "settings")
    window = runtime.window(process)

    def command(*arguments):
        """【桌面设置回归】【输入交付】按arguments向私有显示发送输入，返回命令输出"""
        return runtime.command("xdotool", *arguments)

    def screenshot(name):
        """【桌面设置回归】【可视证据】按name保存实际帧缓冲，等待当前重绘完成，无返回值"""
        time.sleep(0.2)
        ImageGrab.grab(xdisplay=runtime.environment["DISPLAY"]).save(runtime.root / f"{name}.png")

    def open_picker():
        """【桌面设置回归】【打开组件】无参数，点击真实色值字段，返回已获得键盘焦点的颜色窗口"""
        command("mousemove", "--window", window, "700", "190", "click", "1")
        time.sleep(0.2)
        dialog = command("search", "--onlyvisible", "--name", "Color$").decode().splitlines()[0]
        command("windowfocus", dialog)
        return dialog

    command("windowfocus", window, "key", "ctrl+f", "type", "border_color")
    screenshot("color-setting")
    dialog = open_picker()
    screenshot("color-picker")
    command("mousemove", "--window", dialog, "100", "207", "click", "1", "key", "ctrl+a", "type", "#00bb7799")
    command("key", "Return")
    screenshot("color-applied")
    assert tomllib.loads(path.read_text())["pinned"]["border_color"].lower() == "#00bb7799"
    dialog = open_picker()
    command("mousemove", "--window", dialog, "100", "207", "click", "1", "key", "ctrl+a", "type", "#112233ff")
    command("key", "Escape")
    time.sleep(0.2)
    assert tomllib.loads(path.read_text())["pinned"]["border_color"].lower() == "#00bb7799"
    dialog = open_picker()
    command("mousemove", "--window", dialog, "30", "300", "click", "1")
    time.sleep(0.2)
    assert "border_color" not in tomllib.loads(path.read_text()).get("pinned", {})
    command("windowfocus", window, "key", "ctrl+f", "type", "desktop.")
    screenshot("desktop-settings")
    process.terminate()
    process.wait(timeout=3)
    return ["actual color preview", "shared color dialog", "RGBA entry and save", "cancel preserves color", "restore removes explicit override", "desktop settings controls"]


def main():
    """【桌面设置回归】【隔离入口】无参数，建立私有显示并回收本次进程，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/mark-shot"))
    options = parser.parse_args()
    root = (options.root or Path(tempfile.mkdtemp(prefix="mark-shot-settings-"))).resolve()
    root.mkdir(parents=True, exist_ok=True, mode=0o700)
    spec = importlib.util.spec_from_file_location("x11_workflow", Path(__file__).with_name("x11-desktop-workflow.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    runtime = module.Runtime(root, options.binary.resolve())
    try:
        runtime.start()
        print(verify(runtime))
    finally:
        runtime.close()


if __name__ == "__main__":
    main()
