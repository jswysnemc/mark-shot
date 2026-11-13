"""【原生打包】【媒体完整性】在受控的路径修正和剥离符号后复核媒体库与原始源码摘要"""

import argparse
from pathlib import Path
import re

from artifact_metadata import architecture, checksum, verify_elf


def verify_media(directory, refresh=False, arch=None):
    """【原生打包】【媒体复核】按 directory 与 arch 验证清单；refresh 只允许更新库摘要，返回两项摘要记录"""
    directory = Path(directory)
    manifest = directory / "SHA256SUMS"
    expected = {}
    for row in manifest.read_text().splitlines():
        digest, name = row.split("  ", 1)
        if (
            name not in ("libmarkshot-media.so", "MEDIA-SOURCES.tar.xz")
            or name in expected
            or not re.fullmatch(r"[0-9a-f]{64}", digest)
        ):
            raise ValueError("invalid media checksum manifest")
        expected[name] = digest
    if len(expected) != 2:
        raise ValueError("the media package must include matching sources")
    verify_elf(directory / "libmarkshot-media.so", architecture(arch))
    actual = {name: checksum(directory / name) for name in expected}
    for name in expected:
        if actual[name] != expected[name] and not (
            refresh and name == "libmarkshot-media.so"
        ):
            raise ValueError(f"media checksum mismatch: {name}")
    if refresh:
        manifest.write_text("".join(f"{actual[name]}  {name}\n" for name in expected))
    return actual


def main():
    """【原生打包】【摘要入口】读取媒体目录，验证源码不变后选择是否刷新库摘要，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--refresh", action="store_true")
    options = parser.parse_args()
    verify_media(options.directory, options.refresh)


if __name__ == "__main__":
    main()
