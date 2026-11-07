#!/usr/bin/env python3
"""【外框对照】【原版采集】只读编译原版 Qt 导出适配器和 Rust 核心，保存实际 PNG 与源码指纹"""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def fingerprint(path):
    """【外框对照】【文件指纹】读取 path 的全部内容并返回 SHA-256，不修改参考文件"""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def collect(original, root):
    """【外框对照】【实际构建】从 original 编译生产导出函数到 root，返回采集索引；全部构建产物只写入 root"""
    root.mkdir(parents=True, exist_ok=False)
    project = Path(__file__).resolve().parents[1]
    harness = project / "tests/fixtures/qt-export-image-reference/main.cpp"
    sources = [original / "qt/src/export_image_effect.cpp", original / "qt/src/export_image_effect.h"]
    sources.extend(sorted((original / "qt/bridge").glob("*.h")))
    sources.extend(sorted((original / "crates/shot-image/src").rglob("*.rs")))
    sources.extend(sorted((original / "crates/shot-config/src").rglob("*.rs")))
    sources.extend(sorted((original / "crates/shot-ffi/src").rglob("*.rs")))
    before = {str(path.relative_to(original)): fingerprint(path) for path in sources}
    environment = {"QT_QPA_PLATFORM": "offscreen"}
    # 1. 【外框对照】【私有编译】不使用原版旧构建对象，静态核心由当前源码重新构建
    with (root / "build.log").open("w") as log:
        subprocess.run(["cargo", "build", "--manifest-path", str(original / "Cargo.toml"),
                        "--locked", "--release", "-p", "shot-ffi", "--target-dir", str(root / "core")],
                       stdout=log, stderr=log, check=True)
        flags = subprocess.check_output(["pkg-config", "--cflags", "--libs", "Qt6Gui", "Qt6Core"], text=True).split()
        binary = root / "reference"
        subprocess.run(["c++", "-std=c++17", "-fPIC", str(harness), str(sources[0]),
                        "-I" + str(original / "qt/src"), "-I" + str(original / "qt/bridge"),
                        str(root / "core/release/libshot_ffi.a"), *flags, "-ldl", "-lpthread", "-lm", "-o", str(binary)],
                       stdout=log, stderr=log, check=True)
    # 2. 【外框对照】【实际输出】只调用原版函数，期望像素不从当前 Rust 实现计算
    import os
    result = subprocess.run([str(binary), str(root / "images")], env=dict(os.environ, **environment),
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=True)
    (root / "reference.log").write_text(result.stderr)
    cases = json.loads((root / "images/cases.json").read_text())
    if len(cases) != 36 or result.stdout.strip() != "36":
        raise AssertionError("original export matrix was incomplete")
    after = {str(path.relative_to(original)): fingerprint(path) for path in sources}
    if after != before:
        raise AssertionError("the reference source changed during compilation")
    index = {"original": str(original), "qt_version": subprocess.check_output(["pkg-config", "--modversion", "Qt6Gui"], text=True).strip(),
             "source_sha256": before, "harness_sha256": fingerprint(harness), "binary_sha256": fingerprint(binary),
             "cases": len(cases), "png_sha256": {path.name: fingerprint(path) for path in sorted((root / "images").glob("*.png"))}}
    (root / "reference.json").write_text(json.dumps(index, ensure_ascii=False, indent=2) + "\n")
    return index


def main():
    """【外框对照】【命令入口】解析 original、root 并采集当前原版，返回完整构建结果"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--original", type=Path, default=Path.home() / "workspace/mark-shot")
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    result = collect(options.original.resolve(), options.root.resolve())
    print(json.dumps({"cases": result["cases"], "qt_version": result["qt_version"], "root": str(options.root)}, indent=2))


if __name__ == "__main__":
    main()
