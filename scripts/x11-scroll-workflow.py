#!/usr/bin/env python3
"""【X11滚动回归】【真实拼接】验证底层页面实际滚动及完整剪贴板原图像素"""

import argparse
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import time

from PIL import Image, ImageChops, ImageGrab


def load(name, filename):
    """【X11滚动回归】【辅助导入】按name与filename载入已有私有会话工具，返回模块"""
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


WORKFLOW = load("x11_workflow", "x11-desktop-workflow.py")
PIN = load("x11_pin", "x11-pin-workflow.py")


def controls(image):
    """【X11滚动回归】【面板定位】按image查找实际保存按钮，返回暂停和复制按钮中心"""
    remaining = {(column, row) for row in range(image.height) for column in range(image.width) if image.getpixel((column, row)) in ((16, 185, 129), (52, 211, 153))}
    groups = []
    while remaining:
        pending = [remaining.pop()]
        pixels = []
        while pending:
            point = pending.pop()
            pixels.append(point)
            for adjacent in ((point[0] - 1, point[1]), (point[0] + 1, point[1]), (point[0], point[1] - 1), (point[0], point[1] + 1)):
                if adjacent in remaining:
                    remaining.remove(adjacent)
                    pending.append(adjacent)
        groups.append(pixels)
    pixels = max(groups, key=len, default=[])
    if len(pixels) < 200:
        return None
    horizontal = (min(point[0] for point in pixels) + max(point[0] for point in pixels)) // 2
    vertical = (min(point[1] for point in pixels) + max(point[1] for point in pixels)) // 2
    return {"pause": (horizontal - 94, vertical), "copy": (horizontal + 47, vertical)}


def verify(runtime, page_binary):
    """【X11滚动回归】【完整流程】按runtime与page_binary执行选区、滚动、暂停、复制并比对独立期望像素"""
    (runtime.root / "runtime.json").write_text(json.dumps({"private_x11": runtime.environment["DISPLAY"]}))
    trace = runtime.root / "position.txt"
    page = runtime.launch([str(page_binary), str(trace)], "page")
    page_window = runtime.window(page)
    runtime.command("xdotool", "windowsize", page_window, "1024", "768", "windowmove", page_window, "0", "0", "windowfocus", page_window)
    time.sleep(0.2)
    source = ImageGrab.grab(xdisplay=runtime.environment["DISPLAY"]).convert("RGB").crop((40, 40, 360, 360))
    source.save(runtime.root / "source.png")
    process = runtime.launch([str(runtime.binary), "-S", "--to", "c", "--debug"], "scroll")
    overlay = runtime.window(process)
    runtime.command("xdotool", "windowfocus", overlay)
    time.sleep(0.2)
    runtime.command("xdotool", "mousemove", "40", "40", "mousedown", "1", "mousemove", "360", "360", "mouseup", "1")
    PIN.wait(lambda: "【滚动截图】【会话开始】" in (runtime.root / "scroll.log").read_text(), "scroll capture did not start")
    frame = lambda: ImageGrab.grab(xdisplay=runtime.environment["DISPLAY"]).convert("RGB")
    PIN.wait(lambda: controls(frame()), "scroll controls did not map")
    time.sleep(0.6)
    parts = [source]
    for index in range(4):
        runtime.command("xdotool", "mousemove", "160", "160", "click", "5")
        PIN.wait(lambda: trace.exists() and trace.read_text().strip() == f"0 {(index + 1) * 48}", "original page did not scroll")
        time.sleep(0.3)
        screen = frame()
        screen.save(runtime.root / f"screen-{index}.png")
        image = screen.crop((40, 40, 360, 360))
        assert ImageChops.difference(parts[0], image).getbbox(), "page trace advanced without a pixel update"
        parts.append(image.crop((0, 272, 320, 320)))
    buttons = PIN.wait(lambda: controls(frame()), "scroll controls disappeared")
    runtime.command("xdotool", "mousemove", *map(str, buttons["pause"]), "click", "1")
    time.sleep(0.1)
    runtime.command("xdotool", "mousemove", *map(str, buttons["copy"]), "click", "1")
    process.wait(timeout=8)
    assert process.returncode == 0
    expected = Image.new("RGB", (320, 320 + 4 * 48))
    offset = 0
    for part in parts:
        expected.paste(part, (0, offset))
        offset += part.height
    actual = Image.open(io.BytesIO(runtime.command("xclip", "-selection", "clipboard", "-target", "image/png", "-out"))).convert("RGB")
    actual.save(runtime.root / "actual.png")
    expected.save(runtime.root / "expected.png")
    assert actual.size == expected.size and not ImageChops.difference(actual, expected).getbbox(), f"stitched pixels differ: {actual.size} versus {expected.size}"
    return {"checks": ["original page wheel input", "continuous vertical stitching", "pause and clipboard output", "exact source pixel preservation"], "size": actual.size}


def main():
    """【X11滚动回归】【命令入口】无参数，创建独立显示会话并保存验证结果，退出回收全部私有进程"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path(tempfile.mkdtemp(prefix="mark-shot-scroll-x11-")))
    parser.add_argument("--binary", type=Path, default=Path("target/debug/mark-shot"))
    parser.add_argument("--page", type=Path, default=Path("target/debug/examples/scroll-page"))
    options = parser.parse_args()
    options.root.mkdir(parents=True, exist_ok=True, mode=0o700)
    options.root.chmod(0o700)
    (options.root / "result.json").unlink(missing_ok=True)
    runtime = WORKFLOW.Runtime(options.root.resolve(), options.binary.resolve())
    try:
        runtime.start()
        result = verify(runtime, options.page.resolve())
        (runtime.root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result))
    finally:
        runtime.close()


if __name__ == "__main__":
    main()
