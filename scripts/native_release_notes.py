"""【原生发布】【说明恢复】从本地文件或当前提交历史读取摘要固定的发布说明"""

import hashlib
from pathlib import Path
import subprocess

from native_release_manifest import require, sha256_file


def resolve_notes(repository, notes, expected_sha256, destination):
    """【原生发布】【说明定位】按 repository、notes 路径和 expected_sha256 校验说明，缺失时恢复到 destination，返回文件路径"""
    repository = Path(repository).resolve()
    notes = Path(notes)
    if not notes.is_absolute():
        notes = repository / notes
    if notes.is_file():
        require(sha256_file(notes) == expected_sha256, "release notes digest mismatch")
        return notes

    # 1. 【原生发布】【历史定位】只查询仓库内原路径，不读取忽略目录或其他分支
    relative = notes.resolve().relative_to(repository).as_posix()
    commits = subprocess.check_output(
        [
            "git",
            "--literal-pathspecs",
            "-C",
            str(repository),
            "log",
            "--format=%H",
            "--full-history",
            "--no-renames",
            "--diff-filter=AM",
            "--",
            relative,
        ],
        text=True,
    ).splitlines()
    for commit in commits:
        content = subprocess.check_output(
            ["git", "-C", str(repository), "show", f"{commit}:{relative}"]
        )
        if hashlib.sha256(content).hexdigest() != expected_sha256:
            continue
        # 2. 【原生发布】【摘要恢复】只有匹配清单的原始字节才写入临时输出
        destination = Path(destination)
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(content)
        return destination
    raise ValueError("verified release notes not found; full Git history is required")
