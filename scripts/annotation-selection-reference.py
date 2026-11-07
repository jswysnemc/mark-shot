#!/usr/bin/env python3
"""【标注选框对照】【只读采集】提取原版完整生产绘制函数，在新目录编译并记录图像及源码指纹"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess


def digest(path):
    """【标注选框对照】【文件指纹】按 path 返回 SHA-256，不修改来源文件"""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def function(source, name):
    """【标注选框对照】【完整函数】按 source 与 name 提取原始 C++ 成员定义，返回含完整函数体的源码，不改写算法"""
    match = re.search(r"^.*\bShotWindow::" + re.escape(name) + r"\([^\n]*\) const\s*\{", source, re.M)
    if not match:
        raise AssertionError(f"production function was not found: {name}")
    start, opened = match.start(), source.index("{", match.start())
    depth = 0
    for offset in range(opened, len(source)):
        depth += (source[offset] == "{") - (source[offset] == "}")
        if depth == 0:
            return source[start:offset + 1]
    raise AssertionError(f"unclosed production function: {name}")


def collect(original, root):
    """【标注选框对照】【实际采集】在 root 编译 original 的选框、按钮和旋转函数，返回完整来源及72张图像指纹"""
    root.mkdir(parents=True, exist_ok=False)
    project = Path(__file__).resolve().parents[1]
    harness = project / "tests/fixtures/qt-annotation-selection"
    sources = {name: original / "qt/src" / name for name in ("shot_window_selection_frame.cpp", "shot_window_hit_testing.cpp")}
    before = {name: digest(path) for name, path in sources.items()}
    selected = {"shot_window_selection_frame.cpp": ["drawSelectedAnnotationFrame", "selectedAnnotationDeleteButtonRect", "selectionHandlePoints"],
                "shot_window_hit_testing.cpp": ["annotationSizeScale", "annotationSupportsRotation", "annotationRotationCenter", "rotatedPoint", "annotationRotationHandlePoint", "selectionRotationHandlePoint"]}
    extracted = {name: function(sources[file].read_text(), name) for file, names in selected.items() for name in names}
    (root / "production.inc").write_text("\n\n".join(extracted.values()) + "\n")
    flags = subprocess.check_output(["pkg-config", "--cflags", "--libs", "Qt6Widgets"], text=True).split()
    with (root / "build.log").open("w") as log:
        subprocess.run(["c++", "-std=c++17", "-fPIC", str(harness / "reference.cpp"), "-I" + str(root), "-I" + str(harness), *flags, "-o", str(root / "reference")], stdout=log, stderr=log, check=True)
    subprocess.run([str(root / "reference"), str(root)], env=dict(os.environ, QT_QPA_PLATFORM="offscreen"), check=True, capture_output=True)
    assert before == {name: digest(path) for name, path in sources.items()}
    index = {"sources": before, "functions": {name: hashlib.sha256(text.encode()).hexdigest() for name, text in extracted.items()},
             "harness": {path.name: digest(path) for path in harness.iterdir() if path.suffix in (".cpp", ".h")},
             "binary": digest(root / "reference"), "reference": digest(root / "reference.json"),
             "renders": {path.name: digest(path) for path in sorted((root / "render").glob("*.png"))}}
    (root / "index.json").write_text(json.dumps(index, indent=2) + "\n")
    return index


def main():
    """【标注选框对照】【入口】解析只读原版及本次新目录，输出采集指纹，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--original", type=Path, default=Path.home() / "workspace/mark-shot")
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(collect(options.original.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
