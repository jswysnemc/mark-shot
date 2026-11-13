"""【原生源码】【Flatpak准备】为离线源码生成可由宿主 AppStream 读取的固定尺寸图标"""

from pathlib import Path
import subprocess

from prepare_source import prepare
from source_tree import archive


def render_icons(source):
    """【原生源码】【图标渲染】按 source 内原始 SVG 生成 128 和 256 像素 PNG，返回输出路径列表"""
    source = Path(source)
    result = []
    for size in (128, 256):
        icon = (
            source
            / f"packaging/desktop/icons/{size}x{size}/io.github.jswysnemc.MarkShot.png"
        )
        icon.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(
            [
                "rsvg-convert",
                "--width",
                str(size),
                "--height",
                str(size),
                str(source / "assets/hicolor/scalable/apps/mark-shot.svg"),
                "--output",
                str(icon),
            ],
            check=True,
        )
        result.append(icon)
    return result


def prepare_flatpak(repository, commit, package_version, destination):
    """【原生源码】【完整归档】按仓库、提交、版本及输出生成带原始 SVG 和 PNG 的离线源码，返回来源记录"""
    source = prepare(repository, commit, package_version, destination, "bundle")
    render_icons(source["source_root"])
    archive(
        source["source_root"],
        source["source_archive"],
        f"mark-shot-{package_version}",
        source["source_epoch"],
    )
    return source
