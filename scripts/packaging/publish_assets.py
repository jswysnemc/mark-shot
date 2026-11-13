"""【原生发布】【不可变产物】只上传当前原生标签的已校验资产，拒绝覆盖不同内容"""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess

from artifact_metadata import checksum
from release_api import request, verify_asset
from release_context import resolve
from release_notes import render


PACKAGE_SUFFIXES = (
    ".deb",
    ".rpm",
    ".pkg.tar.zst",
    ".tar.gz",
    ".AppImage",
    ".flatpak",
    ".zip",
)


def assets(directory, context):
    """【原生发布】【资产校验】按 directory 和 context 返回安装包列表；摘要和元数据仅用于内部校验"""
    directory = Path(directory)
    result = []
    for record in sorted(directory.glob("*.json")):
        data = json.loads(record.read_text())
        name = data["file"]
        if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.+~-]*", name) or not name.endswith(
            PACKAGE_SUFFIXES
        ):
            raise ValueError("unsafe asset name")
        asset = directory / name
        if asset.is_symlink() or not asset.is_file():
            raise ValueError(f"asset must be a regular file: {name}")
        if (
            data["source_commit"] != context.commit
            or data["version"] != context.version
        ):
            raise ValueError(f"asset does not belong to this release: {name}")
        if data["sha256"] != checksum(asset):
            raise ValueError(f"asset checksum mismatch: {name}")
        sidecar = Path(str(asset) + ".sha256")
        if sidecar.read_text() != f"{data['sha256']}  {name}\n":
            raise ValueError(f"invalid checksum sidecar: {name}")
        result.append(asset)
    if not result:
        raise ValueError("there are no verified release assets")
    packages = {
        path for path in directory.iterdir() if path.name.endswith(PACKAGE_SUFFIXES)
    }
    if packages != set(result) or len(result) != len(set(result)):
        raise ValueError("release packages and verification records do not match")
    return result


def publish(directory, context, notes=None, repository=None):
    """【原生发布】【完整交付】校验 directory 中 context 对应安装包，按 notes 和 repository 生成页面，返回发布网址"""
    files = assets(directory, context)
    repository = repository or os.environ["GITHUB_REPOSITORY"]
    notes = notes or Path("release/notes") / f"{context.version}.md"
    body = render(notes, context, files, repository)
    base = f"repos/{repository}/releases"
    prerelease = "-" in context.version.split("+", 1)[0]
    metadata = request(f"{base}/tags/{context.tag}", missing_ok=True)
    # 1. 【原生发布】【草稿准备】首次发布只创建草稿，任何后续失败都不会公开不完整版本
    if metadata is None:
        metadata = request(
            base,
            "POST",
            {
                "tag_name": context.tag,
                "name": f"Mark Shot {context.version}",
                "body": body,
                "draft": True,
                "prerelease": prerelease,
                "make_latest": "false" if prerelease else "true",
            },
        )
    if metadata["tag_name"] != context.tag or metadata["prerelease"] != prerelease:
        raise ValueError("release tag mismatch")
    existing = {item["name"]: item for item in metadata["assets"]}
    if set(existing) - {path.name for path in files}:
        raise ValueError("release contains unexpected assets")
    # 2. 【原生发布】【不可变上传】相同文件只核验，不同内容拒绝覆盖；摘要文件和元数据不上传
    for path in files:
        if path.name in existing:
            verify_asset(context.tag, path, existing[path.name], repository)
        else:
            subprocess.run(
                [
                    "gh",
                    "release",
                    "upload",
                    context.tag,
                    "--repo",
                    repository,
                    str(path),
                ],
                check=True,
            )
    # 3. 【原生发布】【完整核验】所有远程安装包的字节摘要一致后，统一更新页面并公开发布
    metadata = request(f"{base}/{metadata['id']}")
    remote = {item["name"]: item for item in metadata["assets"]}
    if set(remote) != {path.name for path in files}:
        raise ValueError("published release assets mismatch")
    for path in files:
        verify_asset(context.tag, path, remote[path.name], repository)
    metadata = request(
        f"{base}/{metadata['id']}",
        "PATCH",
        {
            "body": body,
            "draft": False,
            "prerelease": prerelease,
            "make_latest": "false" if prerelease else "true",
        },
    )
    if (
        metadata["draft"]
        or metadata["prerelease"] != prerelease
        or metadata["body"] != body
    ):
        raise ValueError("release publication did not complete")
    return metadata["html_url"]


def main():
    """【原生发布】【发布入口】读取目录、标签与固定源码，重新验证标签后上传，不提供覆盖选项，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--notes", type=Path)
    options = parser.parse_args()
    context = resolve(Path.cwd(), options.tag, options.source_sha, publish=True)
    print(publish(options.directory, context, options.notes))


if __name__ == "__main__":
    main()
