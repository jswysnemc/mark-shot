#!/usr/bin/env python3
"""【画布导航回归】【真实会话】私有Wayland验证实际缩放、平移、复位、滚动条、原图导出和空闲资源"""

import argparse
import hashlib
import importlib.util
import io
import json
import subprocess
import time
from pathlib import Path

from PIL import Image, ImageChops

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("canvas_navigation_layout", PROJECT / "scripts/wayland-canvas-layout.py")
LAYOUT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(LAYOUT)
FLOW, INPUT, session = LAYOUT.FLOW, LAYOUT.INPUT, LAYOUT.session
NAV = FLOW.NAV
ROTATION = LAYOUT.GROUP.ROTATION
TOOLS = ROTATION.TOOLS


def wheel_up(count):
    """【画布导航回归】【滚轮投递】按count向已校验设备发送完整纵轴刻度，无返回值"""
    for _ in range(count):
        pointer = INPUT.CURRENT.get()[0]
        if pointer is None:
            NAV.wheel_up()
        else:
            pointer.scroll(dy=-10.0)
        time.sleep(0.05)


def snapshot(root, name):
    """【画布导航回归】【原生图像】按root和name保存实际合成器画面，返回RGB图像"""
    INPUT.move((650, 480))
    return FLOW.snapshot(root, name)


def middle_drag(start, end):
    """【画布导航回归】【连续平移】通过私有设备从start向end发送中键完整位移，结束后释放，无返回值"""
    INPUT.move(start)
    NAV.button("middle", True)
    try:
        for step in range(1, 7):
            INPUT.move((start[0] + (end[0] - start[0]) * step / 6, start[1] + (end[1] - start[1]) * step / 6))
    finally:
        NAV.button("middle", False)


def reset():
    """【画布导航回归】【Ctrl复位】向已校验座席发送两次独立Ctrl按下边沿，无参数、无返回值"""
    for _ in range(2):
        with ROTATION.modifier("Control_L"):
            pass


def verify(binary, environment, root, alpha):
    """【画布导航回归】【完整流程】按alpha在environment运行binary，root保存屏幕及原图交付证据，返回结果"""
    root.mkdir()
    environment = dict(environment, XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    config = root / "config.toml"
    config.write_text('[general]\noverlay="window"\nto=""\n[notifications]\nenabled=false\n[history]\nenabled=false\n')
    path, original = LAYOUT.original(root, alpha)
    with INPUT.launch(binary, environment, config, path, root, "image") as (process, log):
        FLOW.wait_input(process, log, (200, 200))
        session.key("Space")
        baseline = snapshot(root, "initial")
        INPUT.move((320, 240))
        wheel_up(5)
        region = (80, 80, 760, 430)
        source_probes = [(190, 130), (250, 210), (430, 130), (520, 210), (610, 310)]
        INPUT.move((650, 480))
        zoomed = TOOLS.presented(root, "zoomed", lambda image: sum(image.getpixel(point) != baseline.getpixel(point) for point in source_probes) >= 3, "actual wheel did not update the frozen source pixels")
        assert ImageChops.difference(baseline.crop(region), zoomed.crop(region)).getbbox(), "actual wheel changed no frozen image pixels"
        anchor_before, anchor_after = baseline.getpixel((320, 240)), zoomed.getpixel((320, 240))
        assert max(abs(a - b) for a, b in zip(anchor_before, anchor_after)) <= 3, f"image anchor moved: {anchor_before} -> {anchor_after}"
        # 1. 【画布导航回归】【完整平移】实际原图内容沿中键位移移动，不能只移动控制框或标注缓存
        middle_drag((320, 240), (350, 260))
        probes = [(x, y) for x in (190, 250, 430, 520, 610) for y in (130, 210, 310)]
        INPUT.move((650, 480))
        panned = TOOLS.presented(root, "panned", lambda image: max(max(abs(a - b) for a, b in zip(zoomed.getpixel((x - 30, y - 20)), image.getpixel((x, y)))) for x, y in probes) <= 4, "middle pan did not submit the source pixels at full displacement")
        errors = [max(abs(a - b) for a, b in zip(zoomed.getpixel((x - 30, y - 20)), panned.getpixel((x, y)))) for x, y in probes]
        assert max(errors) <= 4, f"middle pan did not preserve source pixels at full displacement: {errors}"
        # 2. 【画布导航回归】【实际滚动条】边缘揭示控件，真实滑块拖动改变原图视图，离开后清除控件
        edge = (baseline.width - 3, baseline.height / 2)
        INPUT.move(edge)
        revealed = TOOLS.presented(root, "scrollbar-revealed", lambda image: ImageChops.difference(panned.crop((baseline.width - 14, 40, baseline.width, 440)), image.crop((baseline.width - 14, 40, baseline.width, 440))).getbbox() is not None, "overflow image did not submit its actual vertical scrollbar")
        assert ImageChops.difference(panned.crop((baseline.width - 14, 40, baseline.width, 440)), revealed.crop((baseline.width - 14, 40, baseline.width, 440))).getbbox(), "overflow image did not reveal its actual vertical scrollbar"
        NAV.button("left", True)
        try:
            INPUT.move((edge[0], edge[1] + 24))
        finally:
            NAV.button("left", False)
        INPUT.move((650, 480))
        scrolled = TOOLS.presented(root, "scrollbar-dragged", lambda image: ImageChops.difference(panned.crop(region),image.crop(region)).getbbox() is not None,"actual scrollbar drag did not update source pixels")
        assert ImageChops.difference(panned.crop(region), scrolled.crop(region)).getbbox(), "actual scrollbar drag changed no source pixels"
        reset()
        INPUT.move((650, 480))
        restored = TOOLS.presented(root,"reset",lambda image:ImageChops.difference(baseline.crop(region),image.crop(region)).getbbox() is None,"two Ctrl edges did not submit the restored source pixels")
        assert ImageChops.difference(baseline.crop(region), restored.crop(region)).getbbox() is None, "two Ctrl press edges did not restore the actual fitted source pixels"
        INPUT.move((320, 240))
        wheel_up(3)
        snapshot(root, "idle-navigated")
        time.sleep(1.05)
        idle = FLOW.NAV.SIZE.idle(process, log)
        # 3. 【画布导航回归】【原图交付】导航后从生产复制入口导出，尺寸、透明度、四角和全部原始预乘字节保持一致
        session.key("ctrl+c")
        assert process.wait(timeout=5) == 0
        data = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment, timeout=3)
        (root / "export.png").write_bytes(data)
        exported = Image.open(io.BytesIO(data)).convert("RGBA")
        assert exported.size == original.size
        assert LAYOUT.premultiplied(exported) == LAYOUT.premultiplied(original)
    result = {"alpha": alpha, "anchor_before": anchor_before, "anchor_after": anchor_after, "pan_errors": errors, "zoom_visible": True, "scrollbar_drag_visible": True, "reset_pixels_equal": True, "original_pixels_equal": True, "export_sha256": hashlib.sha256(data).hexdigest(), "idle": idle}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def run(binary, root, alphas=(255, 170)):
    """【画布导航回归】【私有矩阵】在已校验会话中测试binary两种透明度，root保存独立结果，返回不可变指纹"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(TMPDIR=str(root / "temporary"), MARK_SHOT_LANG="english", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/canvas-navigation-test")
    Path(environment["TMPDIR"]).mkdir()
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    cases = []
    with session.input_context(), FLOW.NAV.SIZE.pointer_context(backend="x11" if session.backend() is session.niri else "virtual"):
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        for alpha in alphas:
            value = verify(binary, environment, root / f"alpha-{alpha}", alpha)
            cases.append(value)
            print(json.dumps(value), flush=True)
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == fingerprint
    result = {"binary_sha256": fingerprint, "compositor": session.backend().__name__, "cases": cases, "scope": "Native image zoom, immediate middle pan, Ctrl reset, scrollbar drag, original pixels and navigated idle state. Full Qt pixel parity and all tools remain separate checks."}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    """【画布导航回归】【入口】解析独立binary和新root，输出实际结果，失败保留现场，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--alpha", type=int, choices=(255, 170), action="append")
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve(), options.alpha or (255, 170)), indent=2))


if __name__ == "__main__":
    main()
