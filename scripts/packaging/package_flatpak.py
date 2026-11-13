"""【原生打包】【Flatpak产物】使用 Freedesktop SDK 和完整离线源码构建沙箱软件包"""

import argparse
from pathlib import Path
import shutil
import subprocess
import tempfile

from artifact_metadata import finalize, source_sha, version
from prepare_flatpak_source import prepare_flatpak


def package(repository, package_version, commit, destination):
    """【原生打包】【Flatpak生成】按仓库、版本、提交及输出准备离线源码，返回完整产物清单"""
    repository, destination = Path(repository).resolve(), Path(destination).resolve()
    package_version, commit = version(package_version), source_sha(commit)
    destination.mkdir(parents=True, exist_ok=True)
    # 1. 【原生打包】【工作隔离】本轮源码、构建树及仓库不复用缓存或重试留下的目录
    with tempfile.TemporaryDirectory(
        prefix=".flatpak-build-", dir=destination
    ) as temporary:
        return _build(repository, package_version, commit, destination, Path(temporary))


def _build(repository, package_version, commit, destination, workspace):
    """【原生打包】【独立沙箱构建】按仓库、版本及提交在 workspace 构建，验证后复制至 destination，返回清单"""
    source = prepare_flatpak(repository, commit, package_version, workspace / "source")
    template = (
        repository / "packaging/flatpak/io.github.jswysnemc.MarkShot.yml"
    ).read_text()
    template = template.replace(
        "path: mark-shot-source.tar.xz", f"path: {source['source_archive']}"
    )
    manifest = workspace / "io.github.jswysnemc.MarkShot.yml"
    manifest.write_text(template)
    repo = workspace / "flatpak-repo"
    subprocess.run(
        [
            "flatpak-builder",
            "--user",
            "--install-deps-from=flathub",
            "--force-clean",
            f"--repo={repo}",
            str(workspace / "flatpak-build"),
            str(manifest),
        ],
        check=True,
    )
    asset = workspace / f"mark-shot-native-{package_version}-linux-x86_64.flatpak"
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
    # 2. 【原生打包】【验证交付】只复制安装和沙箱验证均通过的最终包，不删除其他输出
    result = destination / asset.name
    shutil.copy2(asset, result)
    return finalize(result, package_version, "x86_64", commit, "flatpak")


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
