"""【原生源码】【媒体依赖】预先取得并校验构建媒体库所需的全部原始源码"""

import argparse
import hashlib
from pathlib import Path
import subprocess
import sys
import tarfile


SOURCES = (
    (
        "ffmpeg.tar.xz",
        "https://ffmpeg.org/releases/ffmpeg-9.0.tar.xz",
        "7f607a00dd0d28a729d5a4811205812eef01cf6ef6155025febb6f36a9062d52",
    ),
    (
        "openh264.tar.gz",
        "https://codeload.github.com/cisco/openh264/tar.gz/refs/tags/v2.6.0",
        "558544ad358283a7ab2930d69a9ceddf913f4a51ee9bf1bfb9e377322af81a69",
    ),
    (
        "nv-codec-headers.tar.gz",
        "https://github.com/FFmpeg/nv-codec-headers/archive/refs/tags/n13.0.19.0.tar.gz",
        "86d15d1a7c0ac73a0eafdfc57bebfeba7da8264595bf531cf4d8db1c22940116",
    ),
)


def prepare(destination, cache, repository):
    """【原生源码】【离线来源】在 destination 提取固定源码，使用 cache 与 repository 的校验下载器，返回清单路径"""
    destination, cache, repository = map(Path, (destination, cache, repository))
    destination.mkdir(parents=True, exist_ok=False)
    cache.mkdir(parents=True, exist_ok=True)
    for filename, url, digest in SOURCES:
        archive = cache / filename
        if (
            not archive.exists()
            or hashlib.sha256(archive.read_bytes()).hexdigest() != digest
        ):
            subprocess.run(
                [
                    sys.executable,
                    str(repository / "scripts/ci-download.py"),
                    url,
                    str(archive),
                    digest,
                ],
                check=True,
            )
        with tarfile.open(archive) as source:
            members = [
                item
                for item in source.getmembers()
                if not item.name.startswith("openh264-2.6.0/res/")
            ]
            source.extractall(destination, members=members, filter="data")
    checksums = destination / "SOURCE-SHA256SUMS"
    rows = []
    for path in sorted(destination.rglob("*")):
        if path.is_file() and not path.is_symlink():
            with path.open("rb") as stream:
                digest = hashlib.file_digest(stream, "sha256").hexdigest()
            rows.append(f"{digest}  ./{path.relative_to(destination).as_posix()}\n")
    checksums.write_text("".join(rows))
    return checksums


def main():
    """【原生源码】【准备入口】读取目标与缓存目录，保存无编译产物的离线媒体源码，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--destination", type=Path, required=True)
    parser.add_argument("--cache", type=Path, default=Path("target/media-source-cache"))
    parser.add_argument("--root", type=Path, default=Path.cwd())
    options = parser.parse_args()
    print(prepare(options.destination, options.cache, options.root))


if __name__ == "__main__":
    main()
