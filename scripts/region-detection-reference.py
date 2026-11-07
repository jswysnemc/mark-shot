#!/usr/bin/env python3
"""【区域识别参考】【场景采集】构造确定性截图并采集原版 Qt 算法输出"""

import argparse
import hashlib
import json
import random
import subprocess
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw

PROJECT = Path(__file__).resolve().parents[1]
QT_ROOT = Path("/home/snemc/workspace/mark-shot/qt")


def candidate(rect, quality, source="contour"):
    """【区域识别参考】【候选构造】按 rect、quality 与 source 构造结构化候选，返回字典"""
    return {"rect": rect, "quality": quality, "sides": [quality] * 4, "interior_edges": 0.1, "sources": [source]}


def action(kind, point, direction=0):
    """【区域识别参考】【动作构造】按 kind、point、direction 返回真实查询或悬停动作"""
    return {"kind": kind, "point": point, "direction": direction}


def fill(image, rect, color):
    """【区域识别参考】【矩形填充】按 image、rect 与 color 填充排除右下边界的整数矩形，无返回值"""
    x, y, width, height = rect
    ImageDraw.Draw(image).rectangle((x, y, x + width - 1, y + height - 1), fill=color)


def hollow(image, rect, color):
    """【区域识别参考】【空心矩形】按 image、rect 和 color 绘制一像素边界，无返回值"""
    x, y, width, height = rect
    fill(image, [x, y, width, 1], color)
    fill(image, [x, y + height - 1, width, 1], color)
    fill(image, [x, y, 1, height], color)
    fill(image, [x + width - 1, y, 1, height], color)


def images():
    """【区域识别参考】【图像场景】返回原版测试覆盖的确定性控件、文字、缩图和透明场景"""
    result = []
    def add(name, image, points, premultiplied=False):
        """【区域识别参考】【场景登记】按参数登记 image 与 points，premultiplied 控制输入格式，无返回值"""
        result.append((name, image, points, premultiplied))

    for size in [(320, 200), (1, 1), (2, 3), (3, 3), (1, 80)]:
        add(f"flat-{size[0]}-{size[1]}", Image.new("RGBA", size, (40, 40, 40, 255)), [[0, 0], [size[0] - 1, size[1] - 1]])
    image = Image.new("RGBA", (360, 240), (20, 20, 20, 255))
    fill(image, [30, 20, 300, 200], (45, 45, 45, 255))
    fill(image, [100, 80, 120, 48], (75, 75, 75, 255))
    add("nested", image, [[144, 96], [50, 50], [10, 10]])
    for index, (background, foreground) in enumerate([((28, 28, 28), (36, 36, 36)), ((200, 200, 200), (208, 208, 208)), ((160, 50, 50), (50, 106, 50))]):
        image = Image.new("RGBA", (360, 240), (*background, 255))
        fill(image, [60, 45, 230, 140], (*foreground, 255))
        add(f"weak-color-{index}", image, [[180, 120], [61, 46], [289, 184]])
    image = Image.new("RGBA", (640, 360), (28, 28, 28, 255))
    fill(image, [35, 30, 560, 290], (36, 36, 36, 255))
    for row in range(9):
        for column in range(28):
            x, y = 55 + column * 18, 48 + row * 28
            for rect in [[x, y, 2, 17], [x, y, 10, 2], [x, y + 8, 8, 2]]:
                fill(image, rect, (220, 220, 220, 255))
    add("bright-text-weak-panel", image, [[512, 252], [80, 100]])
    image = Image.new("RGBA", (360, 240), (22, 22, 22, 255))
    fill(image, [0, 30, 150, 210], (45, 45, 45, 255))
    fill(image, [162, 30, 198, 210], (50, 50, 50, 255))
    add("adjacent-clipped-panels", image, [[72, 120], [252, 120]])
    image = Image.new("RGBA", (400, 250), (30, 30, 30, 255))
    hollow(image, [25, 30, 160, 150], (210, 210, 210, 255))
    for y in range(65, 145):
        for x in range(225, 355):
            dx, dy = max(0, 237 - x, x - 342), max(0, 77 - y, y - 132)
            if dx * dx + dy * dy <= 144:
                image.putpixel((x, y), (50, 50, 50, 255))
    add("hollow-rounded", image, [[80, 100], [280, 100]])
    image = Image.new("RGBA", (3840, 2160), (24, 24, 24, 255))
    fill(image, [421, 213, 1500, 1100], (42, 42, 42, 255))
    fill(image, [631, 451, 30, 30], (100, 100, 100, 255))
    add("small-control-4k", image, [[645, 465], [1200, 900]])
    image = Image.new("RGBA", (640, 360), (25, 25, 25, 255))
    fill(image, [30, 30, 580, 300], (42, 42, 42, 255))
    fill(image, [180, 140, 250, 60], (65, 65, 65, 255))
    glyphs = [[14, 17, 17, 17, 17, 17, 14], [30, 17, 17, 30, 16, 16, 16], [31, 16, 16, 30, 16, 16, 31], [17, 25, 21, 19, 17, 17, 17]]
    for index, glyph in enumerate(glyphs):
        for y, row in enumerate(glyph):
            for x in range(5):
                if row & (1 << (4 - x)):
                    fill(image, [202 + index * 30 + x * 3, 157 + y * 3, 3, 3], (225, 225, 225, 255))
    add("button-text", image, [[x, 168] for x in (205, 215, 240, 285, 310)])
    image = Image.new("RGBA", (360, 240), (20, 20, 20, 255))
    fill(image, [30, 20, 300, 200], (45, 45, 45, 255))
    fill(image, [100, 80, 24, 24], (85, 85, 85, 255))
    fill(image, [110, 84, 4, 16], (220, 220, 220, 255))
    fill(image, [104, 90, 16, 4], (220, 220, 220, 255))
    add("compact-icon", image, [[111, 91]])
    image = Image.new("RGBA", (480, 320))
    image.putdata([(20 + x // 8, 30 + y // 6, 80, 255) for y in range(320) for x in range(480)])
    add("smooth-gradient", image, [[240, 160]])
    image = Image.new("RGBA", (640, 360), (20, 20, 20, 255))
    fill(image, [30, 30, 580, 300], (45, 45, 45, 255))
    for row in range(8):
        for column in range(18):
            hollow(image, [60 + column * 28, 55 + row * 30, 11, 18], (200, 200, 200, 255))
    add("dense-text", image, [[65 + column * 28, 65] for column in (0, 4, 8, 12, 16)])
    image = Image.new("RGBA", (600, 400), (20, 20, 20, 255))
    fill(image, [70, 40, 450, 320], (45, 45, 45, 255))
    fill(image, [70, 100, 450, 2], (80, 80, 80, 255))
    fill(image, [70, 240, 450, 2], (80, 80, 80, 255))
    fill(image, [70, 180, 2, 3], (20, 20, 20, 255))
    add("broken-outline-separators", image, [[300, 160]])
    image = Image.new("RGBA", (360, 240), (20, 20, 20, 255))
    fill(image, [70, 60, 180, 100], (50, 50, 50, 255))
    add("enlarged-input", image.resize((1440, 960), Image.Resampling.NEAREST), [[576, 384]])
    source = Image.open(QT_ROOT / "tests/fixtures/region-toolbar-context.png").convert("RGBA")
    add("real-toolbar", source, [[1310, 181], [195, 115]])
    for size in [(1281, 49), (49, 1281), (3, 4097), (4097, 3), (1600, 205), (1400, 900)]:
        image = Image.new("RGBA", size)
        image.putdata([((x * 17 + y * 3) % 256, (y * 13 + x * 5) % 256, (x * 7 + y * 11) % 256, 255) for y in range(size[1]) for x in range(size[0])])
        add(f"sampling-{size[0]}-{size[1]}", image, [[size[0] // 2, size[1] // 2]])
    image = Image.new("RGBA", (1510, 110), (28, 56, 80, 128))
    fill(image, [300, 20, 900, 70], (100, 70, 35, 64))
    add("transparent-normal", image, [[700, 50]])
    add("transparent-premultiplied", image, [[700, 50]], True)
    return result


def manual_cases():
    """【区域识别参考】【层级场景】返回质量排序、窗口约束、归一化、合并和层级容差的确定性场景"""
    rng = random.Random(140)
    result = []
    for index in range(80):
        size = [rng.randrange(300, 2000), rng.randrange(220, 1200)]
        parent = [20, 20, size[0] - 40, size[1] - 40]
        child = [80, 60, min(120, size[0] - 120), min(70, size[1] - 90)]
        point = [110, 80]
        candidates = [candidate(child, rng.choice([0.60, 0.72, 0.80, 0.85, 0.95])), candidate(parent, 0.95, rng.choice(["background", "window"])), candidate([90, 70, 12, 12], 0.60)]
        actions = [action("region", point), action("regions", point), action("hover", point), action("step", point, 1), action("hover", [111, 81]), action("step", point, -1), action("step", point, 1), action("step", point, 1), action("step", point, 0), action("hover", [116, 80]), action("hover", [117, 80]), action("reset", point)]
        actions.extend({"kind": "region", "normalized": xy} for xy in [[-1.0, -1.0], [0.0, 0.0], [1.0, 1.0], [2.0, 2.0]])
        result.append({"name": f"manual-{index}", "mode": "manual", "width": size[0], "height": size[1], "candidates": candidates, "window_rects": [], "actions": actions})
    result.append({"name": "window-containment", "mode": "manual", "width": 3840, "height": 2160, "candidates": [candidate([20, 0, 3820, 2160], 0.9), candidate([60, 140, 360, 60], 0.85)], "window_rects": [[22, 84, 3794, 2050]], "actions": [action("region", [1900, 1000]), action("region", [200, 170]), action("region", [1900, 30])]})
    result.append({"name": "window-clipping-minimum", "mode": "manual", "width": 320, "height": 240, "candidates": [], "window_rects": [[-20, -20, 100, 100], [310, 20, 40, 60], [100, 40, 7, 50], [100, 40, 8, 8]], "actions": [action("region", [0, 0]), action("region", [315, 30]), action("region", [105, 45]), action("regions", [105, 45])]})
    result.append({"name": "merge-source-quality", "mode": "manual", "width": 320, "height": 240, "candidates": [candidate([40, 30, 100, 80], 0.72)], "merges": [{"candidate": candidate([41, 31, 100, 80], 0.95, "background"), "tolerance": 2}], "window_rects": [[40, 30, 100, 80]], "actions": [action("region", [70, 60]), action("regions", [70, 60])]})
    for z in [None, -5, -2147483648, 0, 20]:
        result.append({"name": f"window-z-{z}", "mode": "windows", "windows": [{"rect": [0, 0, 80, 80], "z": None}, {"rect": [10, 10, 40, 40], "z": z}], "actions": [action("region", point) for point in [[20, 20], [0, 0], [50, 20], [80, 0], [90, 90]]]})
    return result


def run(binary, root):
    """【区域识别参考】【采集入口】按 binary 与新 root 创建输入并调用实际 Qt 算法，返回文件和来源摘要"""
    root.mkdir(parents=True, exist_ok=False)
    directory = root / "images"
    directory.mkdir()
    cases = manual_cases()
    for name, image, points, premultiplied in images():
        path = directory / f"{name}.png"
        image.save(path)
        actions = []
        for point in points:
            actions.extend([action("region", point), action("regions", point), action("hover", point), action("step", point, 1), action("hover", [point[0] + 1, point[1] + 1]), action("step", point, -1), action("reset", point)])
        cases.append({"name": name, "mode": "image", "image": str(path.relative_to(PROJECT)), "premultiplied": premultiplied, "window_rects": [], "actions": actions})
    with tempfile.TemporaryDirectory(prefix="region-reference-") as temporary:
        input_path, output_path = Path(temporary) / "input.json", Path(temporary) / "expected.json"
        input_path.write_text(json.dumps({"cases": cases}, separators=(",", ":")) + "\n")
        subprocess.run([str(binary), str(input_path), str(output_path)], cwd=PROJECT, check=True)
        expected = json.loads(output_path.read_text())
    groups = {}
    for mode in ("manual", "image", "windows"):
        inputs = [case for case in cases if case["mode"] == mode]
        names = {case["name"] for case in inputs}
        outputs = [case for case in expected["cases"] if case["name"] in names]
        for suffix, values in [("input", inputs), ("expected", outputs)]:
            path = root / f"{mode}-{suffix}.json"
            header = '{"qt_version":' + json.dumps(expected["qt_version"]) + ',"cases":[\n'
            path.write_text(header + ",\n".join(json.dumps(case, separators=(",", ":")) for case in values) + "\n]}\n")
            groups[path.name] = hashlib.sha256(path.read_bytes()).hexdigest()
    sources = sorted((QT_ROOT / "src/region_detection").glob("*")) + [QT_ROOT / "src/window_hover_selection.cpp", QT_ROOT / "src/window_hover_selection.h"]
    report = {"cases": len(cases), "qt_version": expected["qt_version"], "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "source_sha256": {str(path.relative_to(QT_ROOT)): hashlib.sha256(path.read_bytes()).hexdigest() for path in sources if path.is_file()}, "group_sha256": groups}
    (root / "reference.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    """【区域识别参考】【命令入口】解析固定 binary 与新 root，输出实际采集摘要，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
