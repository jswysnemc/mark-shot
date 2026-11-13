"""【原生发布】【远程接口】封装 Release 查询及上传后的 SHA-256 核验"""

import json
from pathlib import Path
import subprocess
import tempfile

from artifact_metadata import checksum


def request(endpoint, method="GET", payload=None, missing_ok=False):
    """【原生发布】【接口请求】按 endpoint、method 和 payload 请求 GitHub，返回 JSON；missing_ok 仅容忍 404"""
    command = ["gh", "api", endpoint, "--method", method]
    if payload is not None:
        command.extend(("--input", "-"))
    result = subprocess.run(
        command,
        input=json.dumps(payload) if payload is not None else None,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode:
        if missing_ok and "(HTTP 404)" in result.stderr:
            return None
        raise RuntimeError(result.stderr.strip())
    return json.loads(result.stdout) if result.stdout.strip() else None


def verify_asset(tag, path, metadata, repository):
    """【原生发布】【远程摘要】按 tag、path、metadata 和 repository 核验 SHA-256；旧资产下载复核，成功无返回值"""
    digest = checksum(path)
    remote_digest = metadata.get("digest")
    if remote_digest:
        if remote_digest != f"sha256:{digest}":
            raise ValueError(f"remote asset checksum mismatch: {path.name}")
        return
    # 1. 【原生发布】【兼容核验】没有服务器摘要时下载原始字节，不依赖文件名和大小判断相等
    with tempfile.TemporaryDirectory(prefix="mark-shot-release-") as temporary:
        subprocess.run(
            [
                "gh",
                "release",
                "download",
                tag,
                "--repo",
                repository,
                "--pattern",
                path.name,
                "--dir",
                temporary,
            ],
            check=True,
        )
        if checksum(Path(temporary) / path.name) != digest:
            raise ValueError(f"remote asset checksum mismatch: {path.name}")
