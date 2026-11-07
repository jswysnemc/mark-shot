#!/usr/bin/env python3
"""【结果按钮测试】【真实桌面】在私有会话验证按下、释放、焦点及取消，不启动模型"""

import argparse
import hashlib
import json
from pathlib import Path
import select
import struct
import subprocess
import time

import wayland_text_format_session as session
from wayland_test_keyboard import focused_environment
from wayland_text_pointer import CURRENT, click, pointer_context
from wayland_virtual_pointer import transform_for_pointer
from wayland_window_input_session import geometry
from wayland_ocr_window_layout import source_point

SOURCE = "alpha beta\ngamma"


def mouse(point, pressed):
    """【结果按钮测试】【独立按钮事件】向已校验指针的 point 发送 pressed，保留同一连接与输出变换，无返回值"""
    pointer, logical, _, _ = CURRENT.get()
    pointer.move_abs(*transform_for_pointer(point[0] - logical["x"], point[1] - logical["y"], logical["width"], logical["height"], logical.get("pointer_transform", logical["transform"])))
    time.sleep(0.03)
    if pressed:
        pointer.press()
    else:
        pointer.release()
    time.sleep(0.12)


def sentinel(environment):
    """【结果按钮测试】【剪贴板基准】在 environment 设置固定哨兵，不连接用户座席，无返回值"""
    subprocess.run(["wl-copy", "--", "button sentinel"], env=environment, check=True, timeout=3)


def held_space(environment, assert_pressed):
    """【结果按钮测试】【空格释放】在 environment 按住 Ctrl+Space，调用 assert_pressed 检查中间状态，再释放，无返回值"""
    if session.backend() is session.niri:
        private = focused_environment()
        subprocess.run(["xdotool", "keydown", "ctrl+space"], env=private, check=True, timeout=3)
        try:
            time.sleep(0.15)
            assert_pressed()
        finally:
            subprocess.run(["xdotool", "keyup", "ctrl+space"], env=private, check=True, timeout=3)
    else:
        process = subprocess.Popen(["wtype", "-s", "100", "-M", "ctrl", "-P", "space", "-s", "900", "-p", "space", "-m", "ctrl", "-s", "100"], env=environment)
        try:
            time.sleep(0.35)
            assert_pressed()
            assert process.wait(timeout=3) == 0
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)


def run(binary, root):
    """【结果按钮测试】【完整回归】在私有桌面执行 binary，root 保存同指纹证据；失败立即退出，无返回值"""
    environment = session.environment()
    root.mkdir(parents=True, exist_ok=False)
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    header = json.dumps({"text": SOURCE, "lines": []}).encode()
    png = (Path(__file__).resolve().parents[1] / "tests/fixtures/ocr-table.png").read_bytes()
    with session.input_context(), pointer_context(), (root / "process.log").open("w") as log:
        process = subprocess.Popen([str(binary), "--ocr-result", "--nonotify"], env=environment, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=log)
        try:
            process.stdin.write(b"MSOCR001" + struct.pack("<II", len(header), len(png)) + header + png)
            process.stdin.close()
            assert select.select([process.stdout], [], [], 5)[0] and process.stdout.readline() == b"\n"
            window = geometry(process.pid, environment)
            copy = source_point(window)
            close = (window["x"] + window["width"] - 21, window["y"] + 19)
            # 1. 【结果按钮测试】【左键时机】按下不能复制，释放后持久剪贴板才取得原文
            sentinel(environment)
            mouse(copy, True)
            assert session.read_clipboard(environment) == "button sentinel", "Copy executed on press instead of release"
            session.screenshot(root / "copy-pressed.png")
            mouse(copy, False)
            session.wait_clipboard(environment, SOURCE)
            # 2. 【结果按钮测试】【外部释放】按住后移出正文释放，不能执行复制
            sentinel(environment)
            mouse(copy, True)
            mouse((window["x"] + 24, window["y"] + 190), False)
            assert session.read_clipboard(environment) == "button sentinel", "release outside executed Copy"
            click(copy, "right")
            session.key("Return")
            assert session.read_clipboard(environment) == "button sentinel", "Return incorrectly activated a non-default button"
            # 3. 【结果按钮测试】【键盘时机】右键只聚焦，修饰键 Space 按下不复制，释放才执行
            def unchanged():
                """【结果按钮测试】【按下断言】检查当前私有哨兵和窗口存活，不使用复制后的终态代替中间状态，无返回值"""
                assert process.poll() is None
                assert session.read_clipboard(environment) == "button sentinel", "Space executed before release"
                session.screenshot(root / "space-pressed.png")

            held_space(environment, unchanged)
            session.wait_clipboard(environment, SOURCE)
            # 4. 【结果按钮测试】【关闭职责】关闭按钮按下、右键、Return 和修饰键 Escape 均不能关闭，左键释放才结束
            click(close, "right")
            session.key("Return")
            session.key("ctrl+Escape")
            assert process.poll() is None, "right focus or a modified key closed the result window"
            # 5. 【结果按钮测试】【反馈到期】先等待全文复制的单次反馈期限，不把合法清除当作空闲轮询
            time.sleep(2.05)
            before = session.resources(process.pid)
            time.sleep(1.1)
            after = session.resources(process.pid)
            assert before["threads"] == after["threads"] == 1 and before["cpu_ticks"] == after["cpu_ticks"], "idle control retained workers or consumed CPU"
            mouse(close, True)
            assert process.poll() is None, "Close executed on press"
            mouse(close, False)
            assert process.wait(timeout=3) == 0
            session.wait_clipboard(environment, SOURCE)
            result = {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "bytes": binary.stat().st_size,
                "compositor": "sway" if session.backend() is session.sway else "niri", "pointer_release_delivery": True,
                "outside_release_canceled": True, "right_focus_without_activation": True, "return_not_default": True,
                "modified_space_release_delivery": True, "modified_escape_ignored": True, "close_release_delivery": True,
                "persistent_clipboard": True, "idle_before": before, "idle_after": after, "no_ocr": True}
            (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)


def main():
    """【结果按钮测试】【入口】解析二进制和证据目录后执行验证，无参数；失败返回非零状态"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    args = parser.parse_args()
    run(args.binary.resolve(), args.root.resolve())


if __name__ == "__main__":
    main()
