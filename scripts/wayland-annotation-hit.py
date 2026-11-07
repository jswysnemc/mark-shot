#!/usr/bin/env python3
"""【标注命中回归】【原生流程】仅在已验证私有Wayland检查缩放后的首次线条选择、导出和空闲状态"""

import argparse
import hashlib
import importlib.util
import json
import math
import re
import time
from pathlib import Path

from PIL import Image

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("hit_native_style", PROJECT / "scripts/wayland-annotation-style.py")
STYLE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(STYLE)
SUPPORT = STYLE.SUPPORT
NAVIGATION, FLOW, INPUT, TOOLS, session = STYLE.NAVIGATION, STYLE.FLOW, STYLE.INPUT, STYLE.TOOLS, STYLE.session
REGION = (210, 140, 480, 330)


def wheel(direction, count):
    """【标注命中回归】【完整滚轮】按direction和count向已校验私有设备发送纵轴刻度，无返回值"""
    for _ in range(count):
        pointer = INPUT.CURRENT.get()[0]
        if pointer is None:
            FLOW.NAV.wheel_up() if direction > 0 else FLOW.NAV.wheel_down()
        else:
            pointer.scroll(dy=-10.0 * direction)
        time.sleep(0.05)


def steps(log, direction):
    """【标注命中回归】【真实刻度】按log和direction读取实际Wayland纵轴事件，返回有符号刻度，未到达时失败"""
    source = log.read_text()
    values = re.findall(r"wl_pointer@\d+\.axis_value120, \(0, (-?\d+)\)", source)
    result = -sum(int(value) for value in values) / 120 if values else -sum(float(value) for value in re.findall(r"wl_pointer@\d+\.axis, \(\d+, 0, (-?[\d.]+)\)", source)) / 10
    assert result * direction > 0, "complete signed wheel input did not reach the private Wayland seat"
    return result


def selected(image):
    """【标注命中回归】【选中画面】按image返回正文区域是否出现橙色控制框，不读取模型或预期坐标代替画面"""
    return len(FLOW.pixels(image, REGION, (251, 146, 60))) > 50


def white_bounds(image):
    """【标注命中回归】【实际图像范围】按image返回隐藏界面后的白色原图像素范围，未提交原图时返回空值"""
    return image.convert("L").point(lambda value: 255 if value > 250 else 0).getbbox()


def click(process, log, target):
    """【标注命中回归】【按钮到达】按process、log和target发送点击并确认实际Wayland按下及释放到达，返回新增按钮事件数量"""
    pattern = r"wl_pointer@\d+\.button,"
    before = len(re.findall(pattern, log.read_text()))
    INPUT.click(target)
    deadline = time.monotonic() + 3.0
    while True:
        assert process.poll() is None, "editor exited before the first boundary click arrived"
        received = len(re.findall(pattern, log.read_text())) - before
        if received >= 2:
            return received
        assert time.monotonic() < deadline, "first boundary button press and release did not reach the private Wayland seat"
        time.sleep(0.03)


def verify(binary, environment, root, direction, outside):
    """【标注命中回归】【单项输入】按binary、environment、root、direction和outside执行真实首次点击，返回画面、导出与资源证据"""
    root.mkdir()
    environment = dict(environment, XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    config = root / "config.toml"
    config.write_text('[general]\noverlay="window"\nto=""\n[notifications]\nenabled=false\n[history]\nenabled=false\n[tools]\ncolor="#000000"\n[tools.line]\nwidth=6\n')
    original = root / "original.png"
    Image.new("RGBA", (1280, 960), (255, 255, 255, 255)).save(original)
    with INPUT.launch(binary, environment, config, original, root, "line") as (process, log):
        FLOW.wait_input(process, log, (200, 200))
        session.key("Space")
        baseline = NAVIGATION.snapshot(root, "initial")
        baseline_bounds = white_bounds(baseline)
        assert baseline_bounds is not None, "white original image did not reach the private compositor"
        center = (baseline.width / 2.0, baseline.height / 2.0)
        session.key("d")
        INPUT.drag((center[0] - 40, center[1]), (center[0] + 40, center[1]))
        session.key("v")
        INPUT.move(center)
        wheel(direction, 12)
        received = steps(log, direction)
        factor = max(0.25, 1.12 ** received)
        fitted = min(baseline.width / 1280, baseline.height / 960)
        half_width = 6.0 * fitted * factor / 2.0
        expected_width = max(1.5, half_width * 2.0)
        expected_image_size = (min((baseline_bounds[2] - baseline_bounds[0]) * factor, baseline.width), min((baseline_bounds[3] - baseline_bounds[1]) * factor, baseline.height))
        def presented(image):
            """【标注命中回归】【缩放提交】按image同时核对实际白色原图范围和中心笔迹，返回是否完成本次缩放提交"""
            bounds = white_bounds(image)
            return bounds is not None and all(abs(actual - expected) <= 3.0 for actual, expected in zip((bounds[2] - bounds[0], bounds[3] - bounds[1]), expected_image_size)) and STYLE.width(image, round(center[0]), math.floor(center[1] - 30), math.ceil(center[1] + 30)) >= expected_width * 0.8 and not selected(image)
        INPUT.move(SUPPORT.PARK)
        before = TOOLS.presented(root, "unselected", presented, "unselected line and actual zoomed image bounds did not reach the private compositor")
        assert not selected(before), "line was already selected before the boundary click"
        # 1. 【标注命中回归】【首次点击】七像素接受、九像素拒绝，点击完成后再等待真实控制框
        target = (center[0], center[1] + half_width + outside)
        button_events = click(process, log, target)
        INPUT.move(SUPPORT.PARK)
        expected = outside == 7
        after = TOOLS.presented(root, "after-click", lambda image: selected(image) == expected, f"first line click at {outside} window pixels had wrong selection after zoom {factor}")
        time.sleep(1.05)
        idle = FLOW.NAV.SIZE.idle(process, log)
        exported = SUPPORT.ROTATION.export(environment, process, root, "export")
        assert exported["size"] == [1280, 960], "first selection changed original image dimensions"
    result = {"direction": direction, "outside_window_pixels": outside, "received_wheel_steps": received, "received_button_events": button_events, "zoom_factor": factor, "projected_half_width": half_width, "line_center": center, "presented_image_bounds": white_bounds(before), "expected_visible_image_size": expected_image_size, "click": target, "selected": selected(after), "expected_selected": expected, "idle": idle, "export": exported}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def run(binary, root):
    """【标注命中回归】【私有矩阵】按binary和新root运行四个独立场景，返回不可变候选指纹，失败保留现场"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(TMPDIR=str(root / "temporary"), MARK_SHOT_LANG="english", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/annotation-hit-test")
    Path(environment["TMPDIR"]).mkdir()
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    cases = []
    with session.input_context(), FLOW.NAV.SIZE.pointer_context(backend="x11" if session.backend() is session.niri else "virtual"):
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        for direction in (-1, 1):
            for outside in (7, 9):
                result = verify(binary, environment, root / f"zoom-{direction}-outside-{outside}", direction, outside)
                cases.append(result)
                print(json.dumps(result), flush=True)
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == fingerprint
    assert len({case["export"]["sha256"] for case in cases}) == 1, "first selection changed original line export"
    result = {"binary_sha256": fingerprint, "compositor": session.backend().__name__, "cases": cases, "scope": "Native first line selection after zoom in and out, real visible controls, identical original exports, and idle samples. Other tools, selected rotation handles and complete Qt interface parity are not covered."}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    """【标注命中回归】【入口】解析binary和新root并执行私有矩阵，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
