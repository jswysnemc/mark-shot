"""【原生发布】【AUR模板】生成源码或二进制包的 PKGBUILD，拒绝未经验证的发布资产"""

import argparse
from pathlib import Path
import re

from artifact_metadata import checksum
from release_context import resolve
from publish_assets import assets


def prepare(
    repository, context, destination, binary=False, asset_directory=None, revision="1"
):
    """【原生发布】【AUR准备】按仓库、上下文、输出、包类型、资产目录和修订号生成 PKGBUILD，返回文件"""
    repository, destination = Path(repository), Path(destination)
    if not re.fullmatch(r"[1-9][0-9]*", revision):
        raise ValueError("pkgrel must be a positive integer")
    if not re.fullmatch(r"native-v[0-9]+\.[0-9]+\.[0-9]+", context.tag):
        raise ValueError("AUR publication requires a stable native version")
    template = (
        repository / "packaging/aur" / ("PKGBUILD-bin.in" if binary else "PKGBUILD.in")
    )
    replacements = {
        "pkgver": context.version,
        "pkgrel": revision,
        "commit": context.commit,
        "tag": context.tag,
    }
    if binary:
        assets(asset_directory, context)
        for arch in ("x86_64", "aarch64"):
            asset = (
                Path(asset_directory)
                / f"mark-shot-bin-{context.version}-1-{arch}.pkg.tar.zst"
            )
            replacements[f"sha_{arch}"] = checksum(asset)
    text = template.read_text()
    for key, value in replacements.items():
        text = text.replace(f"@{key}@", value)
    if re.search(r"@[a-z0-9_]+@", text):
        raise ValueError("unresolved AUR template placeholder")
    destination.mkdir(parents=True, exist_ok=True)
    result = destination / "PKGBUILD"
    result.write_text(text)
    return result


def main():
    """【原生发布】【AUR入口】校验标签与固定源码，输出模板；不配置 SSH 或上传，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag", required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--output", type=Path, default=Path("target/aur-work"))
    parser.add_argument("--pkgrel", default="1")
    parser.add_argument("--binary", action="store_true")
    parser.add_argument("--assets", type=Path)
    options = parser.parse_args()
    root = Path.cwd()
    context = resolve(root, options.tag, options.source_sha)
    print(
        prepare(
            root,
            context,
            options.output,
            options.binary,
            options.assets,
            options.pkgrel,
        )
    )


if __name__ == "__main__":
    main()
