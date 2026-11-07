#!/usr/bin/env python3
"""【画布导航回归】【旋转窗口单位】私有Wayland测量放大后的实际控制点，验证命中、历史和导出"""

import argparse
import hashlib
import importlib.util
import json
import math
import re
import time
from pathlib import Path

from PIL import ImageChops

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("canvas_rotation_navigation", PROJECT / "scripts/wayland-canvas-navigation.py")
NAVIGATION = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(NAVIGATION)
FLOW, INPUT, session = NAVIGATION.FLOW, NAVIGATION.INPUT, NAVIGATION.session
ROTATION, TOOLS = NAVIGATION.ROTATION, NAVIGATION.TOOLS
ORANGE = ROTATION.ORANGE
PARK = (650, 480)
REGION = (150, 110, 510, 380)


def project(point, factor):
    """【画布导航回归】【输入投影】按point与factor返回锚点320×240附近的鼠标目标，实际几何由截图独立测量"""
    return 320 + (point[0] - 320) * factor, 240 + (point[1] - 240) * factor


def received_steps(log):
    """【画布导航回归】【协议刻度】按log读取实际纵轴完整刻度，返回正向放大次数，不用投递调用次数替代合成器事件"""
    source = log.read_text()
    values = re.findall(r"wl_pointer@\d+\.axis_value120, \(0, (-?\d+)\)", source)
    if values:
        steps = -sum(int(value) for value in values) / 120
    else:
        values = re.findall(r"wl_pointer@\d+\.axis, \(\d+, 0, (-?[\d.]+)\)", source)
        steps = -sum(float(value) for value in values) / 10
    assert steps > 0, "no complete zoom wheel input reached the actual Wayland seat"
    return steps


def corner_positions(bounds, factor):
    """【画布导航回归】【角点投影】按bounds和factor返回四个角点的窗口目标，不限制到历史截图裁切区"""
    left, top, right, bottom = bounds
    return [project(point, factor) for point in ((left, top), (right, top), (right, bottom), (left, bottom))]


def corners_visible(image, bounds, factor):
    """【画布导航回归】【角点到达】按image、bounds和factor返回四个实际控制点是否均可见，未提交画面返回假值"""
    return all(len(FLOW.pixels(image, (x - 6, y - 6, x + 6, y + 6), ORANGE)) > 25 for x, y in corner_positions(bounds, factor))


def centroid(image, point):
    """【画布导航回归】【实际定点】按image和point附近的橙色像素返回中心，控制点不可见时失败"""
    x, y = point
    pixels = FLOW.pixels(image, (x - 6, y - 6, x + 6, y + 6), ORANGE)
    assert len(pixels) > 25, f"actual corner was not visible near {point}"
    return sum(x for x, _ in pixels) / len(pixels), sum(y for _, y in pixels) / len(pixels)


def controls(image, bounds, factor):
    """【画布导航回归】【窗口间隔】按image、原始bounds和factor定位实际四角及旋转圆心，返回中心、圆心、外移距离"""
    corners = [centroid(image, point) for point in corner_positions(bounds, factor)]
    center = (sum(point[0] for point in corners) / 4, sum(point[1] for point in corners) / 4)
    top = ((corners[0][0] + corners[1][0]) / 2, (corners[0][1] + corners[1][1]) / 2)
    pixels = FLOW.pixels(image, REGION, ORANGE)
    first_y = min(y for _, y in pixels)
    circle = [(x, y) for x, y in pixels if y < first_y + 10]
    assert len(circle) > 30, "rotation circle did not reach the compositor"
    handle = (sum(x for x, _ in circle) / len(circle), sum(y for _, y in circle) / len(circle))
    return center, handle, math.dist(top, handle)


def same(root, name, expected):
    """【画布导航回归】【历史画面】按root、name和expected等待正文与控件逐像素恢复，返回实际截图，超时失败"""
    INPUT.move(PARK)
    return TOOLS.presented(root, name, lambda image: ImageChops.difference(expected.crop(REGION), image.crop(REGION)).getbbox() is None, f"{name}: navigated rotation history did not restore all pixels")


def history_key(value):
    """【画布导航回归】【独立历史按键】按value发送历史快捷键，间隔超过360毫秒，避免同时触发双击Ctrl导航复位，无返回值"""
    time.sleep(0.4)
    session.key(value)


def verify(binary, environment, root, grouped):
    """【画布导航回归】【旋转流程】在environment运行binary，grouped选择单项或共同旋转，root保存所有现场并返回结果"""
    root.mkdir()
    environment = dict(environment, XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    config = root / "config.toml"
    config.write_text('[general]\noverlay="window"\nto=""\n[notifications]\nenabled=false\n[history]\nenabled=false\n[tools]\ncolor="#334155"\n[tools.stroke]\nwidth=3\n')
    path, _ = NAVIGATION.LAYOUT.original(root, 255)
    bounds = (260, 230, 390, 290) if grouped else (260, 210, 380, 270)
    with INPUT.launch(binary, environment, config, path, root, "group" if grouped else "ordinary") as (process, log):
        FLOW.wait_input(process, log, (200, 200))
        session.key("r")
        INPUT.drag((260, 230 if grouped else 210), (380, 270))
        if grouped:
            INPUT.drag((330, 250), (390, 290))
        # 1. 【画布导航回归】【真实缩放】在未选中状态缩放图片，再在实际放大坐标选择标注，避免粗细滚轮的优先职责
        session.key("v")
        INPUT.click((200, 160))
        INPUT.move((320, 240))
        NAVIGATION.wheel_up(5)
        steps = received_steps(log)
        factor = 1.12 ** steps
        if grouped:
            INPUT.drag(project((230, 225), factor), project((410, 310), factor))
        else:
            INPUT.click(project((280, 210), factor))
        INPUT.move(PARK)
        selected = TOOLS.presented(root, "selected-zoomed", lambda image: corners_visible(image, bounds, factor), "navigated selection did not submit four expected corner controls")
        center, handle, gap = controls(selected, bounds, factor)
        assert abs(gap - 26.0) <= 1.2, f"rotation gap scaled with the image: {gap:.3f} window pixels"
        # 2. 【画布导航回归】【实际旋转】真实圆点接受左键拖动，单项旋转点跟随姿态，群组联合框重新保持轴对齐
        target = ROTATION.rotated(handle, center, 90)
        with ROTATION.modifier("Shift_L"):
            INPUT.drag(handle, target)
        INPUT.move(PARK)
        quarter = TOOLS.presented(root, "quarter-turn", lambda image: ImageChops.difference(selected.crop(REGION), image.crop(REGION)).getbbox() is not None, "navigated rotation pointer target changed no visible pixels")
        if not grouped:
            assert len(FLOW.pixels(quarter, (target[0] - 8, target[1] - 8, target[0] + 8, target[1] + 8), ORANGE)) > 30, "ordinary rotation handle did not follow the real pose"
        history_key("ctrl+z")
        same(root, "undo-selected", selected)
        history_key("ctrl+y")
        same(root, "redo-quarter", quarter)
        history_key("ctrl+z")
        same(root, "restored-selected", selected)
        # 3. 【画布导航回归】【静止与导出】倍率提示结束后采样资源，生产复制只交付正文，不把选框或旋转点写入原图
        time.sleep(1.05)
        idle = FLOW.NAV.SIZE.idle(process, log)
        exported = ROTATION.export(environment, process, root, "export")
        assert exported["size"] == [384, 240], "navigation changed the original export dimensions"
    result = {"grouped": grouped, "received_wheel_steps": steps, "zoom_factor": factor, "window_gap": gap, "center": center, "rotation_handle": handle, "actual_pointer_rotation": True, "exact_history": True, "history_region": list(REGION), "idle": idle, "export": exported}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def run(binary, root, selected_cases=(False, True)):
    """【画布导航回归】【独立矩阵】按binary、新root与selected_cases验证私有座席的单项或群组，返回指纹与实际结果"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(TMPDIR=str(root / "temporary"), MARK_SHOT_LANG="english", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/canvas-rotation-test")
    Path(environment["TMPDIR"]).mkdir()
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    cases = []
    with session.input_context(), FLOW.NAV.SIZE.pointer_context(backend="x11" if session.backend() is session.niri else "virtual"):
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        for grouped in selected_cases:
            value = verify(binary, environment, root / ("group" if grouped else "ordinary"), grouped)
            cases.append(value)
            print(json.dumps(value), flush=True)
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == fingerprint
    result = {"binary_sha256": fingerprint, "compositor": session.backend().__name__, "cases": cases, "scope": "Actual ordinary and group rotation gap in a zoomed image, real pointer rotation, exact history within the recorded canvas region, idle resources and original export dimensions. Specialty geometry and full Qt pixel parity remain separate."}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    """【画布导航回归】【命令入口】解析独立binary和新root，输出完整结果，失败保留现场，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--case", choices=("ordinary", "group"))
    options = parser.parse_args()
    selected_cases = (options.case == "group",) if options.case else (False, True)
    print(json.dumps(run(options.binary.resolve(), options.root.resolve(), selected_cases), indent=2))


if __name__ == "__main__":
    main()
