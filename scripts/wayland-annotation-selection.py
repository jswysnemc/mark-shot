#!/usr/bin/env python3
"""【标注选框回归】【真实会话】验证完整橙色选框、删除按钮职责、撤销重做及导出隔离，不连接用户桌面"""

import argparse
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import re
import subprocess
import time

from PIL import Image, ImageChops

PROJECT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("annotation_selection_support", PROJECT / "scripts/wayland-selection-field-navigation.py")
NAV = importlib.util.module_from_spec(spec)
spec.loader.exec_module(NAV)
INPUT, session = NAV.SIZE.INPUT, NAV.session


def wait_input(process, log, point):
    """【标注选框回归】【输入就绪】等待 process 的真实键盘进入和 point 指针投递，log 提供当前进程协议证据，无返回值"""
    INPUT.move(point)
    deadline = time.monotonic() + 3.0
    while True:
        assert process.poll() is None, "editor exited before accepting pointer input"
        text = log.read_text()
        keyboard = re.search(r"wl_keyboard@\d+\.enter,", text) is not None
        events = [line for line in text.splitlines()
                  if re.search(r"wl_pointer@\d+\.(enter|motion),", line)]
        position = re.search(r", (-?[\d.]+), (-?[\d.]+)\)", events[-1]) if events else None
        if keyboard and position and all(abs(float(position[index + 1]) - value) < 2.0
                                         for index, value in enumerate(point)):
            return
        assert time.monotonic() < deadline, "editor pointer or keyboard did not enter the visible surface"
        time.sleep(0.03)


def snapshot(root, name):
    """【标注选框回归】【屏幕证据】把本次私有会话画面写入 root 的 name，返回 RGB 图像"""
    time.sleep(0.15)
    path = root / (name + ".png")
    session.screenshot(path)
    return Image.open(path).convert("RGB")


def empty_desktop(root, region):
    """【标注选框回归】【关闭动画隔离】等待私有空桌面的 region 恢复纯色，root 保存画面，返回背景颜色，超时失败"""
    INPUT.move((620, 400))
    deadline = time.monotonic() + 3.0
    while True:
        image = snapshot(root, "empty-desktop")
        channels = image.crop(region).getextrema()
        if all(low == high for low, high in channels):
            return [low for low, _ in channels]
        assert time.monotonic() < deadline, "private desktop retained a closing window or another surface"


def pixels(image, rect, color):
    """【标注选框回归】【实际颜色】在 image 的 rect 内返回接近 color 的像素位置，不用预期按钮坐标代替真实图像"""
    found = []
    for y in range(max(0, int(rect[1])), min(image.height, int(rect[3]))):
        for x in range(max(0, int(rect[0])), min(image.width, int(rect[2]))):
            if all(abs(a - b) <= 5 for a, b in zip(image.getpixel((x, y)), color)):
                found.append((x, y))
    return found


def exported(environment, process, root):
    """【标注选框回归】【原图输出】用 environment 请求当前 process 复制 PNG 并退出，保存 root 下原始字节，返回指纹和尺寸"""
    session.key("ctrl+c")
    assert process.wait(timeout=5) == 0
    data = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment, timeout=3)
    (root / "export.png").write_bytes(data)
    image = Image.open(io.BytesIO(data)).convert("RGB")
    return {"sha256": hashlib.sha256(data).hexdigest(), "size": list(image.size)}


def run(binary, root, baseline_export):
    """【标注选框回归】【完整流程】在本次私有会话验证 binary，root 保存证据，baseline_export 只记录旧构建原始导出以供比较"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"),
        TMPDIR=str(root / "temporary"), MARK_SHOT_LANG="english", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/annotation-selection-test")
    Path(environment["TMPDIR"]).mkdir()
    config = root / "config.toml"
    config.write_text('[general]\noverlay="window"\nto=""\n[notifications]\nenabled=false\n[history]\nenabled=false\n')
    report = {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "baseline_export_only": baseline_export,
              "compositor": session.backend().__name__}
    region = (160, 90, 330, 240)
    orange, red = (251, 146, 60), (239, 68, 68)
    with session.input_context(), NAV.SIZE.pointer_context(backend="x11" if session.backend() is session.niri else "virtual"):
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        report["background_rgb"] = empty_desktop(root, region)
        with INPUT.launch(binary, environment, config, None, root, "capture") as (process, log):
            wait_input(process, log, (80, 80))
            INPUT.drag((80, 80), (500, 380))
            session.key("r")
            INPUT.drag((180, 130), (280, 210))
            session.key("v")
            INPUT.click((230, 170))
            if baseline_export:
                report["export"] = exported(environment, process, root)
            else:
                selected = snapshot(root, "selected")
                handles = pixels(selected, region, orange)
                assert len(handles) > 150, f"selected annotation has no complete orange controls: {len(handles)}"
                button_pixels = pixels(selected, (282, 92, 327, 127), red)
                assert len(button_pixels) > 80, f"selected annotation has no red delete button: {len(button_pixels)}"
                point = (sum(x for x, _ in button_pixels) / len(button_pixels), sum(y for _, y in button_pixels) / len(button_pixels))
                INPUT.move(point)
                before = snapshot(root, "before-right")
                NAV.button("right", True)
                NAV.button("right", False)
                after = snapshot(root, "after-right")
                assert ImageChops.difference(before.crop(region), after.crop(region)).getbbox() is None, "right button changed the annotation or its selection"
                NAV.button("left", True)
                try:
                    deleted = snapshot(root, "deleted-while-held")
                    assert not pixels(deleted, region, orange), "left press did not remove selected annotation controls"
                    assert not pixels(deleted, (282, 92, 327, 127), red), "delete button remained after deleting its annotation"
                    NAV.button("right", False)
                finally:
                    NAV.button("left", False)
                session.key("ctrl+z")
                restored = snapshot(root, "undo-restored-selection")
                assert ImageChops.difference(before.crop(region), restored.crop(region)).getbbox() is None, "undo did not restore original annotation and selection"
                session.key("ctrl+y")
                assert not pixels(snapshot(root, "redo-deleted"), region, orange), "redo did not delete restored annotation"
                session.key("ctrl+z")
                session.key("ctrl+shift+l")
                dark = snapshot(root, "dark-selection")
                assert len(pixels(dark, region, orange)) > 150 and len(pixels(dark, (282, 92, 327, 127), red)) > 80
                session.key("ctrl+shift+l")
                # 1. 【标注选框回归】【静止资源】隐藏界面后不继续更新按钮或图像，普通截图保持单线程
                session.key("Space")
                report["idle"] = NAV.SIZE.idle(process, log)
                session.key("Space")
                report["delete_point"] = point
                report["orange_pixels"] = len(handles)
                report["delete_pixels"] = len(button_pixels)
                report["right_button_inert"] = True
                report["undo_restored_selection"] = True
                report["export"] = exported(environment, process, root)
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    """【标注选框回归】【入口】解析构建、新目录及旧导出标记，打印证据路径，失败保留日志"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--baseline-export", action="store_true")
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve(), options.baseline_export), indent=2))


if __name__ == "__main__":
    main()
