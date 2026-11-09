#!/usr/bin/env python3
"""【CI下载】【固定资产】下载固定摘要资产，GitHub发布地址失败时通过官方API读取相同资产"""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time
from urllib.parse import urlparse


def download(url, destination, digest):
    """【CI下载】【完整交付】按url下载到destination并验证digest，失败不保留半份目标文件，无返回值"""
    destination = Path(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_suffix(destination.suffix + ".part")
    base = ["curl", "--fail", "--location", "--silent", "--show-error", "--retry", "3",
            "--connect-timeout", "20", "--max-time", "180"]
    try:
        result = subprocess.run([*base, "--output", str(temporary), url], check=False)
        if result.returncode:
            parsed = urlparse(url)
            parts = parsed.path.strip("/").split("/")
            if parsed.hostname != "github.com" or len(parts) != 6 or parts[2:4] != ["releases", "download"]:
                raise RuntimeError(f"asset download failed: {url}")
            owner, repo, _, _, tag, name = parts
            metadata = subprocess.check_output([*base,
                f"https://api.github.com/repos/{owner}/{repo}/releases/tags/{tag}"])
            assets = [asset for asset in json.loads(metadata)["assets"] if asset["name"] == name]
            if len(assets) != 1:
                raise RuntimeError(f"release asset is ambiguous: {name}")
            address = assets[0]["url"] + f"?download={time.time_ns()}"
            subprocess.run([*base, "--header", "Accept: application/octet-stream",
                            "--output", str(temporary), address], check=True)
        if hashlib.sha256(temporary.read_bytes()).hexdigest() != digest:
            raise RuntimeError(f"SHA-256 mismatch: {destination.name}")
        temporary.replace(destination)
    finally:
        temporary.unlink(missing_ok=True)


def main():
    """【CI下载】【命令入口】无参数，读取资产地址、输出与摘要，校验成功才交付文件"""
    parser = argparse.ArgumentParser()
    parser.add_argument("url")
    parser.add_argument("destination", type=Path)
    parser.add_argument("sha256")
    options = parser.parse_args()
    download(options.url, options.destination, options.sha256)


if __name__ == "__main__":
    main()
