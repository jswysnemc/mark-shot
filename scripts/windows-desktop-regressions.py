#!/usr/bin/env python3
"""【Windows桌面回归】【用户症状】在私有显示验证无控制台启动、直接取消截图和可配置贴图边框"""

import argparse
import json
from pathlib import Path
import struct
import subprocess
import time

from PIL import Image
from windows_ocr_actions import path_w, snapshot, stop, window, xdo
from windows_test_runtime import run


def subsystem(binary):
    """【Windows桌面回归】【可执行格式】读取 binary 的 PE 子系统，返回编号，不启动或修改程序"""
    data = Path(binary).read_bytes()
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    assert data[pe:pe + 4] == b"PE\0\0", "invalid PE image"
    return struct.unpack_from("<H", data, pe + 24 + 68)[0]


def execute(command, environment, root, log):
    """【Windows桌面回归】【实际窗口】按 command、environment、root 与 log 验证用户症状，返回退出结果并保存逐项证据"""
    binary = command[0]
    report = {"cases": {}, "passed": False, "real_windows_validation": False}
    config = root / "config.toml"
    config.write_text('[general]\nto=""\n[notifications]\nenabled=false\n[history]\nenabled=false\n'
                      '[pinned]\nborder_enabled=true\nborder_color="#FF00FF"\nborder_width=4.0\n')
    source = root / "source.png"
    Image.new("RGB", (240, 160), (240, 240, 240)).save(source)
    processes = []
    try:
        # 1. 【Windows桌面回归】【焦点限制】其他窗口保持焦点时 Escape 仍应取消截图，不能依赖先点击截图
        other = subprocess.Popen(["wine", binary, "--config", path_w(config, environment), "--pin",
                                  "--position", "500,400", path_w(source, environment)],
                                 env=environment, cwd=root, stdout=log, stderr=log)
        processes.append(other)
        other_window = window(environment, "Pinned Mark Shot", root, other)
        capture = subprocess.Popen(["wine", binary, "--config", path_w(config, environment), "--capture"],
                                   env=environment, cwd=root, stdout=log, stderr=log)
        processes.append(capture)
        window(environment, "Mark Shot", root, capture)
        xdo(environment, "windowfocus", "--sync", other_window)
        xdo(environment, "key", "--clearmodifiers", "Escape")
        try:
            report["cases"]["escape_without_click"] = capture.wait(timeout=3) == 0 and other.poll() is None
        except subprocess.TimeoutExpired:
            report["cases"]["escape_without_click"] = False
            stop(capture)
        stop(other)
        # 2. 【Windows桌面回归】【边框像素】实际贴图四边采用配置颜色，中心仍为原图颜色
        pin = subprocess.Popen(["wine", binary, "--config", path_w(config, environment), "--pin",
                                "--position", "180,180", path_w(source, environment)],
                               env=environment, cwd=root, stdout=log, stderr=log)
        processes.append(pin)
        window(environment, "Pinned Mark Shot", root, pin)
        time.sleep(0.2)
        image = snapshot(environment, root, "pin-border")
        points = [(180, 260), (419, 260), (300, 180), (300, 339)]
        report["cases"]["configured_border"] = all(image.getpixel(point) == (255, 0, 255) for point in points)
        report["cases"]["original_center"] = image.getpixel((300, 260)) == (240, 240, 240)
        report["cases"]["gui_subsystem"] = subsystem(binary) == 2
        report["passed"] = all(report["cases"].values())
        return subprocess.CompletedProcess(command, 0 if report["passed"] else 1)
    finally:
        for process in processes:
            stop(process)
        (root / "desktop-regressions.json").write_text(json.dumps(report, indent=2) + "\n")


def main():
    """【Windows桌面回归】【隔离入口】读取候选和新 root 后执行真实窗口验证，无参数、无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    result = run(options.root.resolve(), [str(options.binary.resolve())], execute=execute)
    print(json.dumps(result, indent=2))
    raise SystemExit(result["exit_code"])


if __name__ == "__main__":
    main()
