"""【原生源码】【发行准备】从固定提交准备离线构建树及 Debian 多组件源码包"""

import argparse
from datetime import datetime, timezone
from email.utils import format_datetime
from pathlib import Path
import re
import shutil
import subprocess
import sys

from artifact_metadata import output, source_sha, version
from source_tree import archive, checkout


def prepare(repository, commit, package_version, destination, kind, revision="1"):
    """【原生源码】【完整交付】按仓库、提交、版本、输出、格式及修订号准备源码，返回工作流记录"""
    repository, destination = Path(repository).resolve(), Path(destination).resolve()
    commit, package_version = source_sha(commit), version(package_version)
    if not re.fullmatch(r"(?:[1-9][0-9]*|0ppa[1-9][0-9]*~resolute1)", revision):
        raise ValueError("invalid Debian revision")
    epoch = int(
        subprocess.check_output(
            ["git", "-C", str(repository), "show", "-s", "--format=%ct", commit]
        )
    )
    upstream = package_version.replace("-", "~", 1)
    name = f"mark-shot-{upstream if kind == 'debian' else package_version}"
    source = checkout(repository, commit, destination / name, identity=False)
    # 1. 【原生源码】【上游归档】先归档原始提交，依赖组件及 Debian 变更不混入上游包
    if kind == "debian":
        archive(source, destination / f"mark-shot_{upstream}.orig.tar.gz", name, epoch)
    subprocess.run(
        ["bash", str(repository / "scripts/packaging/vendor_sources.sh"), str(source)],
        check=True,
        stdout=sys.stderr,
    )
    # 2. 【原生源码】【依赖组件】Debian 组件保留各自顶层目录，源码包解开后可直接离线编译
    if kind == "debian":
        archive(
            source / "vendor",
            destination / f"mark-shot_{upstream}.orig-vendor.tar.xz",
            "vendor",
            epoch,
        )
        archive(
            source / "media",
            destination / f"mark-shot_{upstream}.orig-media.tar.xz",
            "media",
            epoch,
        )
        # 3. 【原生源码】【测试字体】组件名必须匹配解包目录，保留离线对照测试的固定输入
        archive(
            source / "reference-fonts",
            destination / f"mark-shot_{upstream}.orig-reference-fonts.tar.xz",
            "reference-fonts",
            epoch,
        )
        shutil.move(
            source / "packaging/licenses", source / "debian/dependency-licenses"
        )
        (source / "debian/source-commit").write_text(commit + "\n")
        distribution = "resolute" if revision.startswith("0ppa") else "unstable"
        stamp = format_datetime(datetime.fromtimestamp(epoch, timezone.utc))
        (source / "debian/changelog").write_text(
            f"mark-shot ({upstream}-{revision}) {distribution}; urgency=medium\n\n"
            "  * Package the native Rust application and its offline build sources.\n\n"
            f" -- Mark Shot maintainers <snemc@qq.com>  {stamp}\n"
        )
        return {
            "source_root": source,
            "debian_version": f"{upstream}-{revision}",
            "source_epoch": epoch,
        }
    (source / "SOURCE-COMMIT").write_text(commit + "\n")
    asset = archive(source, destination / f"{name}-source.tar.xz", name, epoch)
    return {"source_root": source, "source_archive": asset, "source_epoch": epoch}


def main():
    """【原生源码】【准备入口】读取源码身份与目标格式，输出完整离线源码包的位置，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--format", choices=("debian", "bundle"), default="bundle")
    parser.add_argument("--revision", default="1")
    options = parser.parse_args()
    output(
        prepare(
            options.root,
            options.source_sha,
            options.version,
            options.output,
            options.format,
            options.revision,
        )
    )


if __name__ == "__main__":
    main()
