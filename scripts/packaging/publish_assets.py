"""【原生发布】【不可变产物】只上传当前原生标签的已校验资产，拒绝覆盖不同内容"""

import argparse
import json
from pathlib import Path
import subprocess
import tempfile

from artifact_metadata import checksum
from release_context import resolve


def assets(directory, context):
    """【原生发布】【资产校验】按 directory 和 context 返回匹配版本、来源及摘要的完整文件列表"""
    directory = Path(directory)
    result = []
    for record in sorted(directory.glob("*.json")):
        data = json.loads(record.read_text())
        name = data["file"]
        if Path(name).name != name or name in (".", ".."):
            raise ValueError("unsafe asset name")
        asset = directory / name
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
        result.extend((asset, sidecar, record))
    if not result:
        raise ValueError("there are no verified release assets")
    return result


def publish(directory, context):
    """【原生发布】【GitHub上传】上传 directory 中 context 对应资产，相同资产跳过、不同资产失败，无返回值"""
    files = assets(directory, context)
    view = subprocess.run(
        ["gh", "release", "view", context.tag, "--json", "tagName,assets"],
        capture_output=True,
        text=True,
        check=False,
    )
    if view.returncode:
        creation = subprocess.run(
            [
                "gh",
                "release",
                "create",
                context.tag,
                "--verify-tag",
                "--title",
                f"Mark Shot native {context.version}",
                "--notes",
                "Native Rust builds with in-process recording, OCR, translation and barcode scanning.",
                *(["--prerelease"] if "-" in context.version else []),
            ],
            check=False,
        )
        view = subprocess.run(
            ["gh", "release", "view", context.tag, "--json", "tagName,assets"],
            capture_output=True,
            text=True,
            check=True,
        )
        if creation.returncode:
            print("【原生发布】【并发协调】另一任务已创建发布，继续核对资产")
    metadata = json.loads(view.stdout)
    if metadata["tagName"] != context.tag:
        raise ValueError("release tag mismatch")
    existing = {item["name"] for item in metadata["assets"]}
    for path in files:
        if path.name in existing:
            with tempfile.TemporaryDirectory(prefix="mark-shot-release-") as temporary:
                subprocess.run(
                    [
                        "gh",
                        "release",
                        "download",
                        context.tag,
                        "--pattern",
                        path.name,
                        "--dir",
                        temporary,
                    ],
                    check=True,
                )
                if checksum(Path(temporary) / path.name) != checksum(path):
                    raise ValueError(
                        f"refusing to overwrite a different release asset: {path.name}"
                    )
        else:
            subprocess.run(
                ["gh", "release", "upload", context.tag, str(path)], check=True
            )


def main():
    """【原生发布】【发布入口】读取目录、标签与固定源码，重新验证标签后上传，不提供覆盖选项，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--source-sha", required=True)
    options = parser.parse_args()
    context = resolve(Path.cwd(), options.tag, options.source_sha, publish=True)
    publish(options.directory, context)


if __name__ == "__main__":
    main()
