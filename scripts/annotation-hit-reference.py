#!/usr/bin/env python3
"""【标注命中对照】【只读采集】执行完整Qt生产命中入口，记录实际类型、依赖及结果指纹"""

import argparse
import hashlib
import importlib.util
import json
import os
import re
import subprocess
from pathlib import Path

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("rotation_source", PROJECT / "scripts/annotation-rotation-reference.py")
SOURCE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SOURCE)


def collect(original, root):
    """【标注命中对照】【完整采集】按original只读编译到新root，返回来源、完整入口和采集范围"""
    root.mkdir(parents=True, exist_ok=False)
    base = original / "qt/src"
    harness = PROJECT / "tests/fixtures/qt-annotation-hit"
    selected = {
        "shot_window_internal.cpp": ("markshot::shot", ["clampedMagnifierScale", "ellipseContainsPoint", "rulerTickStep"]),
        "shot_window_hit_testing.cpp": (None, ["ShotWindow::annotationSizeScale", "ShotWindow::annotationSupportsRotation", "ShotWindow::annotationSupportsLineControl", "ShotWindow::rotatedPoint", "ShotWindow::rotatedRectBounds", "ShotWindow::annotationUnrotatedBounds", "ShotWindow::annotationRotationCenter", "ShotWindow::annotationRotationHandlePoint", "ShotWindow::annotationBounds", "ShotWindow::annotationBoundsDragAt", "ShotWindow::annotationDragAt", "ShotWindow::annotationAt"]),
        "shot_window_selection_frame.cpp": (None, ["ShotWindow::magnifierDragAt", "ShotWindow::numberDragAt"]),
        "shot_window_line_anchors.cpp": (None, ["ShotWindow::annotationSupportsLineAnchors", "ShotWindow::lineAnchorPointIndexAt", "ShotWindow::lineAnchorDragAt"]),
        "shot_window_annotation_editing.cpp": (None, ["ShotWindow::clampedMagnifierCircleCenter", "ShotWindow::magnifierCircleRect", "ShotWindow::magnifierSourceRect", "ShotWindow::clampedMagnifierRect"]),
        "shot_window_ruler_tool.cpp": (None, ["ShotWindowRulerTool::imageTolerance"]),
        "shot_window_ruler_drag.cpp": (None, ["ShotWindowRulerTool::rotationHandleHit", "ShotWindowRulerTool::dragAt"]),
        "shot_window_ruler_painter.cpp": (None, ["ShotWindowRulerPainter::ShotWindowRulerPainter", "ShotWindowRulerPainter::handleClearance", "ShotWindowRulerPainter::rotationHandleGap", "ShotWindowRulerPainter::unitFor", "ShotWindowRulerPainter::labelBounds", "ShotWindowRulerPainter::lengthGeometry", "ShotWindowRulerPainter::mapPoint", "ShotWindowRulerPainter::mapRect", "ShotWindowRulerPainter::angleFrame"]),
    }
    complete = ["ui/theme.cpp", "ruler/ruler_geometry.cpp", "ruler/ruler_painter.cpp", "line_skeleton_path.cpp"]
    dependencies = ["shot_window_types.h", "shot_window_internal.h", "shot_window_ruler_tool.h", "shot_window_ruler_painter.h", "line_skeleton_path.h", "ruler/ruler_geometry.h", "ruler/ruler_painter.h", "ui/theme.h", "annotation/arrow_style.h", "shot_window_canvas.cpp"]
    paths = {name: base / name for name in [*selected, *complete, *dependencies]}
    before = {name: SOURCE.digest(path) for name, path in paths.items()}
    chunks, functions = [], {}
    for file, (namespace, names) in selected.items():
        values = [SOURCE.function(paths[file].read_text(), name) for name in names]
        functions.update(zip(names, values))
        chunk = "\n\n".join(values)
        chunks.append("namespace " + namespace + " {\n" + chunk + "\n}\n" if namespace else chunk)
    constants = "\n".join(re.search(r"constexpr qreal " + name + r" = [^;]+;", paths[file].read_text())[0] for file, name in [("shot_window_ruler_painter.cpp", "kAngleFramePadding"), ("shot_window_ruler_painter.cpp", "kRotationStemLength"), ("shot_window_ruler_drag.cpp", "kRotationHandleHitScale")])
    internal = paths["shot_window_internal.h"].read_text()
    # 1. 【标注命中对照】【倍率常量】使用生产声明，不替换放大镜倍率范围
    constants += "\nnamespace markshot::shot {\n" + "\n".join(re.search(r"inline constexpr qreal " + name + r" = [^;]+;", internal)[0] for name in ("kMinMagnifierScale", "kMaxMagnifierScale")) + "\n}\n"
    (root / "production.inc").write_text("using namespace markshot::shot;\n" + constants + "\n" + "\n\n".join(chunks))
    # 2. 【标注命中对照】【依赖声明】完整尺子绘制文件只需要此生产函数声明，避免引入截图窗口和录制依赖
    (root / "shot_window_internal.h").write_text("#pragma once\nnamespace markshot::shot { int rulerTickStep(double); }\n")
    flags = subprocess.check_output(["pkg-config", "--cflags", "--libs", "Qt6Widgets"], text=True).split()
    with (root / "build.log").open("w") as log:
        subprocess.run(["c++", "-std=c++17", "-fPIC", str(harness / "reference.cpp"), *[str(base / name) for name in complete], "-I" + str(root), "-I" + str(harness), "-I" + str(base), *flags, "-o", str(root / "reference")], stdout=log, stderr=log, check=True)
    subprocess.run([str(root / "reference"), str(root / "reference.json")], env=dict(os.environ, QT_QPA_PLATFORM="offscreen", QT_FONT_DPI="96", QT_SCALE_FACTOR="1"), check=True, capture_output=True)
    assert before == {name: SOURCE.digest(path) for name, path in paths.items()}
    values = json.loads((root / "reference.json").read_text())
    index = {"sources": before, "functions": {name: hashlib.sha256(text.encode()).hexdigest() for name, text in functions.items()}, "harness": {name: SOURCE.digest(harness / name) for name in ("probe.h", "reference.cpp")}, "collector": SOURCE.digest(Path(__file__)), "extraction_helper": SOURCE.digest(PROJECT / "scripts/annotation-rotation-reference.py"), "dependency_declarations": SOURCE.digest(root / "shot_window_internal.h"), "binary": SOURCE.digest(root / "reference"), "reference": SOURCE.digest(root / "reference.json"), "counts": {name: len(values[name]) for name in ("scenes", "samples")}, "qt": values["qt"], "coverage": "Rectangle, ellipse, marker, mosaic, pen, freehand highlighter, two-point line/arrow, number, both magnifier shapes, length/angle ruler. Fixed image width 640 and stroke width 6; three image/window ratios and rotations; one object per scene. Complete annotationAt and annotationDragAt are executed; ruler selected/unselected body operations are recorded. Text aborts if executed. Straight highlighter, additional styles/widths, curved line anchors, layering, native input and complete interface parity are outside this capture."}
    (root / "index.json").write_text(json.dumps(index, indent=2) + "\n")
    return index


def main():
    """【标注命中对照】【入口】解析只读original与新root，输出结果指纹，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--original", type=Path, default=Path.home() / "workspace/mark-shot")
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(collect(options.original.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
