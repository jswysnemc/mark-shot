#!/usr/bin/env python3
"""【启动工具原生回归】【真实绘制】验证文件工具继承、独立覆盖和屏幕入口隔离"""

import argparse
import hashlib
import io
import json
from pathlib import Path
import subprocess
import time

from PIL import Image
import wayland_single_instance_checks as checks
import wayland_single_instance_session as runtime
import wayland_text_format_session as session
from wayland_text_pointer import CURRENT, pointer_context
from wayland_virtual_pointer import transform_for_pointer


def draw():
    """【启动工具原生回归】【实际拖动】在当前私有画布中部绘制一笔，避开窄输出的换行工具栏，无参数、无返回值"""
    pointer, logical, _, x11 = CURRENT.get()

    def move(point):
        """【启动工具原生回归】【指针投递】按 point 移动当前私有设备，无返回值，坐标来自本次真实输出"""
        if x11:
            coordinates = [round(point[index] * x11[1][["width", "height"][index]] / logical[["width", "height"][index]]) for index in range(2)]
            subprocess.run(["xdotool", "mousemove", "--sync", *map(str, coordinates)], env=x11[0], check=True, timeout=3)
        else:
            pointer.move_abs(*transform_for_pointer(*point, logical["width"], logical["height"], logical.get("pointer_transform", logical["transform"])))
        time.sleep(0.08)

    move((logical["width"] * 0.3, logical["height"] * 0.45))
    if x11:
        subprocess.run(["xdotool", "mousedown", "1"], env=x11[0], check=True, timeout=3)
    else:
        pointer.press()
    try:
        time.sleep(0.06)
        move((logical["width"] * 0.65, logical["height"] * 0.72))
    finally:
        if x11:
            subprocess.run(["xdotool", "mouseup", "1"], env=x11[0], check=True, timeout=3)
        else:
            pointer.release()


def shape(png, expected):
    """【启动工具原生回归】【实际标注】按 png 检查 expected 矩形或画笔的真实红色笔迹，返回边界、中心及横向覆盖比例"""
    image = Image.open(io.BytesIO(png)).convert("RGB")
    mask = Image.new("L", image.size)
    mask.putdata([255 if red > 130 and green < 130 and blue < 130 else 0 for red, green, blue in image.getdata()])
    bounds = mask.getbbox()
    assert bounds, "the initial tool did not create an annotation"
    left, top, right, bottom = bounds
    assert right - left > 10 and bottom - top > 10, bounds
    center = ((left + right) // 2, (top + bottom) // 2)
    block = mask.crop((center[0] - 2, center[1] - 2, center[0] + 3, center[1] + 3))
    coverage = max(sum(value > 0 for value in mask.crop((left, row, right, row + 1)).getdata())
                   for row in range(top, bottom)) / (right - left)
    if expected == "rectangle":
        assert block.getbbox() is None and coverage > 0.6, (bounds, center, coverage)
    else:
        assert expected == "pen" and block.getbbox() is not None and coverage < 0.5, (bounds, center, coverage)
    return {"png_size": list(image.size), "annotation_bounds": list(bounds), "center": list(center),
            "maximum_row_coverage": coverage, "expected_tool": expected}


def run(binary, root):
    """【启动工具原生回归】【完整流程】按 binary 在当前私有会话直接绘制并导出，root 保存所有进程与像素证据，返回构建结果"""
    root.mkdir(parents=True, exist_ok=False, mode=0o700)
    private = session.environment()
    metadata = json.loads((Path(private["XDG_RUNTIME_DIR"]) / "runtime.json").read_text())
    compositor = metadata.get("compositor", "niri")
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    original = root / "source.png"
    Image.new("RGB", (256, 160), (30, 30, 30)).save(original)
    reports = []
    with session.input_context(), pointer_context(backend="virtual" if compositor == "sway" else "x11"):
        if compositor == "sway":
            session.sway.keyboard_ready()
        for name, options, expected in [
            ("file-override", ["--default-tool", "pen", "--fullscreen-default-tool", "ruler", "--file-default-tool", "rectangle", str(original)], "rectangle"),
            ("file-inherits-fullscreen", ["--default-tool", "pen", "--fullscreen-default-tool", "rectangle", str(original)], "rectangle"),
            ("screen-ignores-file", ["--full-screen", "--all-output", "--fullscreen-default-tool", "pen", "--file-default-tool", "rectangle"], "pen"),
        ]:
            with runtime.launch(binary, root, name, ["--default-color", "#FF4D4D", *options]) as capture:
                capture.wait_present()
                before = checks.idle(capture)
                draw()
                after = checks.idle(capture)
                session.key("Return")
                assert capture.process.wait(timeout=5) == 0
                rendered = shape(capture.output.read_bytes(), expected)
                if name.startswith("file"):
                    assert rendered["png_size"] == [256, 160]
                reports.append({"name": name, **rendered, "idle_before": before, "idle_after": after})
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == fingerprint
    report = {"binary_sha256": fingerprint, "compositor": compositor, "cases": reports}
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    """【启动工具原生回归】【命令入口】解析固定 binary 与新 root，执行私有验证后输出三类真实启动结果，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    report = run(options.binary.resolve(), options.root.resolve())
    print(json.dumps({"binary_sha256": report["binary_sha256"], "compositor": report["compositor"],
                      "cases": [case["name"] for case in report["cases"]]}, indent=2))


if __name__ == "__main__":
    main()
