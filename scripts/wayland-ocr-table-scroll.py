#!/usr/bin/env python3
"""【OCR 表格滚动】【真实验证】私有 Wayland 中验证实际刻度、条带、拖动、中键和重复终止，不连接用户桌面"""

import argparse
import hashlib
import json
from pathlib import Path
import select
import struct
import subprocess
import time

import wayland_text_format_session as session
from wayland_ocr_table_scroll_checks import frame, front, horizontal_pixels, point, reference, vertical_pixel
from wayland_ocr_translation_checks import idle
from wayland_ocr_window_layout import tab_point
from wayland_text_pointer import CURRENT, click, pointer_context
from wayland_virtual_pointer import BTN_LEFT, BTN_MIDDLE, BTN_RIGHT, transform_for_pointer
from wayland_window_input_session import geometry, screenshot


def move(pointer, position):
    """【OCR 表格滚动】【私有定位】用 pointer 将 position 转为本次输出的真实协议坐标，无返回值，禁止访问外部会话"""
    logical = CURRENT.get()[1]
    coordinates = transform_for_pointer(position[0] - logical["x"], position[1] - logical["y"], logical["width"], logical["height"], logical.get("pointer_transform", logical["transform"]))
    pointer.move_abs(*coordinates)
    time.sleep(0.03)


def wheel(pointer, value, discrete=0, axis=0, source=0):
    """【OCR 表格滚动】【协议刻度】用 pointer 发送 value、discrete、axis、source 的真实滚轮，刻度与无刻度位移分开，返回请求记录"""
    pointer.connection.send(pointer.pointer, 5, struct.pack("=I", source))
    if discrete:
        pointer.connection.send(pointer.pointer, 7, struct.pack("=IIii", pointer.timestamp(), axis, round(value * 256), discrete))
    else:
        pointer.connection.send(pointer.pointer, 3, struct.pack("=IIi", pointer.timestamp(), axis, round(value * 256)))
    pointer.connection.send(pointer.pointer, 4)
    pointer.connection.roundtrip()
    time.sleep(0.05)
    return {"absolute": value, "discrete": discrete, "axis": axis, "source": source}


class Checks:
    """【OCR 表格滚动】【完整断言】保持本次窗口与原版期望状态，所有偏移通过实际复制或像素确认"""

    def __init__(self, pointer, window, environment, root, theme):
        """【OCR 表格滚动】【状态初始化】按 pointer、window、environment、root、theme 创建断言器，不改变应用状态"""
        self.pointer, self.window, self.environment = pointer, window, environment
        self.root, self.theme = root, theme
        self.reference = reference(theme)
        self.state = self.reference["initial"]
        self.clipboard, self.pixels, self.requests = {}, {}, []

    def copy_front(self, name, state):
        """【OCR 表格滚动】【实际数据】按 name、state 选择当前原版期望位置并复制完整单元格，返回文字；失败不只比较滑块外观"""
        position, expected = front(self.window, state)
        click(position)
        session.key("F2")
        session.key("ctrl+a")
        session.key("ctrl+c")
        session.wait_clipboard(self.environment, expected)
        session.key("Escape")
        self.clipboard[name] = expected
        self.state = state
        return expected

    def reset(self):
        """【OCR 表格滚动】【真实复位】依次拖动当前真实滑块到两条轨道起点，不直接修改滚动值，无返回值"""
        for axis in ("horizontal", "vertical"):
            bar = self.state[axis]
            start = point(self.window, self.state, axis, "slider")
            groove = point(self.window, self.state, axis, "groove")
            if axis == "horizontal":
                end = (self.window["x"] + 8 + bar["rect"][0] + bar["controls"]["groove"][0], groove[1])
            else:
                end = (groove[0], self.window["y"] + 75 + bar["rect"][1] + bar["controls"]["groove"][1])
            move(self.pointer, start)
            self.pointer.press(BTN_LEFT)
            move(self.pointer, end)
            self.pointer.release(BTN_LEFT)
        self.state = self.reference["initial"]

    def drag_max(self, axis, right_during=False):
        """【OCR 表格滚动】【所属拖动】按 axis 拖到原版末端，right_during 验证右键不能抢占左键事务，无返回值"""
        start = point(self.window, self.state, axis, "slider")
        groove = self.state[axis]["controls"]["groove"]
        bounds = self.state[axis]["rect"]
        end = list(start)
        index = 0 if axis == "horizontal" else 1
        end[index] = self.window["x" if index == 0 else "y"] + (8 if index == 0 else 75) + bounds[index] + groove[index] + groove[index + 2] - 1
        move(self.pointer, start)
        self.pointer.press(BTN_LEFT)
        if right_during:
            self.pointer.press(BTN_RIGHT)
            self.pointer.release(BTN_RIGHT)
        move(self.pointer, tuple(end))
        self.pointer.release(BTN_LEFT)
        self.copy_front(axis + "-drag-max", self.reference[axis + "-maximum"])


def run(binary, root, theme):
    """【OCR 表格滚动】【完整场景】按 binary、root、theme 运行实际刻度和滚动条动作，保存同构建证据，无返回值"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"), MARK_SHOT_LANG="en")
    config = root / "config/mark-shot/config.toml"
    config.parent.mkdir(parents=True)
    config.write_text("[history]\nenabled = false\n[notifications]\nenabled = false\n[theme]\nbackground = " + json.dumps("#F8FAFC" if theme == "light" else "#0F1117E6") + "\n")
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    log = root / "process.log"
    with session.input_context(), pointer_context() as pointer, log.open("w") as output:
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        process = subprocess.Popen([str(binary), "--ocr-result", "--nonotify"], env=dict(environment, WAYLAND_DEBUG="client"), stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=output)
        try:
            process.stdin.write(frame())
            process.stdin.close()
            assert select.select([process.stdout], [], [], 8)[0] and process.stdout.readline() == b"\n"
            window = geometry(process.pid, environment)
            click(tab_point(window, "Table"))
            checks = Checks(pointer, window, environment, root, theme)
            # 1. 【OCR 表格滚动】【实际刻度】通过完整单元格复制检查三项刻度及四次半项累积，不能只观察滑块
            move(pointer, front(window, checks.state)[0])
            checks.requests.append(wheel(pointer, 15, 1))
            checks.copy_front("discrete-three-rows", checks.reference["wheel-down"])
            checks.reset()
            for step in range(1, 5):
                move(pointer, front(window, checks.state)[0])
                checks.requests.append(wheel(pointer, 20 / 12))
                checks.copy_front("partial-" + str(step), checks.reference["partial-" + str(step)])
            checks.reset()
            # 2. 【OCR 表格滚动】【原版滑块】横向常态和按下条带、纵向隐藏和悬停都比较实际像素
            move(pointer, front(window, checks.state)[0])
            image = screenshot(root, "initial-bars")
            checks.pixels["horizontal-normal"] = horizontal_pixels(image, window, checks.state, theme)
            checks.pixels["vertical-hidden"] = vertical_pixel(image, window, checks.state, theme, False)
            move(pointer, point(window, checks.state, "vertical", "slider"))
            checks.pixels["vertical-hover"] = vertical_pixel(screenshot(root, "vertical-hover"), window, checks.state, theme, True)
            move(pointer, point(window, checks.state, "horizontal", "slider"))
            pointer.press(BTN_LEFT)
            checks.pixels["horizontal-pressed"] = horizontal_pixels(screenshot(root, "horizontal-pressed"), window, checks.state, theme, True)
            pointer.release(BTN_LEFT)
            # 3. 【OCR 表格滚动】【鼠标职责】完整左键拖动、中键绝对定位、右键不抢占，最终偏移使用原版末端记录
            checks.drag_max("horizontal", right_during=True)
            checks.reset()
            checks.drag_max("vertical")
            checks.reset()
            move(pointer, point(window, checks.state, "vertical", "add_page"))
            pointer.click(BTN_MIDDLE)
            checks.copy_front("middle-absolute-position", checks.reference["vertical-middle-page"])
            checks.reset()
            # 4. 【OCR 表格滚动】【重复停止】按住轨道后停在原版目标位置，释放后不得留下周期重绘或定时线程
            move(pointer, point(window, checks.state, "vertical", "add_page"))
            pointer.press(BTN_LEFT)
            time.sleep(0.75)
            pointer.release(BTN_LEFT)
            checks.copy_front("page-repeat-target", checks.reference["vertical-page-repeat-stopped"])
            resources = idle(process.pid, log)
            assert not (root / "data/mark-shot/models").exists()
            session.key("Escape")
            assert process.wait(timeout=4) == 0
            assert hashlib.sha256(binary.read_bytes()).hexdigest() == digest
            (root / "result.json").write_text(json.dumps({"sha256": digest, "theme": theme, "compositor": "sway" if session.backend() is session.sway else "niri",
                "clipboard": checks.clipboard, "pixels": checks.pixels, "requests": checks.requests, "resources": resources,
                "no_ocr_or_external_service": True, "owned_window_exited": True}, indent=2) + "\n")
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)


def main():
    """【OCR 表格滚动】【入口】解析私有二进制、目录及主题后执行场景，失败返回非零状态，无参数"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--theme", choices=("dark", "light"), default="dark")
    args = parser.parse_args()
    run(args.binary.resolve(), args.root.resolve(), args.theme)


if __name__ == "__main__":
    main()
