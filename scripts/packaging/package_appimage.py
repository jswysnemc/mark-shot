"""【原生打包】【AppImage产物】部署原生动态依赖并限制 Debian 12 的 glibc 基线"""

import argparse
import os
from pathlib import Path
import re
import subprocess

from artifact_metadata import architecture, finalize, source_sha, version


def glibc_requirement(directory, maximum=(2, 36)):
    """【原生打包】【glibc校验】扫描 directory 内 ELF 的版本符号，验证 maximum 并返回最高需求"""
    requirements = []
    for path in Path(directory).rglob("*"):
        if not path.is_file() or path.is_symlink():
            continue
        with path.open("rb") as stream:
            if stream.read(4) != b"\x7fELF":
                continue
        symbols = subprocess.check_output(
            ["readelf", "--version-info", str(path)], text=True
        )
        requirements.extend(
            tuple(map(int, item.split(".")))
            for item in re.findall(r"GLIBC_([0-9]+\.[0-9]+(?:\.[0-9]+)?)", symbols)
        )
    if not requirements or max(requirements) > maximum:
        raise ValueError(
            f"AppImage glibc requirement exceeds {maximum}: {max(requirements, default=None)}"
        )
    return max(requirements)


def package(repository, stage, package_version, commit, destination):
    """【原生打包】【AppImage生成】按源码、安装树、版本、提交及输出返回完成原生依赖校验的产物清单"""
    repository, stage, destination = (
        Path(repository).resolve(),
        Path(stage).resolve(),
        Path(destination).resolve(),
    )
    if architecture() != "x86_64":
        raise ValueError("the migrated AppImage target is x86_64 only")
    version(package_version)
    source_sha(commit)
    private = stage / "usr/lib/mark-shot"
    if (private / "SOURCE-COMMIT").read_text().strip() != commit:
        raise ValueError("staged source commit mismatch")
    tools = repository / "target/packaging-tools"
    tools.mkdir(parents=True, exist_ok=True)
    tool = tools / "linuxdeploy-x86_64.AppImage"
    subprocess.run(
        [
            "python3",
            str(repository / "scripts/ci-download.py"),
            "https://github.com/linuxdeploy/linuxdeploy/releases/download/1-alpha-20251107-1/linuxdeploy-x86_64.AppImage",
            str(tool),
            "c20cd71e3a4e3b80c3483cef793cda3f4e990aca14014d23c544ca3ce1270b4d",
        ],
        check=True,
    )
    tool.chmod(0o755)
    destination.mkdir(parents=True, exist_ok=True)
    asset = destination / f"mark-shot-native-{package_version}-linux-x86_64.AppImage"
    environment = {
        **os.environ,
        "APPIMAGE_EXTRACT_AND_RUN": "1",
        "NO_STRIP": "1",
        "VERSION": package_version,
        "OUTPUT": str(asset),
    }
    subprocess.run(
        [
            str(tool),
            "--appdir",
            str(stage),
            "--deploy-deps-only",
            str(private),
            "--deploy-deps-only",
            str(private / "media"),
            "--desktop-file",
            str(stage / "usr/share/applications/io.github.jswysnemc.MarkShot.desktop"),
            "--icon-file",
            str(
                stage
                / "usr/share/icons/hicolor/scalable/apps/io.github.jswysnemc.MarkShot.svg"
            ),
            "--custom-apprun",
            str(repository / "packaging/appimage/AppRun"),
            "--output",
            "appimage",
        ],
        cwd=destination,
        env=environment,
        check=True,
    )
    glibc_requirement(stage)
    if not asset.is_file():
        raise FileNotFoundError("linuxdeploy did not create the requested AppImage")
    subprocess.run([str(asset), "--version"], env=environment, check=True)
    return finalize(asset, package_version, "x86_64", commit, "appimage")


def main():
    """【原生打包】【AppImage入口】读取源码身份与安装树，生成可验证的 AppImage，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--stage", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--output", type=Path, default=Path("target/packages"))
    options = parser.parse_args()
    package(
        options.root, options.stage, options.version, options.source_sha, options.output
    )


if __name__ == "__main__":
    main()
