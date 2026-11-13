"""【原生打包】【安装布局】为发行包与沙箱创建不依赖 Qt 的完整安装树"""

import argparse
from pathlib import Path
import shutil

from artifact_metadata import architecture, checksum, source_sha, verify_elf


def install(root, binary, media, destination, prefix, arch, commit, licenses=None):
    """【原生打包】【完整安装】按源码、程序、媒体库、目标、前缀、架构、提交和版权目录返回安装根目录"""
    root, binary, media, destination = map(Path, (root, binary, media, destination))
    arch = architecture(arch)
    source_sha(commit)
    prefix = Path(prefix)
    if not prefix.is_absolute() or ".." in prefix.parts:
        raise ValueError("the installation prefix must be an absolute normalized path")
    verify_elf(binary, arch)
    verify_elf(media / "libmarkshot-media.so", arch)
    if not licenses or not (Path(licenses) / "DEPENDENCIES.txt").is_file():
        raise ValueError("complete packages require dependency license notices")
    expected = {}
    for row in (media / "SHA256SUMS").read_text().splitlines():
        digest, name = row.split("  ", 1)
        if (
            name not in ("libmarkshot-media.so", "MEDIA-SOURCES.tar.xz")
            or name in expected
        ):
            raise ValueError("invalid media checksum manifest")
        if checksum(media / name) != digest:
            raise ValueError(f"media checksum mismatch: {name}")
        expected[name] = digest
    if len(expected) != 2:
        raise ValueError("the media package must include matching sources")
    installed = destination / prefix.relative_to("/")
    private = installed / "lib/mark-shot"
    private.mkdir(parents=True, exist_ok=True)
    shutil.copy2(binary, private / "mark-shot")
    (private / "mark-shot").chmod(0o755)
    shutil.copytree(media, private / "media", dirs_exist_ok=True)
    (private / "media/libmarkshot-media.so").chmod(0o644)
    (private / "SOURCE-COMMIT").write_text(commit + "\n")
    commands = installed / "bin"
    commands.mkdir(parents=True, exist_ok=True)
    command = commands / "mark-shot"
    if command.exists() or command.is_symlink():
        command.unlink()
    command.symlink_to("../lib/mark-shot/mark-shot")
    for source, relative in (
        (
            root / "packaging/desktop/io.github.jswysnemc.MarkShot.desktop",
            "share/applications/io.github.jswysnemc.MarkShot.desktop",
        ),
        (
            root / "packaging/desktop/io.github.jswysnemc.MarkShot.metainfo.xml",
            "share/metainfo/io.github.jswysnemc.MarkShot.metainfo.xml",
        ),
        (
            root / "assets/hicolor/scalable/apps/mark-shot.svg",
            "share/icons/hicolor/scalable/apps/io.github.jswysnemc.MarkShot.svg",
        ),
        (root / "packaging/desktop/mark-shot.1", "share/man/man1/mark-shot.1"),
    ):
        target = installed / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
        target.chmod(0o644)
    notices = installed / "share/licenses/mark-shot"
    notices.mkdir(parents=True, exist_ok=True)
    for source in (
        root / "LICENSE-MIT",
        root / "LICENSE-APACHE",
        root / "assets/fonts/LICENSE-OFL.txt",
        root / "assets/fonts/LICENSE-NUMERALS.txt",
    ):
        shutil.copy2(source, notices / source.name)
    shutil.copy2(root / "assets/icons/mark-shot/LICENSE", notices / "LICENSE-ICONS.txt")
    if licenses:
        shutil.copytree(licenses, notices, dirs_exist_ok=True)
    documentation = installed / "share/doc/mark-shot"
    documentation.mkdir(parents=True, exist_ok=True)
    shutil.copy2(root / "README.md", documentation / "README.md")
    shutil.copytree(root / "docs", documentation / "docs", dirs_exist_ok=True)
    shutil.copy2(root / "Cargo.lock", documentation / "Cargo.lock")
    return installed


def main():
    """【原生打包】【安装入口】读取安装目录与构建标识，生成完整安装树并打印目录，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--media", type=Path, default=Path("target/media-linux"))
    parser.add_argument("--destination", type=Path, required=True)
    parser.add_argument("--prefix", default="/usr")
    parser.add_argument("--arch", default=architecture())
    parser.add_argument("--source-sha", required=True)
    parser.add_argument(
        "--licenses", type=Path, default=Path("target/packaging-licenses")
    )
    options = parser.parse_args()
    licenses = options.licenses if options.licenses.is_dir() else None
    print(
        install(
            options.root,
            options.binary,
            options.media,
            options.destination,
            options.prefix,
            options.arch,
            options.source_sha,
            licenses,
        )
    )


if __name__ == "__main__":
    main()
