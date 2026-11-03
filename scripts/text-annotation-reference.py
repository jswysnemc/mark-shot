#!/usr/bin/env python3
"""【文字对照】【原版证据】执行只读 Qt 探针与 Rust 场景，记录字号、文字框和真实像素差异"""

import argparse
import json
import os
from pathlib import Path
import subprocess

from PIL import Image, ImageChops, ImageStat


CASES = {
    "default": {},
    "regular": {"bold": False},
    "italic": {"italic": True},
    "background": {"color": "#FF4D4D80", "background": "#10203060"},
    "invisible-text": {"color": "#FF4D4D00", "background": "#10203060"},
    "half-scale": {"scale": 0.5},
    "three-quarter-scale": {"scale": 0.75},
    "large-scale": {"scale": 1.25},
    "multiline": {"text": "Mark Shot\nsecond line\n"},
    "wrapped": {"text": "one two three four five six", "wrap_width": 100},
    "wrapped-half": {"text": "one two three four five six", "wrap_width": 100, "scale": 0.5},
    "narrow-no-wrap": {"text": "Mark Shot", "wrap_width": 20},
    "minimum": {"points": 8},
    "empty": {"text": ""},
    "cjk": {"text": "文字标注测试\n日本語"},
}


def execute(arguments, environment, log):
    """【文字对照】【执行记录】用 environment 执行 arguments，输出写入 log；失败抛出异常，无返回值"""
    subprocess.run(arguments, env=environment, stdout=log, stderr=subprocess.STDOUT, check=True)


def compare(qt, rust, output, name):
    """【文字对照】【真实差异】比较 name 的原始 Qt、Rust 缓冲，output 保存并排图；返回度量与像素指标，不缩放或对齐字形"""
    first_metrics = json.loads((qt / f"{name}.json").read_text())
    second_metrics = json.loads((rust / f"{name}.json").read_text())
    with Image.open(qt / f"{name}.png") as image:
        first = image.convert("RGBA")
    with Image.open(rust / f"{name}.png") as image:
        second = image.convert("RGBA")
    size = (max(first.width, second.width), max(first.height, second.height))
    padded = []
    for image in (first, second):
        layer = Image.new("RGBA", size)
        layer.paste(image)
        padded.append(layer)
    raw = ImageChops.difference(*padded)
    background = Image.new("RGBA", size, (30, 34, 38, 255))
    left, right = [Image.alpha_composite(background, image).convert("RGB") for image in padded]
    difference = ImageChops.difference(left, right)
    montage = Image.new("RGB", (size[0] * 3, size[1]))
    for index, image in enumerate((left, right, difference)):
        montage.paste(image, (size[0] * index, 0))
    montage.resize((montage.width * 2, montage.height * 2)).save(output / f"{name}-comparison.png")
    return {
        "qt": first_metrics,
        "rust": second_metrics,
        "font_pixels_equal": first_metrics["font_pixels"] == second_metrics["font_pixels"],
        "bounds_equal": first_metrics["bounds"] == second_metrics["bounds"],
        "bounds_delta": [second - first for first, second in zip(first_metrics["bounds"], second_metrics["bounds"])],
        "raw_rgba_exact": not any(any(pixel) for pixel in raw.get_flattened_data()),
        "raw_rgba_mae": sum(ImageStat.Stat(raw).mean) / 4,
        "composited_rgb_mae": sum(ImageStat.Stat(difference).mean) / 3,
        "composited_rgb_max_channel_delta": max(value[1] for value in difference.getextrema()),
    }


def main():
    """【文字对照】【命令入口】按命令参数生成隔离环境和十五个场景，输出 artifacts 的原始结果；Qt 源码只读，不访问用户配置"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--qt-binary", type=Path, default=Path("target/qt-text-reference/text-reference"))
    options = parser.parse_args()
    artifacts = options.artifacts.resolve()
    cases, qt, rust = [artifacts / name for name in ("cases", "qt", "rust")]
    for directory in (cases, qt, rust):
        directory.mkdir(parents=True, exist_ok=True)
    runtime = artifacts / "runtime"
    runtime.mkdir(mode=0o700, exist_ok=True)
    runtime.chmod(0o700)
    environment = dict(os.environ, QT_QPA_PLATFORM="offscreen", QT_FONT_DPI="96",
                       XDG_RUNTIME_DIR=str(runtime), XDG_CONFIG_HOME=str(artifacts / "config"),
                       XDG_DATA_HOME=str(artifacts / "data"), XDG_STATE_HOME=str(artifacts / "state"))
    with (artifacts / "execution.log").open("w") as log:
        for name, config in CASES.items():
            (cases / f"{name}.json").write_text(json.dumps(config, ensure_ascii=False) + "\n")
            execute([str(options.qt_binary.resolve()), str(cases / f"{name}.json"), str(qt / name)], environment, log)
        environment.update(MARK_SHOT_TEXT_CASES=str(cases), MARK_SHOT_TEXT_ARTIFACTS=str(rust))
        execute(["cargo", "test", "--test", "text_annotation", "render_reference_fixture", "--", "--ignored", "--exact"], environment, log)
    result = {name: compare(qt, rust, artifacts, name) for name in CASES}
    (artifacts / "result.json").write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n")
    print(json.dumps({name: {key: item[key] for key in ("font_pixels_equal", "bounds_equal", "bounds_delta", "composited_rgb_mae")}
                      for name, item in result.items()}, indent=2))


if __name__ == "__main__":
    main()
