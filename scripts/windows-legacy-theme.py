#!/usr/bin/env python3
"""【Windows旧主题】【实际矩阵】私有显示比较旧全局默认色与当前明暗默认，不操作用户配置"""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from PIL import Image, ImageChops
from windows_legacy_checks import case
from windows_ocr_actions import snapshot
from windows_test_runtime import run


def execute(command, environment, root, log):
    """【Windows旧主题】【同构建矩阵】按command在root验证默认、明暗与旧配色完整帧，返回实际状态并保留失败证据"""
    binary, probe = command
    source = root / "source.png"
    image = Image.new("RGBA", (320, 200), (0, 0, 0, 0))
    image.paste((190, 100, 50, 255), (20, 20, 300, 180))
    image.save(source)
    report = {"binary_sha256": hashlib.sha256(Path(binary).read_bytes()).hexdigest(), "cases": [],
              "passed": False, "real_windows_validation": False}
    try:
        frames = {}
        for mode in ("light", "dark"):
            frames[mode] = case(binary, probe, environment, root / (mode + "-default"), log, source, mode, False)
            legacy = case(binary, probe, environment, root / (mode + "-legacy"), log, source, mode, True)
            assert ImageChops.difference(frames[mode], legacy).getbbox() is None, f"legacy {mode} theme differs from current defaults"
            report["cases"].append({"mode": mode, "legacy_frame_exact": True, "original_export_exact": True})
        implicit = case(binary, probe, environment, root / "implicit-legacy", log, source, None, True)
        assert ImageChops.difference(frames["light"], implicit).getbbox() is None, "old configuration without mode did not default to light"
        assert ImageChops.difference(frames["light"], frames["dark"]).getbbox() is not None, "both themes produced the same frame"
        report["default_light_exact"] = True
        report["passed"] = True
        return subprocess.CompletedProcess(command, 0)
    except Exception as error:
        report["error"] = str(error)
        snapshot(environment, root, "failure")
        raise
    finally:
        (root / "workflow.json").write_text(json.dumps(report, indent=2) + "\n")


def main():
    """【Windows旧主题】【隔离入口】解析候选、probe与新root，运行私有矩阵并返回退出码，无参数"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--probe", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    result = run(options.root.resolve(), [str(options.binary.resolve()), str(options.probe.resolve())], execute=execute)
    print(json.dumps(result, indent=2))
    raise SystemExit(result["exit_code"])


if __name__ == "__main__":
    main()
