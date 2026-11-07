#!/usr/bin/env python3
"""【图片翻译对照】【只读采集】独立编译原版段落、配色和排版模块，保存完整来源指纹"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


def fingerprint(path):
    """【图片翻译对照】【来源指纹】按 path 返回文件 SHA-256，不修改内容"""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def collect(original, root):
    """【图片翻译对照】【执行采集】按 original 和 root 编译执行只读模块，返回来源和结果索引"""
    root.mkdir(parents=True, exist_ok=False)
    project = Path(__file__).resolve().parents[1]
    fixture = project / "tests/fixtures/qt-image-translation"
    source = original / "qt/src"
    names = ["ocr_result.cpp", "ocr_result.h"]
    for name in ("paragraph_blocks", "block_layout", "block_colors", "translated_block", "translation_exchange"):
        names.extend(f"translation_layout/{name}.{extension}" for extension in ("cpp", "h"))
    before = {name: fingerprint(source / name) for name in names}
    flags = subprocess.check_output(["pkg-config", "--cflags", "--libs", "Qt6Widgets"], text=True).split()
    binary = root / "reference"
    with (root / "build.log").open("w") as log:
        subprocess.run(["c++", "-std=c++17", "-fPIC", str(fixture / "main.cpp"),
                        *(str(source / name) for name in names if name.endswith(".cpp")),
                        "-I" + str(source), *flags, "-o", str(binary)], stdout=log, stderr=log, check=True)
    environment = dict(os.environ, QT_QPA_PLATFORM="offscreen", QT_HASH_SEED="0")
    output = root / "reference.json"
    result = subprocess.run([str(binary), str(fixture / "cases.json"), str(output)],
                            env=environment, capture_output=True, text=True, check=True)
    (root / "run.log").write_text(result.stderr)
    state = json.loads(output.read_text())
    if before != {name: fingerprint(source / name) for name in names}:
        raise AssertionError("original translation sources changed during collection")
    index = {"sources": before, "harness": fingerprint(fixture / "main.cpp"),
             "cases": fingerprint(fixture / "cases.json"), "binary": fingerprint(binary),
             "reference": fingerprint(output), "qt": state["qt"], "font": state["font"],
             "counts": {key: len(state[key]) for key in ("groups", "joins", "colors", "layouts", "exchanges")}}
    (root / "index.json").write_text(json.dumps(index, indent=2) + "\n")
    return index


def main():
    """【图片翻译对照】【命令入口】解析原版目录和独立输出目录，执行采集后输出指纹"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--original", type=Path, default=Path.home() / "workspace/mark-shot")
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(collect(options.original.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
