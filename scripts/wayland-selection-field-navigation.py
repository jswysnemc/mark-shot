#!/usr/bin/env python3
"""【选区字段回归】【真实会话】验证词语导航、多击选择、剪切、非法删除和鼠标释放归属"""

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import time
from wayland_text_pointer import BUTTONS

PROJECT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("selection_size_workflow", PROJECT / "scripts/wayland-selection-size.py")
SIZE = importlib.util.module_from_spec(spec)
spec.loader.exec_module(SIZE)
session = SIZE.session


def context(method):
    """【选区字段回归】【当前状态】按 method 返回最近原生字段的正文、光标及锚点，缺失状态视为失败"""
    if not method.states:
        raise AssertionError("native field has no surrounding state")
    state = method.states[-1]
    return {key: state[key] for key in ("active", "text", "cursor", "anchor")}


def changed(method, previous, text, cursor=None, anchor=None):
    """【选区字段回归】【完成等待】等待 method 晚于 previous 的 text 状态，按需核对 cursor 与 anchor，返回完整上下文"""
    method.wait(lambda line: line.startswith("state\t") and method.states[-1]["serial"] > previous
                and method.states[-1]["active"] and method.states[-1]["text"] == text
                and (cursor is None or method.states[-1]["cursor"] == cursor)
                and (anchor is None or method.states[-1]["anchor"] == anchor))
    return context(method)


def key(method, value, text, cursor, anchor, held=False):
    """【选区字段回归】【实际按键】向 method 发送 value，held 保留鼠标按下；核对 text、cursor 与 anchor 后返回状态"""
    previous = method.states[-1]["serial"]
    pointer, _, _, x11 = SIZE.INPUT.CURRENT.get()
    if held and pointer is None:
        # 1. 【选区字段回归】【按钮保留】清除修饰键的工具选项也会释放鼠标，持有期间直接发送当前按键
        subprocess.run(["xdotool", "key", value], env=x11[0], check=True, timeout=3)
    else:
        session.key(value)
    return changed(method, previous, text, cursor, anchor)


def click(method, point, text, cursor=None, anchor=None):
    """【选区字段回归】【实际点击】向 point 发送完整左键点击，按 text、cursor 与 anchor 等待 method，返回状态"""
    previous = method.states[-1]["serial"]
    SIZE.INPUT.click(point)
    return changed(method, previous, text, cursor, anchor)


def button(name, pressed):
    """【选区字段回归】【持有按钮】用当前已校验设备发送 name 的 pressed 状态，返回空值，不创建其他输入来源"""
    pointer, _, _, x11 = SIZE.INPUT.CURRENT.get()
    if pointer is None:
        code = {"left": "1", "right": "3", "middle": "2"}[name]
        subprocess.run(["xdotool", "mousedown" if pressed else "mouseup", code], env=x11[0], check=True, timeout=3)
    elif pressed:
        pointer.press(BUTTONS[name])
    else:
        pointer.release(BUTTONS[name])


def wheel_up():
    """【选区字段回归】【真实滚轮】在当前校验设备向上发送一个滚轮动作，无参数、无返回值"""
    pointer, _, _, x11 = SIZE.INPUT.CURRENT.get()
    if pointer is None:
        subprocess.run(["xdotool", "click", "4"], env=x11[0], check=True, timeout=3)
    else:
        pointer.scroll(dy=-1.0)


def clipboard(environment, expected):
    """【选区字段回归】【剪贴板】在 environment 核对完整 expected 文本，返回空值，不读取其他会话"""
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        value = subprocess.run(["wl-paste", "--no-newline"], env=environment, capture_output=True, text=True, timeout=2)
        if value.returncode == 0 and value.stdout == expected:
            return
        time.sleep(0.01)
    raise AssertionError(f"field clipboard did not contain {expected!r}: {value.stdout!r}")


def unchanged(method, before):
    """【选区字段回归】【无动作验证】读取 method 已到达与等待期内的真实状态，核对 before 后返回上下文，不能以旧缓存通过检查"""
    deadline = time.monotonic() + 0.12
    while time.monotonic() < deadline:
        line = method.read(max(0, deadline - time.monotonic()))
        after = context(method)
        if after != before:
            raise AssertionError(f"field changed on an inactive shortcut or unrelated button: {before} -> {after}")
        if line is None:
            break
    return context(method)


def paste(method, text):
    """【选区字段回归】【字段替换】聚焦全选后粘贴 text，等待 method 正文与末尾光标，返回状态"""
    session.key("ctrl+r")
    previous = method.states[-1]["serial"]
    session.paste_text(text)
    return changed(method, previous, text, len(text), len(text))


def run(binary, root):
    """【选区字段回归】【执行流程】在私有会话测试 binary，root 保存原生状态、协议、截图和 PNG，返回验证记录"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"),
                       TMPDIR=str(root / "temporary"), MARK_SHOT_LANG="english", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/mark-shot-test")
    Path(environment["TMPDIR"]).mkdir()
    config = root / "config.toml"
    config.write_text('[general]\noverlay="window"\nto=""\n[notifications]\nenabled=false\n[history]\nenabled=false\n[export]\nimageFrame=false\n')
    report = {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "compositor": session.backend().__name__}
    with session.input_context(), SIZE.pointer_context(backend="x11" if session.backend() is session.niri else "virtual"), SIZE.input_method(environment, root / "input-method") as method:
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        with SIZE.INPUT.launch(binary, environment, config, None, root, "capture") as (process, log):
            SIZE.INPUT.drag((0, 0), (300, 180))
            session.key("ctrl+r")
            method.wait(lambda line: line.startswith("state\t") and method.states[-1]["active"])
            report["capture_order"] = SIZE.INPUT.capture_before_overlay(log)
            states = {}
            # 1. 【选区字段回归】【词语按键】通过真实原生周边文本核对正号边界、反向选择和独立删除撤销
            states["draft"] = paste(method, "+1200")
            states["word_left"] = key(method, "ctrl+Left", "+1200", 1, 1)
            states["word_right"] = key(method, "ctrl+Right", "+1200", 5, 5)
            states["reverse_word"] = key(method, "ctrl+shift+Left", "+1200", 1, 5)
            states["word_deleted"] = key(method, "ctrl+Backspace", "+", 1, 1)
            states["word_undone"] = key(method, "ctrl+z", "+1200", 1, 5)
            key(method, "End", "+1200", 5, 5)
            # 2. 【选区字段回归】【多击选择】使用同一真实指针连续点击，双击选数字段，三击选全文
            point = (6 + 6 + 47 / 2, 6 + 13)
            states["single"] = click(method, point, "+1200")
            assert states["single"]["cursor"] == states["single"]["anchor"]
            states["double"] = click(method, point, "+1200", 5, 1)
            states["triple"] = click(method, point, "+1200", 5, 0)
            states["fourth"] = click(method, point, "+1200", 5, 1)
            SIZE.snapshot(root, "word-selected")
            # 3. 【选区字段回归】【剪切归属】Ctrl+Shift+Delete 保留字段，Shift+Delete 执行真实剪切，撤销保持原选择
            session.key("ctrl+c")
            clipboard(environment, "1200")
            before = context(method)
            session.key("ctrl+shift+Delete")
            states["extra_delete_modifiers"] = unchanged(method, before)
            clipboard(environment, "1200")
            states["cut"] = key(method, "shift+Delete", "+", 1, 1)
            clipboard(environment, "1200")
            states["cut_undone"] = key(method, "ctrl+z", "+1200", 5, 1)
            # 4. 【选区字段回归】【非法删除】删除首个数字会留下非法分组符，正文保留而新词语选择继续存在
            paste(method, "1,200")
            key(method, "Home", "1,200", 0, 0)
            states["rejected_delete"] = key(method, "ctrl+Delete", "1,200", 1, 0)
            session.key("ctrl+c")
            clipboard(environment, "1")
            key(method, "End", "1,200", 5, 5)
            before = context(method)
            SIZE.INPUT.click(point, "right")
            states["right_unchanged"] = unchanged(method, before)
            key(method, "Home", "1,200", 0, 0)
            # 5. 【选区字段回归】【按钮持有】右键释放不结束左键拖选，继续移动到字段左侧仍能选择全部文字
            point_right = (12 + 47 - 8, point[1])
            SIZE.INPUT.move(point_right)
            previous = method.states[-1]["serial"]
            button("left", True)
            try:
                changed(method, previous, "1,200", 5, 5)
                before = context(method)
                button("right", True)
                button("right", False)
                states["right_release_unchanged"] = unchanged(method, before)
                previous = method.states[-1]["serial"]
                SIZE.INPUT.move((6, point[1]))
                states["continued_left_drag"] = changed(method, previous, "1,200", 0, 5)
            finally:
                button("left", False)
            SIZE.snapshot(root, "drag-selected")
            session.key("Escape")
            # 6. 【选区字段回归】【改写持有】上下键和滚轮改写文字后，继续操作同一字段，保持左键抓取
            for operation in ("arrow", "wheel"):
                paste(method, "+120")
                key(method, "Home", "+120", 0, 0)
                SIZE.INPUT.click(point_right, "right")
                SIZE.INPUT.move(point_right)
                previous = method.states[-1]["serial"]
                button("left", True)
                try:
                    changed(method, previous, "+120", 4, 4)
                    expected = "121"
                    if operation == "arrow":
                        states["arrow_replacement"] = key(method, "Up", "121", 3, 0, held=True)
                    else:
                        previous = method.states[-1]["serial"]
                        offset = len(log.read_text())
                        wheel_up()
                        # 7. 【选区字段回归】【协议刻度】私有 X11 运输可在按下与释放各发送一帧，按实际送达事件核对步长
                        deadline = time.monotonic() + 3
                        while time.monotonic() < deadline:
                            events = re.findall(r"<- wl_pointer@\d+\.axis, \(\d+, 0, -[\d.]+\)", log.read_text()[offset:])
                            if events:
                                break
                            time.sleep(0.01)
                        else:
                            raise AssertionError("the private wheel transport did not deliver a vertical event")
                        time.sleep(0.12)
                        events = re.findall(r"<- wl_pointer@\d+\.axis, \(\d+, 0, -[\d.]+\)", log.read_text()[offset:])
                        report["wheel_event_frames"] = len(events)
                        expected = str(120 + len(events))
                        states["wheel_replacement"] = changed(method, previous, expected, len(expected), 0)
                    previous = method.states[-1]["serial"]
                    SIZE.INPUT.move((6, point[1]))
                    states[operation + "_continued_drag"] = changed(method, previous, expected, 0, 0)
                finally:
                    button("left", False)
                session.key("Escape")
            # 7. 【选区字段回归】【导出恢复】结束字段交互后仍按物理像素导出，无界面文字或多余空闲工作
            SIZE.set_size(100)
            SIZE.set_size(80, "height")
            report["states"] = states
            report["idle"] = SIZE.idle(process, log)
            report["export"] = SIZE.png(environment, process, root, (100, 80))
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    """【选区字段回归】【入口】解析 binary 与新 root，打印验证结果，失败保留本次协议和截图"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=PROJECT / "target/debug/mark-shot")
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    report = run(options.binary.resolve(), options.root.resolve())
    print(json.dumps({"binary_sha256": report["binary_sha256"], "compositor": report["compositor"], "states": len(report["states"]), "result": str(options.root / "result.json")}, indent=2))


if __name__ == "__main__":
    main()
