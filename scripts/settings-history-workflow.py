#!/usr/bin/env python3
"""【设置历史回归】【独立桌面】在私有 Xvfb 验证截图锁释放、设置布局及历史完整原图"""

import argparse
import importlib.util
import json
from pathlib import Path
import time

from PIL import Image, ImageGrab


def wait_for(predicate, description):
    """【设置历史回归】【结果等待】按 predicate 等待实际结果，description 描述超时原因，返回匹配值"""
    deadline = time.monotonic() + 12
    while time.monotonic() < deadline:
        value = predicate()
        if value:
            return value
        time.sleep(0.05)
    raise AssertionError(description)


def verify(runtime):
    """【设置历史回归】【原生操作】按 runtime 验证完整交互并保存图像证据，返回检查列表"""
    runtime.environment["MARK_SHOT_LANG"] = "chinese"
    path = runtime.root / "test.toml"
    path.write_text(f'[general]\nto="s"\n[save]\ndirectory="{runtime.root / "saved"}"\n[notifications]\nenabled=false\n[desktop]\ntray_enabled=false\nshortcuts_enabled=false\n[history]\nenabled=true\n')
    settings = None

    def command(*arguments):
        """【设置历史回归】【输入交付】按 arguments 向私有显示发送操作，返回命令输出"""
        return runtime.command("xdotool", *arguments)

    def snapshot(name):
        """【设置历史回归】【画面证据】按 name 保存实际显示像素，无返回值"""
        time.sleep(0.2)
        ImageGrab.grab(xdisplay=runtime.environment["DISPLAY"]).save(runtime.root / f"{name}.png")

    try:
        # 1. 【设置历史回归】【截图交接】从截图工具打开设置，原截图进程必须结束并释放单实例锁
        capture = runtime.launch([str(runtime.binary), "--capture"], "capture-before-settings")
        window = runtime.window(capture)
        command("windowfocus", window, "key", "ctrl+comma")
        assert capture.wait(timeout=12) == 0
        settings = command("search", "--onlyvisible", "--name", "Settings$").decode().splitlines()[0]

        # 2. 【设置历史回归】【设置常驻截图】设置保持打开时，新截图窗口必须真实映射并成功保存选区
        second = runtime.launch([str(runtime.binary), "--capture"], "capture-with-settings")
        window = runtime.window(second)
        command("windowfocus", window, "mousemove", "--window", window, "100", "100", "mousedown", "1", "mousemove", "--window", window, "410", "310", "mouseup", "1")
        time.sleep(0.15)
        command("key", "Return")
        assert second.wait(timeout=12) == 0
        assert command("getwindowpid", settings)
        history = runtime.root / "data/mark-shot/capture-history"
        result = wait_for(lambda: list(history.glob("capture-*.png")), "completed screenshot was not recorded")[0]
        original = history / result.name.replace("capture-", "original-", 1)
        assert original.is_file()
        with Image.open(result) as cropped, Image.open(original) as complete:
            assert cropped.size == (310, 210), cropped.size
            assert complete.size == (1024, 768), complete.size

        # 3. 【设置历史回归】【输入布局】编辑长路径时整行展示输入框，保存和恢复按钮保持可见
        command("windowfocus", settings, "key", "ctrl+f", "type", "log.path")
        command("mousemove", "--window", settings, "735", "190", "click", "1", "key", "ctrl+a", "type", "/tmp/mark-shot-settings-workflow/diagnostics/session/mark-shot.log")
        snapshot("expanded-path-editor")
        command("key", "Escape", "key", "ctrl+f", "key", "ctrl+a", "key", "BackSpace")
        command("mousemove", "--window", settings, "65", "468", "click", "1")
        snapshot("about-information")

        # 4. 【设置历史回归】【历史入口】真实历史按钮能够把完整原图交接到重选区窗口
        history_process = runtime.launch([str(runtime.binary), "--history"], "history")
        history_window = runtime.window(history_process)
        time.sleep(0.3)
        snapshot("history-original-action")
        command("windowfocus", history_window, "mousemove", "--window", history_window, "162", "49", "click", "1")
        time.sleep(0.4)
        visible = command("search", "--onlyvisible", "--name", "Mark Shot").decode().splitlines()
        other = [window for window in visible if window not in [settings, history_window]]
        assert other, "history reselect did not map a new editor"
        snapshot("history-reselection")
        previous = {path.name for path in history.glob("capture-*.png")}
        editor = other[0]
        command("windowfocus", editor, "mousemove", "--window", editor, "700", "450", "mousedown", "1", "mousemove", "--window", editor, "900", "590", "mouseup", "1")
        time.sleep(0.15)
        command("key", "Return")
        reselected = wait_for(lambda: [path for path in history.glob("capture-*.png") if path.name not in previous], "reselected screenshot was not saved")[0]
        with Image.open(reselected) as cropped, Image.open(history / reselected.name.replace("capture-", "original-", 1)) as complete:
            assert 0 < cropped.width < 1024 and 0 < cropped.height < 768, cropped.size
            assert complete.size == (1024, 768), complete.size
        for window in other[1:]:
            command("windowfocus", window, "key", "Escape")
        command("windowfocus", history_window, "key", "Escape")
        # 5. 【设置历史回归】【无界面保存】区域命令也保存完整原图，完成图保持用户指定尺寸
        previous = {path.name for path in history.glob("capture-*.png")}
        runtime.command(str(runtime.binary), "--capture-to", str(runtime.root / "headless-region.png"), "--region", "100,100,40,50")
        newest = wait_for(lambda: [path for path in history.glob("capture-*.png") if path.name not in previous], "headless history was not recorded")[0]
        with Image.open(newest) as cropped, Image.open(history / newest.name.replace("capture-", "original-", 1)) as complete:
            assert cropped.size == (40, 50), cropped.size
            assert complete.size == (1024, 768), complete.size
        return ["settings handoff releases screenshot process", "capture while settings remains open", "full original and crop saved separately", "expanded path editing", "about page", "history reselect saves a new crop from the original", "headless region keeps full original"]
    finally:
        if settings:
            try:
                command("windowfocus", settings, "key", "Escape", "key", "Escape")
            except Exception:
                pass


def main():
    """【设置历史回归】【隔离入口】无参数，创建私有显示并清理本次资源，输出检查结果"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/mark-shot"))
    options = parser.parse_args()
    root = options.root.resolve()
    root.mkdir(parents=True, exist_ok=True, mode=0o700)
    spec = importlib.util.spec_from_file_location("x11_workflow", Path(__file__).with_name("x11-desktop-workflow.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    runtime = module.Runtime(root, options.binary.resolve())
    try:
        runtime.start()
        report = verify(runtime)
        (root / "checks.json").write_text(json.dumps(report, ensure_ascii=False, indent=2))
        print(json.dumps(report, ensure_ascii=False))
    finally:
        runtime.close()


if __name__ == "__main__":
    main()
