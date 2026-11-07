#!/usr/bin/env python3
"""【Windows贴图DPI】【实际流程】在私有高DPI会话验证显示几何、滚轮锚点、键盘菜单及原图输出"""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from PIL import Image
from windows_dpi_session import configure
from windows_ocr_actions import key, path_w, rectangle, snapshot, stop, window, xdo
from windows_pin_checks import absent, copied, menu, size
from windows_test_runtime import run


def execute(command, environment, root, log):
    """【Windows贴图DPI】【同构建执行】按command设置私有dpi并运行binary，返回结果，尺寸和像素断言不读取产品内部状态"""
    binary, probe, dpi = command
    environment = configure(environment, root, log, dpi)
    scale = int(dpi) / 96
    config = root / "config.toml"
    config.write_text('[ui]\ntheme="light"\nlanguage="en"\n[notifications]\nenabled=false\n[history]\nenabled=false\n')
    displays = json.loads(subprocess.check_output(["wine", binary, "--config", path_w(config, environment), "--list-displays"],
                                                  env=environment, cwd=root, text=True, timeout=15))
    assert len(displays) == 1 and displays[0]["scale"] == scale, "actual application DPI differs from requested DPI"
    source = root / "source.png"
    image = Image.new("RGBA", (320, 200))
    image.putdata([(x % 255, y % 255, (x + y) % 255, 255) for y in range(200) for x in range(320)])
    image.save(source)
    report = {"binary_sha256": hashlib.sha256(Path(binary).read_bytes()).hexdigest(), "dpi": int(dpi), "scale": scale,
              "passed": False, "real_windows_validation": False}
    process = subprocess.Popen(["wine", binary, "--config", path_w(config, environment), "--pin", "--position", "100,100", path_w(source, environment)],
                               env=environment, cwd=root, stdout=log, stderr=log)
    try:
        identity = window(environment, "Pinned Mark Shot", root, process)
        before = size(environment, identity, (int(320 * scale), int(200 * scale)))
        copied(probe, environment, root, identity, "initial-original", image)
        xdo(environment, "windowfocus", "--sync", identity)
        xdo(environment, "mousemove", "--window", identity, round(80 * scale), round(50 * scale))
        xdo(environment, "click", 4)
        enlarged = size(environment, identity, (round(346 * scale), round(216 * scale)))
        assert abs(enlarged["x"] - (before["x"] - 6.5 * scale)) <= 1
        assert abs(enlarged["y"] - (before["y"] - 4 * scale)) <= 1
        copied(probe, environment, root, identity, "zoomed-original", image)
        xdo(environment, "click", 5)
        restored = size(environment, identity, (int(320 * scale), int(200 * scale)))
        assert abs(restored["x"] - before["x"]) <= 1 and abs(restored["y"] - before["y"]) <= 1
        # 1. 【Windows贴图DPI】【菜单缩放】真实菜单像素尺寸按同一DPI增长，键盘确认仍交付正确旋转
        popup = menu(environment, root, identity)
        geometry = rectangle(environment, popup)
        assert geometry["width"] >= 200 * scale, "menu did not scale with the window"
        snapshot(environment, root, "dpi-menu")
        key(environment, identity, "Home")
        key(environment, identity, "Down")
        key(environment, identity, "Return")
        absent(environment, "Mark Shot menu")
        size(environment, identity, (int(200 * scale), int(320 * scale)))
        copied(probe, environment, root, identity, "rotated-original", image.transpose(Image.Transpose.ROTATE_270))
        key(environment, identity, "Escape")
        assert process.wait(timeout=8) == 0
        report.update(passed=True, before=before, enlarged=enlarged, restored=restored, menu=geometry, original_exact=True)
        return subprocess.CompletedProcess(command, 0)
    except Exception as error:
        report["error"] = str(error)
        snapshot(environment, root, "failure")
        raise
    finally:
        stop(process)
        (root / "workflow.json").write_text(json.dumps(report, indent=2) + "\n")


def main():
    """【Windows贴图DPI】【隔离入口】解析binary、probe、dpi与新root并输出运行状态，无参数、无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--probe", type=Path, required=True)
    parser.add_argument("--dpi", type=int, choices=[144, 192], required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    result = run(options.root.resolve(), [str(options.binary.resolve()), str(options.probe.resolve()), str(options.dpi)], execute=execute)
    print(json.dumps(result, indent=2))
    raise SystemExit(result["exit_code"])


if __name__ == "__main__":
    main()
