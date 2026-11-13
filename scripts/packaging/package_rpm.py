"""【原生打包】【RPM源码构建】从完整离线源码构建 Fedora 软件包与可重建源码 RPM"""

import argparse
from pathlib import Path
import shutil
import subprocess

from artifact_metadata import architecture, finalize, source_sha, version
from prepare_source import prepare


def package(repository, package_version, arch, commit, destination):
    """【原生打包】【RPM生成】按仓库、版本、架构、提交及输出准备源码构建，返回二进制产物路径"""
    repository, destination = Path(repository).resolve(), Path(destination).resolve()
    package_version, commit, arch = (
        version(package_version),
        source_sha(commit),
        architecture(arch),
    )
    if architecture() != arch:
        raise ValueError(
            "RPM builds must use a native runner of the requested architecture"
        )
    top = destination / "rpmbuild"
    for directory in ("BUILD", "BUILDROOT", "RPMS", "SOURCES", "SPECS", "SRPMS"):
        (top / directory).mkdir(parents=True, exist_ok=False)
    source = prepare(
        repository, commit, package_version, destination / "source", "bundle"
    )
    shutil.copy2(source["source_archive"], top / "SOURCES")
    rpm_version = package_version.replace("-", "~", 1).replace("+", ".")
    spec = (repository / "packaging/rpm/mark-shot.spec.in").read_text()
    spec = spec.replace("@rpm_version@", rpm_version).replace(
        "@upstream@", package_version
    )
    specification = top / "SPECS/mark-shot.spec"
    specification.write_text(spec)
    subprocess.run(
        [
            "rpmbuild",
            "-ba",
            "--define",
            f"_topdir {top}",
            "--define",
            "debug_package %{nil}",
            str(specification),
        ],
        check=True,
    )
    binaries = list((top / "RPMS").rglob(f"mark-shot-*.{arch}.rpm"))
    sources = list((top / "SRPMS").glob("mark-shot-*.src.rpm"))
    if len(binaries) != 1 or len(sources) != 1:
        raise ValueError("RPM build must produce one binary and one source package")
    for item, kind in ((binaries[0], "rpm"), (sources[0], "srpm")):
        name = (
            item.name
            if kind == "rpm"
            else f"mark-shot_{package_version}_fedora_{arch}.src.rpm"
        )
        asset = destination / name
        shutil.copy2(item, asset)
        finalize(asset, package_version, arch, commit, kind)
    return destination / binaries[0].name


def main():
    """【原生打包】【RPM入口】读取固定源码与架构，输出二进制和源码包及摘要，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--version", required=True)
    parser.add_argument("--arch", default=architecture())
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--output", type=Path, default=Path("target/packages"))
    options = parser.parse_args()
    package(
        options.root, options.version, options.arch, options.source_sha, options.output
    )


if __name__ == "__main__":
    main()
