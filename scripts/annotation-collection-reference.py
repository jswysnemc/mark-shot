#!/usr/bin/env python3
"""【标注多选对照】【只读采集】提取完整生产编号、边界、群组变换和宽度函数，只在新证据目录编译"""

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
    """【标注多选对照】【完整采集】按original只读提取到新root，返回来源、完整函数与结果指纹"""
    root.mkdir(parents=True, exist_ok=False)
    harness = PROJECT / "tests/fixtures/qt-annotation-collection"
    selected = {
        "ui/theme.cpp": ("markshot::theme", ["makeFont", "uiFontFamily", "textFontFamily", "textFont"]),
        "text_annotation_layout.cpp": ("markshot::shot", ["prepareTextDocument", "textWrapWidth", "textAnnotationBaseRect", "textAnnotationWraps", "textAnnotationBoxSize", "fitTextAnnotationRect"]),
        "ruler/ruler_geometry.cpp": ("markshot::ruler", ["inflatedToPixel", "rotatedAround", "rulerBounds", "rotateAnglePoints"]),
        "annotation/rotation_drag.cpp": ("markshot::annotation", ["rotationDragDegrees", "snappedRotationDegrees"]),
        "shot_window_internal.cpp": ("markshot::shot", ["normalizedRotationDegrees", "clampedMagnifierScale"]),
        "shot_window_text_layout.cpp": (None, ["ShotWindow::textContentRect"]),
        "shot_window_annotation_editing.cpp": (None, ["ShotWindow::clampImagePoint", "ShotWindow::clampedMagnifierCircleCenter", "ShotWindow::magnifierCircleRect", "ShotWindow::clampedMagnifierRect", "ShotWindow::magnifierSourceRect", "ShotWindow::moveAnnotation", "ShotWindow::transformAnnotation", "ShotWindow::updateAnnotationDrag", "ShotWindow::setCurrentColor"]),
        "shot_window_property_panels.cpp": (None, ["ShotWindow::applyPropertyColor"]),
        "shot_window_hit_testing.cpp": (None, ["ShotWindow::annotationSizeScale", "ShotWindow::annotationSupportsRotation", "ShotWindow::annotationSupportsLineControl", "ShotWindow::annotationLineControlPoint", "ShotWindow::rotatedPoint", "ShotWindow::rotatedRectBounds", "ShotWindow::annotationUnrotatedBounds", "ShotWindow::annotationRotationCenter", "ShotWindow::selectedAnnotationIds", "ShotWindow::setSelectedAnnotations", "ShotWindow::selectedAnnotationsBounds", "ShotWindow::annotationsInRect", "ShotWindow::annotationBounds"]),
        "shot_window_line_anchors.cpp": (None, ["ShotWindow::annotationSupportsLineAnchors", "ShotWindow::updateLineAnchorDrag"]),
        "shot_window_selection_frame.cpp": (None, ["ShotWindow::resizedBounds"]),
        "shot_window_ruler_drag.cpp": (None, ["ShotWindowRulerTool::rotateInGroup"]),
        "shot_window_width_state.cpp": (None, ["ShotWindow::setSelectedAnnotationWidth"]),
    }
    paths = {name: original / "qt/src" / name for name in selected}
    paths["shot_window_internal.h"] = original / "qt/src/shot_window_internal.h"
    paths["line_skeleton_path.h"] = original / "qt/src/line_skeleton_path.h"
    before = {name: SOURCE.digest(path) for name, path in paths.items()}
    chunks, functions = [], {}
    for file, (namespace, names) in selected.items():
        text = paths[file].read_text()
        if file == "shot_window_width_state.cpp":
            text = text[text.index("bool ShotWindow::setSelectedAnnotationWidth(int width, bool captureHistory)"):]
        values = [SOURCE.function(text, name) for name in names]
        functions.update(zip(names, values))
        chunk = "\n\n".join(values)
        chunks.append("namespace " + namespace + " {\n" + chunk + "\n}\n" if namespace else chunk)
    constants = ["kMinSelectionSize", "kMinStrokeWidth", "kMaxStrokeWidth", "kMaxHighlighterWidth", "kMinNumberWidth", "kMaxNumberWidth", "kMinMosaicBlockSize", "kMaxMosaicBlockSize", "kTextBackgroundPaddingX", "kTextBackgroundPaddingY", "kMinMagnifierScale", "kMaxMagnifierScale", "kLineSkeletonFirstPointIndex"]
    header = paths["shot_window_internal.h"].read_text() + "\n" + paths["line_skeleton_path.h"].read_text()
    definitions = [re.search(r"inline constexpr (?:qreal|int) " + name + r" = [^;]+;", header)[0] for name in constants]
    (root / "constants.inc").write_text("namespace markshot::shot {\n" + "\n".join(definitions) + "\n}\n")
    (root / "production.inc").write_text("using namespace markshot::shot;\n" + "\n\n".join(chunks))
    flags = subprocess.check_output(["pkg-config", "--cflags", "--libs", "Qt6Widgets"], text=True).split()
    with (root / "build.log").open("w") as log:
        subprocess.run(["c++", "-std=c++17", "-fPIC", str(harness / "reference.cpp"), "-I" + str(root), "-I" + str(harness), *flags, "-o", str(root / "reference")], stdout=log, stderr=log, check=True)
    subprocess.run([str(root / "reference"), str(root / "reference.json")], env=dict(os.environ, QT_QPA_PLATFORM="offscreen", QT_FONT_DPI="96", QT_SCALE_FACTOR="1"), check=True, capture_output=True)
    # 1. 【标注多选对照】【原生尺寸】原版完整样式文件直接参与编译，setup控件参数由界面对照器提供并单独登记
    styles = [original / "qt/src/ui/toolbar_panel_style.cpp", original / "qt/src/ui/toolbar_palette.cpp"]
    styles_before = {str(path.relative_to(original)): SOURCE.digest(path) for path in styles}
    with (root / "panel-build.log").open("w") as log:
        subprocess.run(["c++", "-std=c++17", "-fPIC", str(harness / "panel.cpp"), *map(str, styles), "-I" + str(original / "qt/src"), *flags, "-o", str(root / "panel")], stdout=log, stderr=log, check=True)
    subprocess.run([str(root / "panel"), str(root / "panel.json")], env=dict(os.environ, QT_QPA_PLATFORM="offscreen", QT_FONT_DPI="96", QT_SCALE_FACTOR="1"), check=True, capture_output=True)
    assert styles_before == {str(path.relative_to(original)): SOURCE.digest(path) for path in styles}
    assert before == {name: SOURCE.digest(path) for name, path in paths.items()}
    values = json.loads((root / "reference.json").read_text())
    index = {"sources": before, "functions": {name: hashlib.sha256(text.encode()).hexdigest() for name, text in functions.items()}, "harness": {name: SOURCE.digest(harness / name) for name in ("probe.h", "reference.cpp")}, "binary": SOURCE.digest(root / "reference"), "reference": SOURCE.digest(root / "reference.json"), "counts": {name: len(values[name]) for name in ("selection", "transform", "width", "sequence", "color", "line_edit")}, "qt": values["qt"]}
    index["panel"] = {"sources": styles_before, "harness": SOURCE.digest(harness / "panel.cpp"), "reference": SOURCE.digest(root / "panel.json"), "binary": SOURCE.digest(root / "panel")}
    (root / "index.json").write_text(json.dumps(index, indent=2) + "\n")
    return index


def main():
    """【标注多选对照】【入口】解析只读original与新root，输出采集清单，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--original", type=Path, default=Path.home() / "workspace/mark-shot")
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(collect(options.original.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
