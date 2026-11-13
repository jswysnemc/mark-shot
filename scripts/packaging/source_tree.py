"""【原生源码】【归档基础】从确认过的提交创建干净源码树与可复现的组件包"""

import gzip
from pathlib import Path
import subprocess
import tarfile

from artifact_metadata import source_sha


def checkout(repository, commit, destination, identity=True):
    """【原生源码】【提交归档】从 repository 的 commit 创建 destination，identity 控制是否写来源文件，返回源码目录"""
    destination = Path(destination)
    if destination.exists():
        raise FileExistsError(f"source destination already exists: {destination}")
    source_sha(commit)
    destination.mkdir(parents=True)
    process = subprocess.Popen(
        ["git", "-C", str(repository), "archive", commit], stdout=subprocess.PIPE
    )
    try:
        with tarfile.open(fileobj=process.stdout, mode="r|") as archive:
            archive.extractall(destination, filter="data")
    finally:
        process.stdout.close()
        code = process.wait()
    if code:
        raise subprocess.CalledProcessError(code, process.args)
    if identity:
        (destination / "SOURCE-COMMIT").write_text(commit + "\n")
    return destination


def archive(directory, destination, name, epoch):
    """【原生源码】【组件归档】将 directory 以 name 为顶层目录写入 destination，统一 epoch、权限所有者，返回归档路径"""
    directory, destination = map(Path, (directory, destination))
    destination.parent.mkdir(parents=True, exist_ok=True)
    with destination.open("wb") as raw:
        compressed = (
            raw
            if destination.name.endswith(".xz")
            else gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=epoch)
        )
        try:
            with tarfile.open(
                fileobj=compressed,
                mode="w:xz" if destination.name.endswith(".xz") else "w",
                format=tarfile.PAX_FORMAT,
            ) as output:
                _entries(output, directory, name, epoch)
        finally:
            if compressed is not raw:
                compressed.close()
    return destination


def _entries(output, directory, name, epoch):
    """【原生源码】【归档条目】将 directory 的规范化条目写入 output，使用 name 和 epoch，无返回值"""
    for path in [directory, *sorted(directory.rglob("*"))]:
        relative = Path(name) / path.relative_to(directory)
        entry = output.gettarinfo(str(path), relative.as_posix())
        entry.uid = entry.gid = 0
        entry.uname = entry.gname = "root"
        entry.mtime = epoch
        if entry.isfile():
            with path.open("rb") as source:
                output.addfile(entry, source)
        else:
            output.addfile(entry)
