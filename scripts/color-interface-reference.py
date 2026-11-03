#!/usr/bin/env python3
"""【颜色对照】【像素证据】比较实际 Qt 子控件与 Rust 固定缓冲，分别报告几何相同和像素差异"""

import argparse
import json
from pathlib import Path

from PIL import Image, ImageChops, ImageStat


EXPECTED = [
    ("markshot::ui::ColorPicker", [8, 8, 184, 212]),
    ("markshot::ui::SVField", [8, 8, 184, 112]),
    ("markshot::ui::HueSlider", [8, 126, 184, 12]),
    ("markshot::ui::AlphaSlider", [8, 144, 184, 12]),
    ("markshot::ui::ColorSwatch", [8, 162, 26, 26]),
    ("QLineEdit", [40, 162, 152, 26]),
]
HISTORY_X = (8, 32, 56, 79, 103, 127, 150, 174)
RADIAL = ((74, 6), (114, 19), (139, 53), (139, 95), (114, 129),
          (74, 142), (34, 129), (9, 95), (9, 53), (34, 19))


def geometry(root):
    """【颜色对照】【几何核对】读取 root 的实际 Qt JSON，逐项核对 Rust 共享几何；不相同时抛出异常，无返回值"""
    detailed = json.loads((root / "dark.json").read_text())
    if detailed["panel"] != [0, 0, 200, 228]:
        raise AssertionError(f"unexpected Qt picker size: {detailed['panel']}")
    expected = EXPECTED + [("markshot::ui::ColorSwatch", [x, 194, 26, 26]) for x in HISTORY_X]
    actual = [(item["class"], item["geometry"]) for item in detailed["children"]]
    if actual != expected:
        raise AssertionError(f"the Rust layout does not match the actual Qt tree: {actual}")
    radial = json.loads((root / "radial.json").read_text())
    if radial["panel"] != [0, 0, 178, 178]:
        raise AssertionError(f"unexpected Qt radial size: {radial['panel']}")
    buttons = [item["geometry"] for item in radial["children"] if item["class"] == "QPushButton"]
    if buttons != [[x, y, 28, 28] for x, y in RADIAL]:
        raise AssertionError(f"the Rust radial positions do not match Qt: {buttons}")


def pixels(qt, rust, output, name):
    """【颜色对照】【像素核对】将 qt、rust 的 name 面板并排写入 output，返回原始 RGBA 与相同背景合成后的误差指标"""
    with Image.open(qt / f"{name}.png") as image:
        first = image.convert("RGBA")
    with Image.open(rust / f"{name}.png") as image:
        second = image.convert("RGBA")
    if first.size != second.size:
        raise AssertionError(f"{name} buffer sizes differ: {first.size} != {second.size}")
    raw = ImageChops.difference(first, second)
    background = Image.new("RGBA", first.size, (30, 34, 38, 255))
    first_rgb = Image.alpha_composite(background, first).convert("RGB")
    second_rgb = Image.alpha_composite(background, second).convert("RGB")
    difference = ImageChops.difference(first_rgb, second_rgb)
    montage = Image.new("RGB", (first.width * 3, first.height))
    montage.paste(first_rgb)
    montage.paste(second_rgb, (first.width, 0))
    montage.paste(difference, (first.width * 2, 0))
    montage.resize((montage.width * 2, montage.height * 2)).save(output / f"{name}-comparison.png")
    changed = sum(any(pixel) for pixel in raw.get_flattened_data())
    return {"size": list(first.size), "geometry_equal": True, "raw_rgba_exact": changed == 0,
            "raw_rgba_changed_pixels": changed, "raw_rgba_mae": sum(ImageStat.Stat(raw).mean) / 4,
            "composited_rgb_mae": sum(ImageStat.Stat(difference).mean) / 3,
            "composited_rgb_max_channel_delta": max(value[1] for value in difference.getextrema())}


def main():
    """【颜色对照】【命令入口】读取 Qt、Rust 证据目录并输出几何与像素指标，任何像素差异如实保留，不宣称完全一致"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--qt", type=Path, required=True)
    parser.add_argument("--rust", type=Path, required=True)
    parser.add_argument("--artifacts", type=Path, required=True)
    options = parser.parse_args()
    options.artifacts.mkdir(parents=True, exist_ok=True)
    geometry(options.qt)
    result = {name: pixels(options.qt, options.rust, options.artifacts, name) for name in ("dark", "radial")}
    (options.artifacts / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
