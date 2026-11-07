#!/usr/bin/env python3
"""【旋转文字对照】【只读采集】提取完整生产字体及文字测量函数，在重写目录编译并保存原始结果"""

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("rotation_reference", PROJECT / "scripts/annotation-rotation-reference.py")
SUPPORT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SUPPORT)


def collect(original, root):
    """【旋转文字对照】【生产采集】按original只读提取到新root，返回来源、函数、字号与结果指纹"""
    root.mkdir(parents=True, exist_ok=False)
    harness = PROJECT / "tests/fixtures/qt-annotation-rotation"
    selected = {"ui/theme.cpp": ("markshot::theme", ["makeFont", "uiFontFamily", "textFontFamily", "textFont"]),
                "text_annotation_layout.cpp": ("markshot::shot", ["prepareTextDocument", "textWrapWidth", "textAnnotationBaseRect", "textAnnotationWraps", "textAnnotationBoxSize"])}
    sources = {name: original / "qt/src" / name for name in selected}
    before = {name: SUPPORT.digest(path) for name, path in sources.items()}
    extracted = {}
    chunks = []
    for file, (namespace, names) in selected.items():
        functions = [SUPPORT.function(sources[file].read_text(), name) for name in names]
        extracted.update(zip(names, functions))
        chunks.append("namespace " + namespace + " {\n" + "\n\n".join(functions) + "\n}\n")
    (root / "text-production.inc").write_text("\n".join(chunks))
    flags = subprocess.check_output(["pkg-config", "--cflags", "--libs", "Qt6Widgets"], text=True).split()
    with (root / "build.log").open("w") as log:
        subprocess.run(["c++", "-std=c++17", "-fPIC", str(harness / "text_reference.cpp"), "-I" + str(root),
                        "-I" + str(harness), *flags, "-o", str(root / "reference")], stdout=log, stderr=log, check=True)
    subprocess.run([str(root / "reference"), str(root / "reference.json")],
                   env=dict(os.environ, QT_QPA_PLATFORM="offscreen", QT_FONT_DPI="96", QT_SCALE_FACTOR="1"),
                   check=True, capture_output=True)
    assert before == {name: SUPPORT.digest(path) for name, path in sources.items()}
    values = json.loads((root / "reference.json").read_text())
    index = {"sources": before, "functions": {name: hashlib.sha256(text.encode()).hexdigest() for name, text in extracted.items()},
             "harness": {name: SUPPORT.digest(harness / name) for name in ("text_reference.cpp", "text_probe.h")},
             "binary": SUPPORT.digest(root / "reference"), "reference": SUPPORT.digest(root / "reference.json"),
             "samples": len(values["samples"]), "qt": values["qt"], "dpi": values["dpi"], "family": values["family"]}
    (root / "index.json").write_text(json.dumps(index, indent=2) + "\n")
    return index


def main():
    """【旋转文字对照】【入口】解析只读原版及新目录，输出完整采集指纹，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--original", type=Path, default=Path.home() / "workspace/mark-shot")
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(collect(options.original.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
