"""【原生打包】【Linux便携包】封装完整原生安装树，保留源码标识和媒体库来源"""

import argparse
import os
from pathlib import Path
import shutil

from artifact_metadata import architecture, finalize, source_sha, verify_elf, version
from source_tree import archive


def package(stage, package_version, arch, commit, destination):
    """【原生打包】【便携生成】按安装树、版本、架构、提交及输出返回带摘要的便携包清单"""
    stage, destination = Path(stage), Path(destination)
    arch, package_version, commit = (
        architecture(arch),
        version(package_version),
        source_sha(commit),
    )
    private = stage / "usr/lib/mark-shot"
    verify_elf(private / "mark-shot", arch)
    verify_elf(private / "media/libmarkshot-media.so", arch)
    if (private / "SOURCE-COMMIT").read_text().strip() != commit:
        raise ValueError("staged source commit mismatch")
    launch = stage / "mark-shot"
    launch.write_text(
        '#!/bin/sh\nbase=$(CDPATH=\'\' cd -- "$(dirname -- "$0")" && pwd)\nexec "$base/usr/lib/mark-shot/mark-shot" "$@"\n'
    )
    launch.chmod(0o755)
    shutil.copy2(private / "SOURCE-COMMIT", stage / "SOURCE-COMMIT")
    destination.mkdir(parents=True, exist_ok=True)
    name = f"mark-shot-native-{package_version}-linux-{arch}"
    asset = archive(
        stage,
        destination / f"{name}.tar.gz",
        name,
        int(os.environ.get("SOURCE_DATE_EPOCH", "0")),
    )
    return finalize(asset, package_version, arch, commit, "portable")


def main():
    """【原生打包】【便携入口】读取完整安装树和来源参数，输出便携包及摘要，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--stage", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--arch", default=architecture())
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--output", type=Path, default=Path("target/packages"))
    options = parser.parse_args()
    package(
        options.stage, options.version, options.arch, options.source_sha, options.output
    )


if __name__ == "__main__":
    main()
