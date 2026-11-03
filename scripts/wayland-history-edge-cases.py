#!/usr/bin/env python3
"""【截图历史测试】【边界入口】在私有 Wayland 桌面执行尚未覆盖的历史边界场景"""

import argparse
import hashlib
import json
from pathlib import Path

import wayland_history_edges as edges


def run(binary, artifacts):
    """【截图历史测试】【边界执行】固定 binary 指纹，依次执行隔离案例；报告写入新 artifacts，不覆盖旧证据"""
    artifacts.mkdir(parents=True)
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    report = {
        "sha256": fingerprint,
        "binary_bytes": binary.stat().st_size,
    }
    for name, directory, scenario in [
        ("first_disabled", "first-disabled", edges.first_disabled),
        ("shortcut_guard", "shortcut-guard", edges.shortcut_guard),
        ("long_reedit", "long-reedit", edges.long_reedit),
    ]:
        destination = artifacts / directory
        report[name] = scenario(binary, destination)
        (destination / "result.json").write_text(json.dumps(report[name], indent=2) + "\n")
    if hashlib.sha256(binary.read_bytes()).hexdigest() != fingerprint:
        raise AssertionError("binary changed during the history edge regression")
    (artifacts / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report))


def main():
    """【截图历史测试】【边界命令】读取构建和新证据目录；执行失败时返回非零退出码"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, default=Path("target/wayland-history-edges-r0"))
    options = parser.parse_args()
    run(options.binary.resolve(), options.artifacts.resolve())


if __name__ == "__main__":
    main()
