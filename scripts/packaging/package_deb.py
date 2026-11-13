"""【原生打包】【DEB产物】在目标发行版生成真实动态依赖与可安装的 Debian 软件包"""

import argparse
import gzip
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from artifact_metadata import (
    ARCHITECTURES,
    architecture,
    finalize,
    source_sha,
    verify_elf,
    version,
)


SUFFIXES = {
    "debian12": "",
    "debian13": ".debian13",
    "ubuntu2404": ".ubuntu24.04",
    "ubuntu2604": ".ubuntu26.04",
}


def dependencies(binary, library):
    """【原生打包】【DEB依赖】扫描 binary 与 library，返回目标系统提供的精确运行依赖，缺失时直接失败"""
    with tempfile.TemporaryDirectory(prefix="mark-shot-shlibs-") as directory:
        control = Path(directory) / "debian/control"
        control.parent.mkdir()
        control.write_text(
            "Source: mark-shot\n\nPackage: mark-shot\nArchitecture: any\nDescription: native screenshot tool\n"
        )
        result = subprocess.check_output(
            [
                "dpkg-shlibdeps",
                "-O",
                f"-l{Path(library).parent}",
                f"-e{binary}",
                f"-e{library}",
            ],
            cwd=directory,
            text=True,
        )
    row = next(
        (line for line in result.splitlines() if line.startswith("shlibs:Depends=")),
        None,
    )
    if row is None:
        raise ValueError("dpkg-shlibdeps returned no runtime dependencies")
    return row.split("=", 1)[1] + ", libpulse0"


def package(stage, package_version, arch, target, commit, destination, repository):
    """【原生打包】【DEB生成】按安装树、版本、架构、发行版、提交、输出和源码目录返回经过校验的产物清单"""
    stage, destination, repository = map(Path, (stage, destination, repository))
    stage = stage.resolve()
    arch, package_version, commit = (
        architecture(arch),
        version(package_version),
        source_sha(commit),
    )
    private = stage / "usr/lib/mark-shot"
    binary, library = private / "mark-shot", private / "media/libmarkshot-media.so"
    verify_elf(binary, arch)
    verify_elf(library, arch)
    if (private / "SOURCE-COMMIT").read_text().strip() != commit:
        raise ValueError(
            "the staged source commit does not match the requested package"
        )
    runtime = dependencies(binary, library)
    deb_version = package_version.replace("-", "~", 1) + "-1"
    control = stage / "DEBIAN"
    control.mkdir(exist_ok=True)
    size = (
        sum(path.stat().st_size for path in stage.rglob("*") if path.is_file()) // 1024
    )
    (control / "control").write_text(
        f"Package: mark-shot\nVersion: {deb_version}\nArchitecture: {ARCHITECTURES[arch]['deb']}\n"
        "Section: graphics\nPriority: optional\nMaintainer: Mark Shot maintainers <snemc@qq.com>\n"
        f"Installed-Size: {size}\nDepends: {runtime}\n"
        "Recommends: pipewire, xdg-desktop-portal, fonts-noto-core, fonts-noto-cjk\n"
        "Homepage: https://github.com/jswysnemc/mark-shot\n"
        "Description: native screenshot, annotation and recording tool\n"
        " Native Wayland and X11 capture, scrolling screenshots, local OCR,\n"
        " translation, QR and barcode recognition, and screen recording.\n"
    )
    documentation = stage / "usr/share/doc/mark-shot"
    shutil.copy2(repository / "debian/copyright", documentation / "copyright")
    man = stage / "usr/share/man/man1/mark-shot.1"
    Path(str(man) + ".gz").write_bytes(gzip.compress(man.read_bytes(), mtime=0))
    man.unlink()
    destination.mkdir(parents=True, exist_ok=True)
    asset = (
        destination
        / f"mark-shot_{deb_version}_{ARCHITECTURES[arch]['deb']}{SUFFIXES[target]}.deb"
    )
    subprocess.run(
        ["dpkg-deb", "--root-owner-group", "-Zxz", "--build", str(stage), str(asset)],
        check=True,
    )
    actual = subprocess.check_output(
        ["dpkg-deb", "--field", str(asset), "Architecture"], text=True
    ).strip()
    if actual != ARCHITECTURES[arch]["deb"]:
        raise ValueError("generated DEB architecture mismatch")
    return finalize(asset, package_version, arch, commit, "deb")


def main():
    """【原生打包】【DEB入口】读取发行版和安装树，构建并输出 DEB 文件与摘要，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--stage", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--arch", default=architecture())
    parser.add_argument("--target", choices=SUFFIXES, required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--output", type=Path, default=Path("target/packages"))
    parser.add_argument("--root", type=Path, default=Path.cwd())
    options = parser.parse_args()
    os.environ.setdefault("SOURCE_DATE_EPOCH", "0")
    package(
        options.stage,
        options.version,
        options.arch,
        options.target,
        options.source_sha,
        options.output,
        options.root,
    )


if __name__ == "__main__":
    main()
