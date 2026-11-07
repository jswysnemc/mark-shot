#!/usr/bin/env python3
"""【标注样式对照】【只读采集】提取完整生产绘制函数，在新目录运行Qt离屏绘制并保存来源指纹"""

import argparse
import hashlib
import importlib.util
import json
import os
import re
import subprocess
from pathlib import Path

from PIL import Image

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("style_reference_source", PROJECT / "scripts/annotation-rotation-reference.py")
SOURCE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SOURCE)


def metrics(root, values):
    """【标注样式对照】【像素度量】按root内真实Qt图像与values返回边界和固定像素样本，不复写生产尺寸公式"""
    rows = []
    for sample in values["samples"]:
        image = Image.open(root / sample["image"]).convert("RGBA")
        points = [(index % image.width, index // image.width) for index, pixel in enumerate(image.getdata()) if pixel != (255, 255, 255, 255)]
        bounds = [min(point[0] for point in points), min(point[1] for point in points), max(point[0] for point in points) + 1, max(point[1] for point in points) + 1]
        device = sample["device"]
        colors = [list(image.getpixel((round(x * device), round(240 * device)))) for x in (260, 320, 370)]
        rows.append({key: sample[key] for key in ("tool", "scale", "width", "device", "image", "sha256")} | {"bounds": bounds, "colors": colors})
    return {"qt": values["qt"], "font": values["font"], "samples": rows}


def collect(original, root):
    """【标注样式对照】【生产采集】按original只读提取到新root，返回完整函数、文件和绘制图像的指纹清单"""
    root.mkdir(parents=True, exist_ok=False)
    harness = PROJECT / "tests/fixtures/qt-annotation-style"
    selected = {
        "shot_window_annotation_painting.cpp": ["ShotWindow::drawAnnotation", "ShotWindow::drawNumber"],
        "shot_window_hit_testing.cpp": ["ShotWindow::annotationSizeScale"],
        "shot_window_annotation_editing.cpp": ["ShotWindow::imageToWidget", "ShotWindow::imageRectToWidget"],
        "shot_window_internal.cpp": ["smoothedStrokePath"],
    }
    linked = ["ui/theme.cpp", "line_skeleton_path.cpp", "annotation/arrow_rendering.cpp"]
    paths = {name: original / "qt/src" / name for name in [*selected, *linked, "shot_window_internal.h", "shot_window_types.h", "ui/theme.h", "annotation/arrow_rendering.h", "annotation/arrow_style.h", "line_skeleton_path.h"]}
    before = {name: SOURCE.digest(path) for name, path in paths.items()}
    harness_before = {name: SOURCE.digest(harness / name) for name in ("probe.h", "reference.cpp")}
    functions = {name: SOURCE.function(paths[file].read_text(), name) for file, names in selected.items() for name in names}
    constants = [re.search(r"inline constexpr qreal " + name + r" = [^;]+;", paths["shot_window_internal.h"].read_text())[0] for name in ("kTextBackgroundPaddingX", "kTextBackgroundPaddingY")]
    (root / "constants.inc").write_text("namespace markshot::shot {\n" + "\n".join(constants) + "\n}\n")
    chunks = ["namespace markshot::shot {\n" + functions["smoothedStrokePath"] + "\n}"]
    chunks.extend(value for name, value in functions.items() if name != "smoothedStrokePath")
    (root / "production.inc").write_text("using namespace markshot::shot;\n" + "\n\n".join(chunks) + "\n")
    flags = subprocess.check_output(["pkg-config", "--cflags", "--libs", "Qt6Widgets"], text=True).split()
    with (root / "build.log").open("w") as log:
        subprocess.run(["c++", "-std=c++17", "-fPIC", str(harness / "reference.cpp"), *[str(paths[name]) for name in linked], "-I" + str(root), "-I" + str(harness), "-I" + str(original / "qt/src"), *flags, "-o", str(root / "reference")], stdout=log, stderr=log, check=True)
    environment = dict(os.environ, QT_QPA_PLATFORM="offscreen", QT_QPA_OFFSCREEN_NO_GLX="1", QT_FONT_DPI="96", QT_SCALE_FACTOR="1", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/style-reference")
    for name in ("DISPLAY", "WAYLAND_DISPLAY", "SWAYSOCK", "NIRI_SOCKET"):
        environment.pop(name, None)
    with (root / "stdout.log").open("w") as output, (root / "stderr.log").open("w") as errors:
        subprocess.run([str(root / "reference"), str(root)], env=environment, stdout=output, stderr=errors, check=True)
    assert before == {name: SOURCE.digest(path) for name, path in paths.items()}
    assert harness_before == {name: SOURCE.digest(harness / name) for name in harness_before}
    values = json.loads((root / "reference.json").read_text())
    assert all(SOURCE.digest(root / sample["image"]) == sample["sha256"] for sample in values["samples"])
    measured = metrics(root, values)
    (root / "metrics.json").write_text('{"qt":' + json.dumps(measured["qt"]) + ',"font":' + json.dumps(measured["font"]) + ',"samples":[\n' + ',\n'.join(json.dumps(row, separators=(",", ":")) for row in measured["samples"]) + '\n]}\n')
    index = {"sources": before, "functions": {name: hashlib.sha256(text.encode()).hexdigest() for name, text in functions.items()}, "harness": harness_before, "binary": SOURCE.digest(root / "reference"), "reference": SOURCE.digest(root / "reference.json"), "samples": len(values["samples"]), "qt": values["qt"], "scope": "Complete production drawAnnotation, drawNumber, view conversion and smoothing; complete linked theme, line skeleton and arrow rendering. Nonrotated pen, line, freehand highlighter, ellipse, arrow and Arabic number only. Rectangle, marker, ruler, filter, magnifier, text and nonzero rotation branches abort if reached. No full ShotWindow setup or native input is claimed."}
    index["metrics"] = SOURCE.digest(root / "metrics.json")
    (root / "index.json").write_text(json.dumps(index, indent=2) + "\n")
    return index


def main():
    """【标注样式对照】【入口】解析original与root后执行采集并输出清单，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--original", type=Path, default=Path.home() / "workspace/mark-shot")
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(collect(options.original.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
