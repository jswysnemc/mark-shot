#!/usr/bin/env python3
"""【工具栏回归】【原版布局】在隔离 Wayland 桌面验证双工具栏尺寸、鼠标职责与握柄拖动"""

import argparse
import hashlib
import io
import json
from pathlib import Path
import subprocess
import time

from PIL import Image
import wayland_test_session as session


MAIN = ("move", "pick", None, "pen", "line", "highlighter", None,
        "rectangle", "circle", "marker", "arrow", None,
        "text", "number", "filter", "magnifier", "ruler", "laser", None,
        "clear", "undo", "redo")
SIDE = ("scope", None, "open", "extensions", "scroll", "pin", "ocr", "translate", None,
        "copy", "save", "upload", None, "settings", None, "close")


def layout(items, button, margin):
    """【工具栏回归】【原版几何】按 Qt 的 items、button、margin 和十像素握柄返回总长与按钮中心"""
    offset = margin + 10 + 2
    points = {}
    for item in items:
        length = button if item else 1
        if item:
            points[item] = offset + length / 2
        offset += length + 2
    return offset - 2 + margin, points


def key(value):
    """【工具栏回归】【键盘输入】发送 value 原生按键并等候处理，不切换 Xvfb 窗口焦点"""
    session.control("key", value)
    time.sleep(0.07)


def click(point, button="left", count=1):
    """【工具栏回归】【鼠标输入】按 button 在 point 点击 count 次；不把右键转换为左键"""
    session.control("click", "--logical", *map(round, point), "--button", button, "--count", count)
    time.sleep(0.08)


def capture(artifacts, name):
    """【工具栏回归】【图像证据】保存 artifacts 下 name 截图并返回 RGB 图像；证据不复用历史目录"""
    path = artifacts / f"{name}.png"
    session.screenshot(path)
    return Image.open(path).convert("RGB")


def alive(process):
    """【工具栏回归】【取消隔离】要求 process 仍然运行；鼠标右键误执行输出或关闭时立即失败"""
    if process.poll() is not None:
        raise AssertionError(f"the editor unexpectedly exited: {process.returncode}")


def active(image, point):
    """【工具栏回归】【选中状态】在 image 的 point 周边统计青色选中背景；返回是否存在足够强调像素"""
    x, y = map(round, point)
    pixels = image.load()
    count = sum(pixels[left, top][1] - pixels[left, top][0] > 18
                for top in range(y - 12, y + 13) for left in range(x - 12, x + 13))
    return count >= 80


def run(binary, artifacts):
    """【工具栏回归】【完整输入】用 binary 执行原版双栏职责，将构建指纹与真实输入证据写入全新 artifacts"""
    artifacts.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    config = artifacts / "config.toml"
    config.write_text('[general]\nto = ""\noverlay = "layer"\n[history]\nenabled = false\n[notifications]\nenabled = false\n')
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    width, main_cells = layout(MAIN, 30, 6)
    height, side_cells = layout(SIDE, 28, 4)
    # 1. 【工具栏回归】【几何约束】Qt 默认主栏 610x42，副栏 36x390，选区下侧与右侧相距八像素
    if (width, height) != (610, 390):
        raise AssertionError("the independent Qt layout fixture changed")
    main_origin = (520 - width / 2, 448)
    side_origin = (888, 305 - height / 2)
    main = {name: (main_origin[0] + value, main_origin[1] + 21) for name, value in main_cells.items()}
    side = {name: (side_origin[0] + 18, side_origin[1] + value) for name, value in side_cells.items()}
    with (artifacts / "process.log").open("w") as errors:
        process = subprocess.Popen([str(binary), "--config", str(config), "--nonotify"], env=environment,
                                   stdout=subprocess.DEVNULL, stderr=errors)
        try:
            time.sleep(0.4)
            session.control("move", "--logical", 160, 170)
            time.sleep(0.08)
            session.control("drag", "--from", 160, 170, "--to", 880, 440, "--steps", 12)
            time.sleep(0.2)
            initial = capture(artifacts, "selection-dual-toolbar")
            if not active(initial, main["move"]):
                raise AssertionError("the original move tool was not selected at the expected toolbar position")
            if not active(initial, side["save"]):
                raise AssertionError("the secondary toolbar was not rendered at the independent Qt fixture position")

            # 2. 【工具栏回归】【按钮隔离】右键点击复制或关闭不得执行按钮，也不得落到下方选区
            session.control("copy", "toolbar sentinel")
            click(side["copy"], "right")
            click(side["close"], "right")
            alive(process)
            if session.read_clipboard(environment) != "toolbar sentinel":
                raise AssertionError("a right click activated the Copy button")
            if not active(capture(artifacts, "right-buttons-isolated"), main["move"]):
                raise AssertionError("a right click on an action button leaked through to the canvas")
            click(main["pen"])
            session.control("move", "--logical", 400, 250)
            if not active(capture(artifacts, "pen-selected"), main["pen"]):
                raise AssertionError("left click did not select the original Pen button")
            click((400, 250), "right")
            alive(process)
            if not active(capture(artifacts, "right-click-pick"), main["pick"]):
                raise AssertionError("editing right click did not return to annotation selection")

            # 3. 【工具栏回归】【握柄拖动】只从十像素握柄移动主栏，副栏与选区位置保持不变
            grip = (main_origin[0] + 11, main_origin[1] + 21)
            session.control("move", "--logical", *map(round, grip))
            time.sleep(0.08)
            session.control("drag", "--from", *map(round, grip), "--to", round(grip[0] + 70), round(grip[1] + 90), "--steps", 12)
            time.sleep(0.15)
            moved = {name: (point[0] + 70, point[1] + 90) for name, point in main.items()}
            if not active(capture(artifacts, "main-grip-moved"), moved["pick"]):
                raise AssertionError("dragging the toolbar grip did not preserve the selected tool and offset")
            click(side["copy"], "right")
            alive(process)
            click(side["copy"])
            process.wait(timeout=3)
            if process.returncode != 0:
                raise AssertionError("left click Copy did not complete the screenshot")
            png = subprocess.check_output(["wl-paste", "--no-newline", "--type", "image/png"], env=environment, timeout=3)
            copied = Image.open(io.BytesIO(png))
            if abs(copied.width - 720 * 1.2) > 2 or abs(copied.height - 270 * 1.2) > 2:
                raise AssertionError(f"toolbar input changed the screenshot selection: {copied.size}")
            (artifacts / "copied-selection.png").write_bytes(png)

            # 4. 【工具栏回归】【选择取消】新会话没有选区时右键关闭，不需要第二次右键
            cancel = subprocess.Popen([str(binary), "--config", str(config), "--nonotify"], env=environment,
                                      stdout=subprocess.DEVNULL, stderr=errors)
            try:
                time.sleep(0.35)
                click((400, 250), "right")
                cancel.wait(timeout=3)
                if cancel.returncode != 0:
                    raise AssertionError("selection-stage right click did not cancel cleanly")
            finally:
                if cancel.poll() is None:
                    cancel.terminate()
                    cancel.wait(timeout=3)
            result = {"sha256": fingerprint, "main_size": [width, 42], "side_size": [36, height],
                      "right_button_isolation": True, "left_button_activation": True,
                      "editing_right_click_pick": True, "selection_right_click_cancel": True,
                      "grip_drag": True, "source_crop_size": list(copied.size)}
            (artifacts / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)
    print("Original dual toolbar geometry, mouse responsibilities, grip movement and copy passed")


def main():
    """【工具栏回归】【命令入口】读取二进制与证据目录参数；失败保持非零退出码，成功记录构建指纹"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, default=Path("target/wayland-toolbar"))
    options = parser.parse_args()
    run(options.binary.resolve(), options.artifacts.resolve())


if __name__ == "__main__":
    main()
