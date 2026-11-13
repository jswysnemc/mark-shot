"""【原生源码】【产物收集】校验 Debian 描述文件引用的完整源码集合，避免容器路径映射产生残缺交付"""

import argparse
from pathlib import Path
import re
import shutil

from artifact_metadata import checksum


def referenced_files(manifest):
    """【原生源码】【清单解析】从 manifest 的 SHA-256 字段校验大小和摘要，返回引用文件，不验证 GPG 签名"""
    manifest = Path(manifest)
    match = re.search(
        r"^Checksums-Sha256:\s*\n((?:[ \t]+[^\n]+\n)+)",
        manifest.read_text(),
        re.MULTILINE,
    )
    if not match:
        raise ValueError(f"source manifest has no SHA-256 inventory: {manifest}")
    files = []
    for line in match[1].splitlines():
        entry = re.fullmatch(r"\s+([a-f0-9]{64}) ([0-9]+) (\S+)", line)
        if not entry:
            raise ValueError(f"invalid source checksum entry: {line}")
        digest, size, name = entry.groups()
        if Path(name).name != name or not name.startswith("mark-shot_"):
            raise ValueError(f"invalid source artifact filename: {name}")
        path = manifest.parent / name
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"referenced source artifact is missing: {path}")
        if path.stat().st_size != int(size) or checksum(path) != digest:
            raise ValueError(f"source artifact size or checksum mismatch: {path}")
        files.append(path)
    return files


def collect(source, destination):
    """【原生源码】【完整收集】按 source 校验唯一 dsc 和 changes 的全部引用，再复制至 destination，返回产物列表"""
    source, destination = Path(source), Path(destination)
    descriptors = list(source.glob("mark-shot_*.dsc"))
    changes = list(source.glob("mark-shot_*.changes"))
    if len(descriptors) != 1 or len(changes) != 1:
        raise ValueError("source collection requires one dsc and one changes file")
    # 1. 【原生源码】【完整性校验】全部引用通过校验后才写入输出，不把缺少组件的描述文件视为完整产物
    files = {
        path.name: path
        for path in (
            *descriptors,
            *changes,
            *referenced_files(descriptors[0]),
            *referenced_files(changes[0]),
        )
    }
    destination.mkdir(parents=True, exist_ok=True)
    for name, path in files.items():
        shutil.copy2(path, destination / name)
    return sorted(destination / name for name in files)


def main():
    """【原生源码】【收集入口】读取输入和输出目录，打印完整产物集合，缺失或篡改时失败，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--destination", type=Path, required=True)
    options = parser.parse_args()
    for path in collect(options.source, options.destination):
        print(path)


if __name__ == "__main__":
    main()
