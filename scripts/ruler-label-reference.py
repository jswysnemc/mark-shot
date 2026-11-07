#!/usr/bin/env python3
"""【尺子说明对照】【只读采集】执行完整Qt窗口说明入口，报告布局并校验相同生产像素"""

import argparse
import hashlib
import importlib.util
import json
import os
import subprocess
from pathlib import Path

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("ruler_label_source", PROJECT / "scripts/annotation-rotation-reference.py")
SOURCE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SOURCE)


def collect(original, root):
    """【尺子说明对照】【完整采集】按original只读编译到新root，返回来源指纹、数量与覆盖范围"""
    root.mkdir(parents=True, exist_ok=False)
    base = original / "qt/src"
    harness = PROJECT / "tests/fixtures/qt-ruler-labels"
    selected = {
        "shot_window_internal.cpp": ("markshot::shot", ["rulerTickStep"]),
        "shot_window_hit_testing.cpp": (None, ["ShotWindow::annotationSizeScale"]),
        "shot_window_annotation_editing.cpp": (None, ["ShotWindow::normalizedSelection", "ShotWindow::imageRectToWidget"]),
        "shot_window_ruler_painter.cpp": (None, ["ShotWindowRulerPainter::ShotWindowRulerPainter", "ShotWindowRulerPainter::unitFor", "ShotWindowRulerPainter::labelBounds", "ShotWindowRulerPainter::lengthGeometry", "ShotWindowRulerPainter::angleGeometry", "ShotWindowRulerPainter::mapPoint", "ShotWindowRulerPainter::mapRect", "ShotWindowRulerPainter::drawLabels"]),
    }
    complete = ["ui/theme.cpp", "ruler/ruler_geometry.cpp"]
    dependencies = ["ruler/ruler_painter.cpp", "ruler/angle_painter.cpp", "ruler/ruler_painter.h", "ruler/angle_painter.h", "ruler/ruler_geometry.h", "shot_window_types.h", "shot_window_ruler_painter.h", "ui/theme.h"]
    paths = {name: base / name for name in [*selected, *complete, *dependencies]}
    before = {name: SOURCE.digest(path) for name, path in paths.items()}
    chunks, functions = [], {}
    for file, (namespace, names) in selected.items():
        values = [SOURCE.function(paths[file].read_text(), name) for name in names]
        functions.update(zip(names, values))
        chunk = "\n\n".join(values)
        chunks.append("namespace " + namespace + " {\n" + chunk + "\n}\n" if namespace else chunk)
    (root / "production.inc").write_text("using namespace markshot::shot;\nusing namespace markshot::shot::types;\n" + "\n\n".join(chunks))
    (root / "shot_window_internal.h").write_text("#pragma once\nnamespace markshot::shot { int rulerTickStep(double); }\n")
    flags = subprocess.check_output(["pkg-config", "--cflags", "--libs", "Qt6Widgets"], text=True).split()
    with (root / "build.log").open("w") as log:
        subprocess.run(["c++", "-std=c++17", "-fPIC", str(harness / "reference.cpp"), *[str(base / name) for name in complete], "-I" + str(root), "-I" + str(harness), "-I" + str(base), *flags, "-o", str(root / "reference")], stdout=log, stderr=log, check=True)
    # 1. 【尺子说明对照】【不可见执行】固定逻辑DPI，不连接用户桌面，报告与完整生产绘制不一致即失败
    with (root / "run.log").open("w") as log:
        subprocess.run([str(root / "reference"), str(root)], env=dict(os.environ, QT_QPA_PLATFORM="offscreen", QT_FONT_DPI="96", QT_SCALE_FACTOR="1"), stdout=log, stderr=log, check=True)
    assert before == {name: SOURCE.digest(path) for name, path in paths.items()}
    values = json.loads((root / "reference.json").read_bytes())
    index = {
        "sources": before,
        "functions": {name: hashlib.sha256(text.encode()).hexdigest() for name, text in functions.items()},
        "harness": {name: SOURCE.digest(harness / name) for name in ("probe.h", "reference.cpp")},
        "collector": SOURCE.digest(Path(__file__)), "extraction_helper": SOURCE.digest(PROJECT / "scripts/annotation-rotation-reference.py"),
        "dependency_declarations": SOURCE.digest(root / "shot_window_internal.h"), "binary": SOURCE.digest(root / "reference"), "reference": SOURCE.digest(root / "reference.json"),
        "scenes": len(values["scenes"]), "labels": sum(len(scene["labels"]) for scene in values["scenes"]),
        "frames": {path.name: SOURCE.digest(path) for path in sorted(root.glob("frame-*.png"))}, "qt": values["qt"],
        "coverage": "Complete production ShotWindowRulerPainter::drawLabels and complete ruler_painter.cpp, angle_painter.cpp and ruler_geometry.cpp. 4 image/window projections, 4 capture ratios, 3 widths, 3 selection/translation contexts, 10 persistent/draft/mixed/flat/degenerate patterns, both window and export targets. Each scene compares the reported production geometry with complete production render pixel-for-pixel. Affine point mapping and window rect are explicit state providers. Text and magnifier dependency branches abort. Native input, property-toolbar avoidance, hover chips and complete screenshot UI pixel parity are outside this capture.",
    }
    (root / "index.json").write_text(json.dumps(index, indent=2) + "\n")
    return index


def main():
    """【尺子说明对照】【入口】解析只读原版目录与新输出目录，打印证据清单，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--original", type=Path, default=Path.home() / "workspace/mark-shot")
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(collect(options.original.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
