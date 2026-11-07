#!/usr/bin/env python3
"""【普通标注旋转对照】【只读采集】完整提取原版生产函数，仅在重写项目的新目录编译和保存证据"""

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess


def digest(path):
    """【普通标注旋转对照】【文件指纹】按 path 返回 SHA-256，不修改来源"""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def function(source, name):
    """【普通标注旋转对照】【完整函数】按 source 与 name 提取自由或成员定义，返回未经改写的完整函数源码"""
    match = re.search(r"^[^\n;{}]*\b" + re.escape(name) + r"\([^;{}]*\)\s*(?:const\s*)?\{", source, re.M)
    if not match:
        raise AssertionError(f"production function was not found: {name}")
    opened, depth = source.index("{", match.start()), 0
    for offset in range(opened, len(source)):
        depth += (source[offset] == "{") - (source[offset] == "}")
        if depth == 0:
            return source[match.start():offset + 1]
    raise AssertionError(f"unclosed production function: {name}")


def collect(original, root):
    """【普通标注旋转对照】【实际计算】按 original 只读采集到新 root，返回来源、函数、探针、结果指纹"""
    root.mkdir(parents=True, exist_ok=False)
    harness = Path(__file__).resolve().parents[1] / "tests/fixtures/qt-annotation-rotation"
    selected = {"shot_window_internal.cpp": ["normalizedRotationDegrees"],
                "annotation/rotation_drag.cpp": ["rotationDragDegrees", "snappedRotationDegrees"],
                "shot_window_annotation_editing.cpp": ["ShotWindow::clampImagePoint"],
                "shot_window_selection_frame.cpp": ["ShotWindow::resizedBounds"]}
    sources = {name: original / "qt/src" / name for name in selected}
    before = {name: digest(path) for name, path in sources.items()}
    extracted = {name: function(sources[file].read_text(), name) for file, names in selected.items() for name in names}
    (root / "production.inc").write_text("\n\n".join(extracted.values()) + "\n")
    flags = subprocess.check_output(["pkg-config", "--cflags", "--libs", "Qt6Gui"], text=True).split()
    with (root / "build.log").open("w") as log:
        subprocess.run(["c++", "-std=c++17", "-fPIC", str(harness / "reference.cpp"),
                        "-I" + str(root), "-I" + str(harness), *flags, "-o", str(root / "reference")],
                       stdout=log, stderr=log, check=True)
    subprocess.run([str(root / "reference"), str(root / "reference.json")], check=True)
    assert before == {name: digest(path) for name, path in sources.items()}
    values = json.loads((root / "reference.json").read_text())
    index = {"sources": before, "functions": {name: hashlib.sha256(text.encode()).hexdigest() for name, text in extracted.items()},
             "harness": {path.name: digest(path) for path in harness.iterdir() if path.suffix in (".cpp", ".h")},
             "binary": digest(root / "reference"), "reference": digest(root / "reference.json"),
             "counts": {name: len(values[name]) for name in ("angles", "drags", "resize")}, "qt": values["qt"]}
    (root / "index.json").write_text(json.dumps(index, indent=2) + "\n")
    return index


def main():
    """【普通标注旋转对照】【入口】解析只读 original 和新 root，输出来源与样本计数，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--original", type=Path, default=Path.home() / "workspace/mark-shot")
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(collect(options.original.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
