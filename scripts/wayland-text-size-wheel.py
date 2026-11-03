#!/usr/bin/env python3
"""【字号滚轮回归】【实际输入】在私有 Wayland 会话验证平台刻度、字号修改与连续撤销，不连接真实桌面"""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import tomllib

from PIL import Image
import wayland_test_session as session
from wayland_text_property_locator import font_button, font_popup
from wayland_test_keyboard import hold


def capture(process, root, name):
    """【字号滚轮回归】【画面记录】把 process 的 name 画面保存到 root，返回 RGB 图，进程提前退出时报错"""
    time.sleep(0.15)
    if process.poll() is not None:
        raise AssertionError(f"editor exited unexpectedly: {process.returncode}")
    path = root / f"{name}.png"
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def click(point):
    """【字号滚轮回归】【按钮点击】只向私有会话的 point 发送完整左键点击，无返回值"""
    session.control("click", "--logical", *map(round, point))
    time.sleep(0.04)


def scroll(point, notches=1):
    """【字号滚轮回归】【刻度发送】向私有 point 连续发送 notches 个向上刻度，无返回值，不用键盘模拟滚轮"""
    session.control("scroll", "--to", *map(round, point), "--dy", -1, "--notches", notches)


def size(process, root, environment, name, expected, replacement=None, close=True):
    """【字号滚轮回归】【字号读取】通过真实字段验证 expected，replacement 可替换字号，close 控制关闭；返回字体弹层矩形"""
    click(font_button(capture(process, root, name + "-closed")))
    rect = font_popup(capture(process, root, name + "-open"))
    session.key("Tab")
    session.key("ctrl+a")
    session.key("ctrl+c")
    session.wait_clipboard(environment, str(expected))
    if replacement is not None:
        session.paste_text(str(replacement))
    if close:
        session.key("Escape")
    return rect


def qt_axes(reference, root, environment):
    """【滚轮协议回归】【平台对照】以 reference 接收真实滚轮，在 root 保存角度和像素，environment 必须属于私有会话；返回 Qt 原始事件"""
    prefix = root / "qt-axis"
    with (root / "qt-axis.log").open("w") as log:
        socket = Path(environment["XDG_RUNTIME_DIR"]) / environment["WAYLAND_DISPLAY"]
        process = subprocess.Popen([str(reference), str(prefix), "axis"], env=dict(environment, QT_QPA_PLATFORM="wayland", WAYLAND_DISPLAY=str(socket), QT_ACCESSIBILITY="0"), stderr=log)
        try:
            deadline = time.monotonic() + 8
            while True:
                matching = [item for item in session.windows() if item["pid"] == process.pid]
                if len(matching) == 1:
                    item = matching[0]
                    break
                if len(matching) > 1 or time.monotonic() >= deadline or process.poll() is not None:
                    raise AssertionError(f"Qt axis window did not appear unambiguously: {matching}; exit={process.poll()}")
                time.sleep(0.04)
            item = session.size_window(item, 600, 400)
            left, top = item["floating_pos"]
            point = (left + 300, top + 200)
            scroll(point)
            time.sleep(0.1)
            scroll(point, 2)
            time.sleep(0.15)
            path = Path(str(prefix) + "-axis.json")
            events = json.loads(path.read_text())
            angles = [event["angle"][1] for event in events]
            if sum(angles) != 360 or any(event["angle"][0] != 0 or event["pixels"][0] != 0 for event in events):
                raise AssertionError(f"unexpected real Qt Wayland wheel conversion: {events}")
            return events
        finally:
            if process.poll() is None:
                try:
                    process.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    process.terminate()
                    process.wait(timeout=3)


def native(binary, root, environment):
    """【字号滚轮回归】【完整场景】用 binary 验证工具默认值、控件和已有文字，root 保存证据；返回实际数值、输出与空闲资源"""
    with tempfile.TemporaryDirectory(prefix="mark-shot-size-wheel-") as directory:
        private = Path(directory)
        config = private / "config/mark-shot"
        state = private / "state/mark-shot"
        config.mkdir(parents=True)
        state.mkdir(parents=True)
        (config / "config.toml").write_text('[general]\nto = "s"\n[notifications]\nenabled = false\n')
        (state / "tools.toml").write_text('magnifier = false\nfont_size = 20.0\nfont_size_unit = "pt"\ntext_family = "Sans Serif"\nbold = true\nitalic = false\n')
        environment = dict(environment, XDG_CONFIG_HOME=str(private / "config"), XDG_DATA_HOME=str(private / "data"), XDG_STATE_HOME=str(private / "state"))
        source = private / "source.png"
        Image.new("RGB", (1600, 900), (30, 34, 38)).save(source)
        with (root / "process.log").open("w") as log:
            process = subprocess.Popen([str(binary), "--fullscreen", "--fullscreen-default-tool", "text", "--default-color", "#FF4D4D", "--to", "s", "--output", str(private / "export"), str(source)], env=environment, stderr=log)
            try:
                initial = capture(process, root, "initial")
                session.wait_single_thread(process.pid)
                canvas = (initial.width * 0.7, initial.height * 0.6)
                # 1. 【字号滚轮回归】【默认入口】两次独立完整刻度保留半点余量，数字框和滑条使用各自步进
                scroll(canvas)
                size(process, root, environment, "default-one", 21)
                scroll(canvas)
                rect = size(process, root, environment, "default-two", 23, close=False)
                field = (rect[0] + 140, rect[1] + 339)
                scroll(field)
                session.key("ctrl+a")
                session.key("ctrl+c")
                session.wait_clipboard(environment, "24")
                with hold("ctrl"):
                    scroll(field)
                session.key("ctrl+a")
                session.key("ctrl+c")
                session.wait_clipboard(environment, "34")
                capture(process, root, "spin-control")
                session.key("Escape")
                button = font_button(capture(process, root, "slider-before"))
                scroll((button[0] - 335, button[1]))
                size(process, root, environment, "slider-one", 37, replacement=20)
                # 2. 【字号滚轮回归】【已有文字】结束正文输入再切换选择工具，真实滚轮修改选中对象并同步默认值
                point = (round(initial.width * 0.58), round(initial.height * 0.52))
                click(point)
                session.paste_text("Wheel text")
                click((point[0] - 100, point[1] + 140))
                session.key("v")
                click((point[0] + 12, point[1] + 12))
                size(process, root, environment, "selected", 20)
                started = time.monotonic()
                scroll(canvas)
                scroll(canvas)
                continuous_span = time.monotonic() - started
                if continuous_span >= 0.35:
                    raise AssertionError(f"two independent wheel events exceeded debounce interval: {continuous_span}")
                size(process, root, environment, "continuous", 23)
                session.key("ctrl+z")
                size(process, root, environment, "continuous-undo", 20)
                # 3. 【字号滚轮回归】【分组期限】跨越防抖期限后，两次滚轮分别撤销，字号余量不因等待而清除
                scroll(canvas)
                time.sleep(0.45)
                scroll(canvas)
                size(process, root, environment, "separated", 23)
                session.key("ctrl+z")
                size(process, root, environment, "separated-undo-one", 21)
                session.key("ctrl+z")
                size(process, root, environment, "separated-undo-two", 20)
                capture(process, root, "final")
                before = session.wait_single_thread(process.pid)
                time.sleep(1)
                after = session.resources(process.pid)
                if before["cpu_ticks"] != after["cpu_ticks"] or after["threads"] != 1:
                    raise AssertionError(f"wheel timer kept idle editor active: {before}, {after}")
                session.key("Return")
                process.wait(timeout=6)
                files = list((private / "export").glob("*.png"))
                if process.returncode or len(files) != 1:
                    raise AssertionError(f"export failed or ambiguous: {process.returncode}, {files}")
                saved = tomllib.loads((state / "tools.toml").read_text())
                if saved["font_size"] != 23:
                    raise AssertionError(f"undo reverted synchronized default: {saved}")
                shutil.copyfile(files[0], root / "export.png")
                shutil.copyfile(state / "tools.toml", root / "tools.toml")
                if (private / "data/mark-shot/models").exists():
                    raise AssertionError("wheel editing initialized OCR models")
                return {"default_wheel_points": [20, 21, 23], "spin_points": [24, 34], "slider_points": 37, "continuous_event_span_ms": round(continuous_span * 1000, 3), "continuous_undo": [23, 20], "separated_undo": [23, 21, 20], "saved_default_points": 23, "idle_before": before, "idle_after": after}
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=3)


def main():
    """【字号滚轮回归】【程序入口】解析构建路径和证据目录，验证私有会话后执行场景，保存同一构建指纹，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--qt-reference", type=Path)
    parser.add_argument("--artifacts", type=Path, required=True)
    options = parser.parse_args()
    root = options.artifacts.resolve()
    root.mkdir(parents=True, exist_ok=False)
    binary = options.binary.resolve()
    environment = session.environment()
    result = {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest()}
    if options.qt_reference:
        result["qt_axis_events"] = qt_axes(options.qt_reference.resolve(), root, environment)
    result["native"] = native(binary, root, environment)
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
