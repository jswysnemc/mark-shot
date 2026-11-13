"""【原生打包】【Flatpak产物】使用 Freedesktop SDK 和完整离线源码构建沙箱软件包"""

import argparse
from pathlib import Path
import subprocess

from artifact_metadata import finalize, source_sha, version
from prepare_flatpak_source import prepare_flatpak


def package(repository, package_version, commit, destination):
    """【原生打包】【Flatpak生成】按仓库、版本、提交及输出准备离线源码，返回完整产物清单"""
    repository, destination = Path(repository).resolve(), Path(destination).resolve()
    package_version, commit = version(package_version), source_sha(commit)
    source = prepare_flatpak(
        repository, commit, package_version, destination / "source"
    )
    template = (
        repository / "packaging/flatpak/io.github.jswysnemc.MarkShot.yml"
    ).read_text()
    template = template.replace(
        "path: mark-shot-source.tar.xz", f"path: {source['source_archive']}"
    )
    manifest = destination / "io.github.jswysnemc.MarkShot.yml"
    manifest.write_text(template)
    repo = destination / "flatpak-repo"
    subprocess.run(
        [
            "flatpak-builder",
            "--user",
            "--install-deps-from=flathub",
            "--force-clean",
            f"--repo={repo}",
            str(destination / "flatpak-build"),
            str(manifest),
        ],
        check=True,
    )
    asset = destination / f"mark-shot-native-{package_version}-linux-x86_64.flatpak"
    subprocess.run(
        [
            "flatpak",
            "build-bundle",
            str(repo),
            str(asset),
            "io.github.jswysnemc.MarkShot",
            "stable",
        ],
        check=True,
    )
    subprocess.run(
        ["flatpak", "install", "--user", "--noninteractive", str(asset)], check=True
    )
    subprocess.run(
        ["flatpak", "run", "io.github.jswysnemc.MarkShot", "--version"], check=True
    )
    subprocess.run(
        [
            "flatpak",
            "run",
            "--command=sh",
            "io.github.jswysnemc.MarkShot",
            "-c",
            "test -f /app/lib/mark-shot/media/libmarkshot-media.so && "
            "test -f /app/lib/mark-shot/media/MEDIA-SOURCES.tar.xz && "
            "test -f /app/share/licenses/mark-shot/DEPENDENCIES.txt && "
            "cd /app/lib/mark-shot/media && sha256sum --check SHA256SUMS",
        ],
        check=True,
    )
    return finalize(asset, package_version, "x86_64", commit, "flatpak")


def main():
    """【原生打包】【Flatpak入口】读取固定源码与版本，构建并运行最终沙箱包，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--version", required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--output", type=Path, default=Path("target/packages"))
    options = parser.parse_args()
    package(options.root, options.version, options.source_sha, options.output)


if __name__ == "__main__":
    main()
