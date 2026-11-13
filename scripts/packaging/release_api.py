"""【原生发布】【远程接口】封装 Release 查询及上传后的 SHA-256 核验"""

import json
from pathlib import Path
import shutil
import subprocess
import tempfile

from artifact_metadata import checksum
from release_asset_names import asset_name


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
        name = asset_name(path)
        subprocess.run(
            [
                "gh",
                "release",
                "download",
                tag,
                "--repo",
                repository,
                "--pattern",
                name,
                "--dir",
                temporary,
            ],
            check=True,
        )
        if checksum(Path(temporary) / name) != digest:
            raise ValueError(f"remote asset checksum mismatch: {path.name}")


def find_release(repository, tag, api=request):
    """【原生发布】【草稿定位】按 repository 和 tag 查询公开版本及草稿；api 为请求函数，返回发布记录或空值"""
    base = f"repos/{repository}/releases"
    release = api(f"{base}/tags/{tag}", missing_ok=True)
    if release is not None:
        return release
    # 1. 【原生发布】【草稿恢复】标签接口只查询公开版本，必须分页检查当前令牌可见的草稿
    page = 1
    while True:
        releases = api(f"{base}?per_page=100&page={page}")
        for release in releases:
            if release["tag_name"] == tag:
                return release
        if len(releases) < 100:
            return None
        page += 1


def upload_asset(tag, path, repository):
    """【原生发布】【规范上传】按 tag、path 和 repository 上传规范名称的文件，原始包和内部版本不变，无返回值"""
    path = Path(path)
    name = asset_name(path)
    with tempfile.TemporaryDirectory(prefix="mark-shot-release-upload-") as temporary:
        uploaded = path
        if name != path.name:
            uploaded = Path(temporary) / name
            shutil.copyfile(path, uploaded)
        subprocess.run(
            ["gh", "release", "upload", tag, "--repo", repository, str(uploaded)],
            check=True,
        )
