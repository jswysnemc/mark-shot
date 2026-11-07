#!/usr/bin/env python3
"""【尺子窗口回归】【原生输入】私有Wayland检查旋转把手、真实拖动、历史像素和空闲资源"""

import argparse
import hashlib
import importlib.util
import json
import time
from pathlib import Path

from PIL import ImageChops

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("ruler_native_hit", PROJECT / "scripts/wayland-annotation-hit.py")
HIT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(HIT)
STYLE, INPUT, FLOW, TOOLS, session = HIT.STYLE, HIT.INPUT, HIT.FLOW, HIT.TOOLS, HIT.session


def orange_at(image, point):
    """【尺子窗口回归】【实际圆心】按image和point检查圆心附近实际橙色像素，不以输入日志代替画面"""
    x, y = point
    return len(FLOW.pixels(image, (round(x) - 2, round(y) - 2, round(x) + 3, round(y) + 3), (251, 146, 60))) >= 12


def verify(binary, environment, root, direction, width):
    """【尺子窗口回归】【单场景】按binary、environment、root、direction和width创建缩放后尺子，返回把手、历史和资源证据"""
    from PIL import Image
    root.mkdir()
    environment = dict(environment, XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    config = root / "config.toml"
    config.write_text('[general]\noverlay="window"\nto=""\n[notifications]\nenabled=false\n[history]\nenabled=false\n[tools]\ncolor="#000000"\n[tools.ruler]\nwidth=' + str(width) + '\nmode="length"\n')
    original = root / "original.png"
    Image.new("RGBA", (1280, 960), (255, 255, 255, 255)).save(original)
    with INPUT.launch(binary, environment, config, original, root, "ruler") as (process, log):
        FLOW.wait_input(process, log, (200, 200))
        session.key("Space")
        baseline = HIT.NAVIGATION.snapshot(root, "initial")
        center = (baseline.width / 2, baseline.height / 2)
        INPUT.move(center)
        HIT.wheel(direction, 6)
        received = HIT.steps(log, direction)
        factor = max(0.25, 1.12 ** received)
        # 1. 【尺子窗口回归】【真实创建】几何取实际窗口位置，工具样式继续使用原图像素笔宽
        session.key("u")
        start, end = (center[0] - 60, center[1] - 20), (center[0] + 60, center[1] + 50)
        INPUT.drag(start, end)
        session.key("v")
        INPUT.click(center)
        INPUT.move(STYLE.SUPPORT.PARK)
        fitted = min(baseline.width / 1280, baseline.height / 960)
        unit = max(0.75, min(3.5, 1 + (width - 3) / 9)) * fitted * factor
        # 2. 【尺子窗口回归】【Qt普通布局】此场景两条条带完整容纳，Qt顶部留量为三十单位，连线固定二十窗口像素
        handle = (center[0], start[1] - 30 * unit - 20)
        assert 10 < handle[1] < baseline.height - 10, "reference handle must be inside this native window"
        selected = TOOLS.presented(root, "selected", lambda image: orange_at(image, handle), "Qt reference rotation handle did not reach the private compositor")
        region = (max(0, round(center[0] - 150)), max(0, round(handle[1] - 20)), min(baseline.width, round(center[0] + 150)), min(baseline.height, round(end[1] + 100)))
        # 3. 【尺子窗口回归】【真实旋转与历史】点击画面上的把手，旋转后撤销必须恢复同一区域全部像素
        INPUT.drag(handle, (handle[0] + 24, handle[1] + 8))
        INPUT.move(STYLE.SUPPORT.PARK)
        rotated = TOOLS.presented(root, "rotated", lambda image: ImageChops.difference(image.crop(region), selected.crop(region)).getbbox() is not None, "rotation drag did not change the actual ruler frame")
        session.key("ctrl+z")
        INPUT.move(STYLE.SUPPORT.PARK)
        restored = TOOLS.presented(root, "restored", lambda image: ImageChops.difference(image.crop(region), selected.crop(region)).getbbox() is None, "native rotation undo did not restore exact ruler pixels")
        assert ImageChops.difference(rotated.crop(region), restored.crop(region)).getbbox() is not None
        time.sleep(1.05)
        idle = FLOW.NAV.SIZE.idle(process, log)
        exported = STYLE.SUPPORT.ROTATION.export(environment, process, root, "export")
        assert exported["size"] == [1280, 960]
    result = {"direction": direction, "width": width, "received_steps": received, "zoom": factor, "unit": unit, "expected_qt_handle": handle, "exact_undo": True, "export": exported, "idle": idle}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def run(binary, root):
    """【尺子窗口回归】【私有矩阵】按binary和新root执行四个单输出场景，返回不可变候选指纹与覆盖范围"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(TMPDIR=str(root / "temporary"), MARK_SHOT_LANG="english", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/ruler-window-test")
    Path(environment["TMPDIR"]).mkdir()
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    cases = []
    with session.input_context(), FLOW.NAV.SIZE.pointer_context(backend="x11" if session.backend() is session.niri else "virtual"):
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        for direction in (-1, 1):
            for width in (1, 6):
                result = verify(binary, environment, root / f"zoom-{direction}-width-{width}", direction, width)
                cases.append(result)
                print(json.dumps(result), flush=True)
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == fingerprint
    result = {"binary_sha256": fingerprint, "compositor": session.backend().__name__, "cases": cases, "scope": "Native selected length ruler handle for two widths and zoom directions, rotation gesture, exact undo pixels, original-size export and idle. Ordinary fully fitting tick bands only. Edge switching, flat rulers, mixed outputs, full Qt pixel parity and cold startup remain outside this native capture."}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    """【尺子窗口回归】【入口】解析候选binary和新root，执行校验后的私有矩阵，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
