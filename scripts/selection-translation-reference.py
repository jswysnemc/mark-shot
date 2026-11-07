#!/usr/bin/env python3
"""【框选翻译对照】【只读采集】编译原版生产控件与位置模块，固定英文并保存来源指纹"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


def fingerprint(path):
    """【框选翻译对照】【来源指纹】按 path 返回 SHA-256，不修改文件"""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def collect(original, root):
    """【框选翻译对照】【实际采集】按 original 编译生产组件到 root，返回样本和完整来源指纹"""
    root.mkdir(parents=True, exist_ok=False)
    project = Path(__file__).resolve().parents[1]
    source = original / "qt/src"
    fixture = project / "tests/fixtures/qt-selection-translation/main.cpp"
    names = ["selection_translate/selection_translate_placement.cpp", "selection_translate/selection_translate_placement.h",
             "selection_translate/translate_language_popup.cpp", "selection_translate/translate_language_popup.h",
             "selection_translate/translate_result_bar.cpp", "selection_translate/translate_result_bar.h",
             "ui/theme.cpp", "ui/theme.h", "ui/svg_icon.cpp", "ui/svg_icon.h", "ui/toolbar_palette.cpp", "ui/toolbar_palette.h"]
    before = {name: fingerprint(source / name) for name in names}
    generated = []
    for name in ["translate_language_popup", "translate_result_bar"]:
        destination = root / f"moc_{name}.cpp"
        subprocess.run(["/usr/lib/qt6/moc", str(source / f"selection_translate/{name}.h"), "-o", str(destination)], check=True)
        generated.append(destination)
    resource = root / "resources.cpp"
    subprocess.run(["/usr/lib/qt6/rcc", str(original / "qt/resources.qrc"), "-o", str(resource)], check=True)
    flags = subprocess.check_output(["pkg-config", "--cflags", "--libs", "Qt6Widgets", "Qt6Svg"], text=True).split()
    binary = root / "reference"
    with (root / "build.log").open("w") as log:
        subprocess.run(["c++", "-std=c++17", "-fPIC", str(fixture), *(str(source / n) for n in names if n.endswith(".cpp")),
                        *(str(n) for n in generated), str(resource), "-I" + str(source), *flags, "-o", str(binary)], stdout=log, stderr=log, check=True)
    output = root / "reference.json"
    result = subprocess.run([str(binary), str(output)], env=dict(os.environ, QT_QPA_PLATFORM="offscreen"), capture_output=True, text=True, check=True)
    (root / "run.log").write_text(result.stderr)
    if before != {name: fingerprint(source / name) for name in names}:
        raise AssertionError("original selection translation sources changed")
    state = json.loads(output.read_text())
    index = {"sources": before, "harness": fingerprint(fixture), "binary": fingerprint(binary), "reference": fingerprint(output),
             "qt": state["qt"], "font": state["font"], "language": "English", "style": "Fusion", "position_samples": len(state["positions"]), "language_samples": len(state["languages"])}
    (root / "index.json").write_text(json.dumps(index, indent=2) + "\n")
    return index


def main():
    """【框选翻译对照】【执行入口】解析原版和输出目录，打印采集指纹，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--original", type=Path, default=Path.home() / "workspace/mark-shot")
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(collect(options.original.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
