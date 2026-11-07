#!/usr/bin/env python3
"""【文字存储对照】【只读采集】提取完整原版变换、缩放、字体和排版函数，只在重写项目的新目录编译"""

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("rotation_source", PROJECT / "scripts/annotation-rotation-reference.py")
SOURCE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SOURCE)


def collect(original, root):
    """【文字存储对照】【完整采集】按original只读提取到新root，返回生产函数、原始结果和宿主指纹"""
    root.mkdir(parents=True, exist_ok=False)
    harness = PROJECT / "tests/fixtures/qt-text-storage"
    selected = {"ui/theme.cpp": ("markshot::theme", ["makeFont", "uiFontFamily", "textFontFamily", "textFont"]),
        "text_annotation_layout.cpp": ("markshot::shot", ["prepareTextDocument", "textWrapWidth", "textAnnotationBaseRect", "textAnnotationWraps", "textAnnotationBoxSize", "fitTextAnnotationRect"]),
        "shot_window_annotation_editing.cpp": (None, ["ShotWindow::clampImagePoint", "ShotWindow::moveAnnotation", "ShotWindow::transformAnnotation"]),
        "shot_window_selection_frame.cpp": (None, ["ShotWindow::resizedBounds"])}
    paths = {name: original / "qt/src" / name for name in selected}
    before = {name: SOURCE.digest(path) for name, path in paths.items()}
    chunks, functions = [], {}
    for file, (namespace, names) in selected.items():
        values = [SOURCE.function(paths[file].read_text(), name) for name in names]
        functions.update(zip(names, values))
        chunk = "\n\n".join(values)
        chunks.append("namespace " + namespace + " {\n" + chunk + "\n}\n" if namespace else chunk)
    (root / "production.inc").write_text("using namespace markshot::shot;\n" + "\n\n".join(chunks))
    flags = subprocess.check_output(["pkg-config", "--cflags", "--libs", "Qt6Widgets"], text=True).split()
    with (root / "build.log").open("w") as log:
        subprocess.run(["c++", "-std=c++17", "-fPIC", str(harness / "reference.cpp"), "-I" + str(root), "-I" + str(harness),
            *flags, "-o", str(root / "reference")], stdout=log, stderr=log, check=True)
    subprocess.run([str(root / "reference"), str(root / "reference.json")],
        env=dict(os.environ, QT_QPA_PLATFORM="offscreen", QT_FONT_DPI="96", QT_SCALE_FACTOR="1"), check=True, capture_output=True)
    assert before == {name: SOURCE.digest(path) for name, path in paths.items()}
    values = json.loads((root / "reference.json").read_text())
    index = {"sources": before, "functions": {name: hashlib.sha256(text.encode()).hexdigest() for name, text in functions.items()},
        "harness": {name: SOURCE.digest(harness / name) for name in ("probe.h", "reference.cpp")},
        "reference": SOURCE.digest(root / "reference.json"), "binary": SOURCE.digest(root / "reference"),
        "counts": {name: len(values[name]) for name in ("sequences", "single", "scales", "fallbacks")}, "qt": values["qt"], "dpi": values["dpi"]}
    (root / "index.json").write_text(json.dumps(index, indent=2) + "\n")
    return index


def main():
    """【文字存储对照】【入口】解析只读原版及新证据目录，输出完整指纹，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--original", type=Path, default=Path.home() / "workspace/mark-shot")
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(collect(options.original.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
