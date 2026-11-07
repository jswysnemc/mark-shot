#!/usr/bin/env python3
"""【画布导航对照】【只读采集】提取完整生产视图、坐标和滚轮函数，只在新证据目录编译"""

import argparse
import hashlib
import importlib.util
import json
import os
import re
import subprocess
from pathlib import Path

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("canvas_reference_source", PROJECT / "scripts/annotation-rotation-reference.py")
SOURCE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SOURCE)


def collect(original, root):
    """【画布导航对照】【完整采集】从original只读提取到新root，返回来源、生产函数和实际结果指纹"""
    root.mkdir(parents=True, exist_ok=False)
    harness = PROJECT / "tests/fixtures/qt-canvas-navigation"
    selected = {
        "shot_window_layout.cpp": ["ShotWindow::updateFrozenImageRect", "ShotWindow::zoomImageAt", "ShotWindow::resetImageZoom", "ShotWindow::panImageTo", "ShotWindow::updateImageScrollBars", "ShotWindow::setImageCenterFromScrollBars"],
        "shot_window_hit_testing.cpp": ["ShotWindow::imageNavigationAvailable", "ShotWindow::wheelZoomsImage"],
        "shot_window_annotation_editing.cpp": ["ShotWindow::clampImagePoint", "ShotWindow::widgetToImage", "ShotWindow::imageToWidget"],
        "shot_window_input_shortcuts.cpp": ["annotationWidthWheelSteps", "annotationWidthWheelStepSize", "ShotWindow::wheelEvent"],
        "shot_window_internal.cpp": ["imageNavigationWheelFactor"],
    }
    paths = {name: original / "qt/src" / name for name in selected}
    paths["shot_window_internal.h"] = original / "qt/src/shot_window_internal.h"
    style = original / "qt/src/ui/canvas_scrollbar_style.cpp"
    paths["ui/canvas_scrollbar_style.cpp"] = style
    paths["ui/canvas_scrollbar_style.h"] = original / "qt/src/ui/canvas_scrollbar_style.h"
    paths["shot_window_types.h"] = original / "qt/src/shot_window_types.h"
    before = {name: SOURCE.digest(path) for name, path in paths.items()}
    harness_before = {name: SOURCE.digest(harness / name) for name in ("probe.h", "reference.cpp", "style_reference.cpp")}
    functions = {name: SOURCE.function(paths[file].read_text(), name) for file, names in selected.items() for name in names}
    header = paths["shot_window_internal.h"].read_text()
    constants = [re.search(r"inline constexpr (?:qreal|qint64|int) " + name + r" = [^;]+;", header)[0] for name in ("kMinImageZoom", "kMaxImageZoom", "kImageScrollBarExtent", "kCtrlDoubleTapMs", "kMinStartupColorLoupeSize", "kMaxStartupColorLoupeSize")]
    (root / "constants.inc").write_text("namespace markshot::shot {\n" + "\n".join(constants) + "\n}\nusing namespace markshot::shot;\n")
    chunks = ["namespace markshot::shot {\n" + functions["imageNavigationWheelFactor"] + "\n}"]
    chunks.extend(value for name, value in functions.items() if name != "imageNavigationWheelFactor")
    (root / "production.inc").write_text("using Mode = ShotWindow::Mode;\nusing Tool = ShotWindow::Tool;\nusing StartupTool = ShotWindow::StartupTool;\nusing HistorySnapshot = ShotWindow::HistorySnapshot;\nusing namespace markshot::shot;\n" + "\n\n".join(chunks) + "\n")
    flags = subprocess.check_output(["pkg-config", "--cflags", "--libs", "Qt6Widgets"], text=True).split()
    with (root / "build.log").open("w") as log:
        subprocess.run(["c++", "-std=c++17", "-fPIC", str(harness / "reference.cpp"), str(harness / "style_reference.cpp"), str(style), "-I" + str(root), "-I" + str(harness), "-I" + str(original / "qt/src"), *flags, "-o", str(root / "reference")], stdout=log, stderr=log, check=True)
    environment = dict(os.environ, QT_QPA_PLATFORM="offscreen", QT_QPA_OFFSCREEN_NO_GLX="1", QT_FONT_DPI="96", QT_SCALE_FACTOR="1", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/canvas-reference")
    for name in ("DISPLAY", "WAYLAND_DISPLAY", "SWAYSOCK", "NIRI_SOCKET"):
        environment.pop(name, None)
    with (root / "stdout.log").open("w") as output, (root / "stderr.log").open("w") as errors:
        subprocess.run([str(root / "reference"), str(root / "reference.json")], env=environment, stdout=output, stderr=errors, check=True)
    assert before == {name: SOURCE.digest(path) for name, path in paths.items()}
    assert harness_before == {name: SOURCE.digest(harness / name) for name in harness_before}
    values = json.loads((root / "reference.json").read_text())
    assert all(SOURCE.digest(root / sample["image"]) == sample["sha256"] for sample in values["styles"])
    index = {"sources": before, "functions": {name: hashlib.sha256(text.encode()).hexdigest() for name, text in functions.items()}, "harness": harness_before, "binary": SOURCE.digest(root / "reference"), "reference": SOURCE.digest(root / "reference.json"), "counts": {name: len(values[name]) for name in ("geometry", "wheel", "factor", "styles")}, "geometry_steps": sum(len(value["steps"]) for value in values["geometry"]), "qt": values["qt"], "scope": "Complete production navigation, coordinate and wheel functions; real native scrollbars with complete production style; fixture callbacks only record width routing and omit full ShotWindow setup."}
    (root / "index.json").write_text(json.dumps(index, indent=2) + "\n")
    return index


def main():
    """【画布导航对照】【入口】解析只读original与新root，输出实际采集清单，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--original", type=Path, default=Path.home() / "workspace/mark-shot")
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(collect(options.original.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
