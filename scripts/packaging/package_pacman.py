"""【原生打包】【pacman产物】将完整安装树生成两种架构可安装的二进制包"""

import argparse
import io
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile

from artifact_metadata import architecture, finalize, source_sha, verify_elf, version


DEPENDENCIES = (
    "glibc",
    "fontconfig",
    "libxkbcommon",
    "libxkbcommon-x11",
    "libxcb",
    "openssl",
    "pipewire",
    "libpulse",
    "libva",
    "libdrm",
    "mesa",
)


def package(stage, package_version, arch, commit, destination):
    """【原生打包】【pacman生成】按安装树、版本、架构、提交和输出目录返回原生二进制包清单"""
    stage, destination = map(Path, (stage, destination))
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
    pkgver = package_version.replace("-", "_")
    epoch = int(os.environ.get("SOURCE_DATE_EPOCH", "0"))
    size = sum(path.stat().st_size for path in stage.rglob("*") if path.is_file())
    metadata = (
        f"pkgname = mark-shot-bin\npkgbase = mark-shot-bin\npkgver = {pkgver}-1\n"
        "pkgdesc = Native screenshot, annotation and recording tool\n"
        "url = https://github.com/jswysnemc/mark-shot\n"
        f"builddate = {epoch}\npackager = Mark Shot maintainers <snemc@qq.com>\n"
        f"size = {size}\narch = {arch}\nlicense = MIT\nlicense = Apache-2.0\n"
        f"provides = mark-shot={pkgver}\nconflict = mark-shot\n"
    )
    metadata += "".join(f"depend = {item}\n" for item in DEPENDENCIES)
    metadata += "optdepend = noto-fonts: fallback fonts\noptdepend = noto-fonts-cjk: CJK fonts\n"
    metadata += "optdepend = xdg-desktop-portal: portal capture\n"
    destination.mkdir(parents=True, exist_ok=True)
    asset = destination / f"mark-shot-bin-{pkgver}-1-{arch}.pkg.tar.zst"
    with tempfile.TemporaryDirectory(prefix="mark-shot-pacman-") as temporary:
        archive = Path(temporary) / "package.tar"
        with tarfile.open(archive, "w", format=tarfile.PAX_FORMAT) as output:
            entry = tarfile.TarInfo(".PKGINFO")
            entry.size, entry.mode, entry.mtime = len(metadata.encode()), 0o644, epoch
            output.addfile(entry, io.BytesIO(metadata.encode()))
            for path in sorted(stage.rglob("*")):
                entry = output.gettarinfo(str(path), path.relative_to(stage).as_posix())
                entry.uid = entry.gid = 0
                entry.uname = entry.gname = "root"
                entry.mtime = epoch
                if entry.isfile():
                    with path.open("rb") as source:
                        output.addfile(entry, source)
                else:
                    output.addfile(entry)
        subprocess.run(
            ["zstd", "-q", "-f", "-T2", "-19", str(archive), "-o", str(asset)],
            check=True,
        )
    return finalize(asset, package_version, arch, commit, "pacman")


def main():
    """【原生打包】【pacman入口】读取安装树及构建身份，保存包、摘要和来源清单，无返回值"""
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
