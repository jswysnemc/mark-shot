#!/usr/bin/env python3
"""【X11贴图回归】【真实窗口】验证翻译交接、文字选择、原图复制及菜单外部关闭"""

import importlib.util
import io
import json
from pathlib import Path
import struct
import subprocess
import time

from PIL import Image, ImageChops, ImageGrab


def wait(predicate, message):
    """【X11贴图回归】【有界等待】按predicate等待界面变化，超时报告message，返回有效结果"""
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        result = predicate()
        if result:
            return result
        time.sleep(0.05)
    raise AssertionError(message)


def verify(runtime):
    """【X11贴图回归】【交接验证】按runtime检查原文、译文、图像与菜单，返回通过项目"""
    binary = str(runtime.binary)
    image = Image.new("RGB", (420, 240), "white")
    encoded = io.BytesIO()
    image.save(encoded, format="PNG")
    source = [35, 35, 150, 30]
    seed = {
        "source_tokens": [{"text": "原文测试", "box": source, "line": 0, "index": 0, "confidence": 1.0}],
        "blocks": [{"source": source, "plate": source, "background": "#ffffffff", "foreground": "#000000ff", "font_size": 18.0, "text": "Translated text", "lines": [{"text": "Translated text", "rect": source, "baseline": 57.0}]}],
        "show_translation": True,
    }
    metadata = json.dumps(seed).encode()
    payload = b"MS-PIN-TRANSLATION\0" + struct.pack("<I", len(metadata)) + metadata + encoded.getvalue()
    pin = runtime.launch([binary, "--pin", "--translation-seed", "--position", "100,120"], "translated-pin", stdin=subprocess.PIPE)
    pin.stdin.write(payload)
    pin.stdin.close()
    window = runtime.window(pin)
    runtime.command("xdotool", "windowfocus", window)

    def geometry():
        """【X11贴图回归】【窗口坐标】无参数，返回当前私有贴图客户区的根坐标及尺寸"""
        return {key: int(value) for key, value in (line.split("=", 1) for line in runtime.command("xdotool", "getwindowgeometry", "--shell", window).decode().splitlines())}

    def windows():
        """【X11贴图回归】【可见表面】无参数，返回本贴图进程所有可见窗口编号"""
        return runtime.command("xdotool", "search", "--onlyvisible", "--pid", str(pin.pid)).decode().splitlines()

    def menu(index):
        """【X11贴图回归】【菜单动作】按可见菜单index完成真实按下释放，不直接调用业务接口"""
        box = geometry()
        runtime.command("xdotool", "mousemove", str(box["X"] + 80), str(box["Y"] + 80), "click", "3")
        wait(lambda: len(windows()) == 2, "pin popup did not map")
        runtime.command("xdotool", "key", "Home", *(["Down"] * index), "Return")
        wait(lambda: len(windows()) == 1, "pin popup did not close")

    time.sleep(0.15)
    box = geometry()
    actual = ImageGrab.grab(xdisplay=runtime.environment["DISPLAY"]).convert("RGB").crop((box["X"], box["Y"], box["X"] + box["WIDTH"], box["Y"] + box["HEIGHT"]))
    actual.save(runtime.root / "translated-pin.png")
    assert ImageChops.difference(image, actual).getbbox(), "translation seed was not painted"
    runtime.command("xdotool", "mousemove", str(box["X"] + 260), str(box["Y"] + 160), "mousedown", "1")
    time.sleep(0.1)
    runtime.command("xdotool", "mousemove", str(box["X"] + 320), str(box["Y"] + 200))
    time.sleep(0.1)
    runtime.command("xdotool", "mouseup", "1")
    wait(lambda: (geometry()["X"], geometry()["Y"]) != (box["X"], box["Y"]), "pin drag did not move the client window")
    runtime.command("xdotool", "key", "ctrl+c")
    copied = Image.open(io.BytesIO(runtime.command("xclip", "-selection", "clipboard", "-target", "image/png", "-out"))).convert("RGB")
    assert copied.size == image.size and not ImageChops.difference(copied, image).getbbox(), "pin copy baked display translation into original image"
    # 1. 【X11贴图回归】【文字选择】按菜单可用行定位文字选择，置顶仅在管理器提供能力时出现
    top = b"_NET_WM_STATE_ABOVE" in runtime.command("xprop", "-root", "_NET_SUPPORTED")
    menu(8 + int(top))
    runtime.command("xdotool", "key", "ctrl+a", "ctrl+c")
    translated = runtime.command("xclip", "-selection", "clipboard", "-target", "UTF8_STRING", "-out").decode()
    assert translated == "Translated text", translated
    # 2. 【X11贴图回归】【切换原文】选择文字后新增复制所选项，关闭译文再全选复制原文
    menu(12 + int(top))
    runtime.command("xdotool", "key", "ctrl+a", "ctrl+c")
    original = runtime.command("xclip", "-selection", "clipboard", "-target", "UTF8_STRING", "-out").decode()
    assert original == "原文测试", original
    box = geometry()
    runtime.command("xdotool", "mousemove", str(box["X"] + 80), str(box["Y"] + 80), "click", "3")
    wait(lambda: len(windows()) == 2, "pin popup did not reopen")
    runtime.command("xdotool", "mousemove", "10", "10", "click", "1")
    wait(lambda: len(windows()) == 1, "outside click did not dismiss pin popup")
    runtime.command("xdotool", "windowfocus", window, "key", "Escape")
    pin.wait(timeout=5)
    assert pin.returncode == 0
    return ["translation seed rendering", "window drag and popup coordinates", "original PNG preservation", "translated text selection", "original text selection", "popup outside click"]


if __name__ == "__main__":
    spec = importlib.util.spec_from_file_location("x11_workflow", Path(__file__).with_name("x11-desktop-workflow.py"))
    workflow = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(workflow)
    import argparse
    import tempfile
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path(tempfile.mkdtemp(prefix="mark-shot-pin-")))
    parser.add_argument("--binary", type=Path, default=Path("target/debug/mark-shot"))
    parser.add_argument("--wm", type=Path)
    options = parser.parse_args()
    options.root.mkdir(parents=True, exist_ok=True, mode=0o700)
    options.root.chmod(0o700)
    (options.root / "result.json").unlink(missing_ok=True)
    runtime = workflow.Runtime(options.root.resolve(), options.binary.resolve())
    try:
        runtime.start()
        manager = workflow.start_window_manager(runtime, options.wm) if options.wm else None
        result = {"checks": verify(runtime)}
        assert manager is None or manager.poll() is None, "window manager exited during verification"
        (runtime.root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result))
    finally:
        runtime.close()
