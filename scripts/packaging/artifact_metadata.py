"""【原生打包】【产物契约】统一架构、版本、摘要与工作流输出"""

import hashlib
import argparse
import json
import os
from pathlib import Path
import platform
import re
import struct


ARCHITECTURES = {
    "x86_64": {"rust": "x86_64-unknown-linux-gnu", "deb": "amd64", "elf": 62},
    "aarch64": {"rust": "aarch64-unknown-linux-gnu", "deb": "arm64", "elf": 183},
}
VERSION = re.compile(
    r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?\Z"
)


def architecture(value=None):
    """【原生打包】【架构规范】按 value 或主机架构返回标准名称，未知架构直接报错"""
    value = value or platform.machine()
    value = {"arm64": "aarch64", "amd64": "x86_64"}.get(value, value)
    if value not in ARCHITECTURES:
        raise ValueError(f"unsupported native architecture: {value}")
    return value


def version(value):
    """【原生打包】【版本校验】按 value 返回合法上游版本，拒绝路径、空白及命令片段"""
    if not VERSION.fullmatch(value):
        raise ValueError(f"invalid native package version: {value}")
    return value


def source_sha(value):
    """【原生打包】【源码标识】按 value 返回完整提交摘要，拒绝缩写和非十六进制内容"""
    if not re.fullmatch(r"[0-9a-f]{40}", value):
        raise ValueError("a complete lowercase source commit is required")
    return value


def checksum(path):
    """【原生打包】【文件摘要】读取 path 并返回 SHA-256，按流处理大文件"""
    with Path(path).open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def verify_elf(path, arch):
    """【原生打包】【目标校验】验证 path 是 arch 对应的 64 位 ELF，成功时无返回值"""
    with Path(path).open("rb") as source:
        header = source.read(64)
    if len(header) < 64 or header[:6] != b"\x7fELF\x02\x01":
        raise ValueError(f"not a little-endian 64-bit ELF: {path}")
    if (
        struct.unpack_from("<H", header, 18)[0]
        != ARCHITECTURES[architecture(arch)]["elf"]
    ):
        raise ValueError(f"ELF architecture does not match {arch}: {path}")


def output(values, destination=None):
    """【原生打包】【工作流输出】将 values 写入 destination 或 GitHub 输出文件，并打印同一记录"""
    destination = destination or os.environ.get("GITHUB_OUTPUT")
    lines = []
    for key, value in values.items():
        value = str(value).lower() if isinstance(value, bool) else str(value)
        if not re.fullmatch(r"[a-z_]+", key) or any(char in value for char in "\r\n"):
            raise ValueError("unsafe workflow output")
        lines.append(f"{key}={value}\n")
    if destination:
        with Path(destination).open("a") as stream:
            stream.writelines(lines)
    print("".join(lines), end="")


def finalize(path, package_version, arch, commit, kind):
    """【原生打包】【完成产物】为 path 写摘要和来源记录，按版本、架构、提交与 kind 返回清单"""
    path = Path(path)
    digest = checksum(path)
    checksum_path = Path(str(path) + ".sha256")
    checksum_path.write_text(f"{digest}  {path.name}\n")
    manifest = {
        "file": path.name,
        "sha256": digest,
        "version": version(package_version),
        "architecture": architecture(arch),
        "source_commit": source_sha(commit),
        "format": kind,
    }
    Path(str(path) + ".json").write_text(json.dumps(manifest, indent=2) + "\n")
    output({"asset": path, "checksum": checksum_path, "manifest": str(path) + ".json"})
    return manifest


def main():
    """【原生打包】【摘要入口】读取最终文件及版本、架构、来源和格式，保存交付清单，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("asset", type=Path)
    parser.add_argument("--version", required=True)
    parser.add_argument("--arch", default=architecture())
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--format", required=True)
    options = parser.parse_args()
    finalize(
        options.asset, options.version, options.arch, options.source_sha, options.format
    )


if __name__ == "__main__":
    main()
