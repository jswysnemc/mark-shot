"""【文字回归】【私有指针】完整菜单场景使用同一设备，不在合成器抓取期间替换指针连接"""

from contextlib import contextmanager
from contextvars import ContextVar
import json
import os
from pathlib import Path
import time
import subprocess

import wayland_text_format_session as session
from wayland_test_keyboard import focused_environment
from wayland_pin_pointer import GlobalPointer

from wayland_virtual_pointer import VirtualPointer, BTN_LEFT, BTN_RIGHT, BTN_MIDDLE, transform_for_pointer


CURRENT = ContextVar("private_text_pointer", default=None)
BUTTONS = {"left": BTN_LEFT, "right": BTN_RIGHT, "middle": BTN_MIDDLE}


@contextmanager
def pointer_context(right_hold=0.04, backend="virtual"):
    """【文字回归】【设备范围】校验私有会话，right_hold 为右键秒数，backend 选择协议或私有 Xvfb；退出时释放设备，返回设备"""
    if not 0 < right_hold <= 2:
        raise ValueError("private right press duration must be between zero and two seconds")
    session.environment()
    runtime = json.loads((Path(os.environ["XDG_RUNTIME_DIR"]) / "runtime.json").read_text())
    outputs = runtime["outputs"]
    if len(outputs) != 1:
        raise RuntimeError("text pointer requires exactly one isolated output")
    logical = next(iter(outputs.values()))["logical"]
    if backend == "x11":
        environment = focused_environment()
        output = next(iter(outputs.values()))
        mode = output["modes"][output["current_mode"]]
        token = CURRENT.set((None, logical, right_hold, (environment, mode)))
        try:
            yield None
        finally:
            CURRENT.reset(token)
        return
    if backend != "virtual":
        raise ValueError(f"unsupported private pointer backend: {backend}")
    pointer_class = VirtualPointer
    if runtime.get("compositor") == "sway":
        pointer_class = GlobalPointer
        logical = dict(logical, pointer_transform="normal")
    with pointer_class() as pointer:
        token = CURRENT.set((pointer, logical, right_hold, None))
        try:
            yield pointer
        finally:
            CURRENT.reset(token)


def click(point, button="left"):
    """【文字回归】【设备点击】在已验证设备上向 point 发送 button 完整点击，按实际输出变换转换坐标，无返回值"""
    active = CURRENT.get()
    if active is None:
        raise RuntimeError("text pointer input requires a verified private context")
    if button not in BUTTONS:
        raise ValueError(f"unsupported pointer button: {button}")
    pointer, logical, right_hold, x11 = active
    coordinates = transform_for_pointer(point[0] - logical["x"], point[1] - logical["y"], logical["width"], logical["height"], logical.get("pointer_transform", logical["transform"]))
    hold = right_hold if button == "right" else 0.04
    if x11 is not None:
        environment, mode = x11
        # 1. 【文字回归】【私有输入方向】winit 鼠标坐标已经使用逻辑方向，不能再次套用虚拟指针协议的输出翻转
        x = round((point[0] - logical["x"]) * mode["width"] / logical["width"])
        y = round((point[1] - logical["y"]) * mode["height"] / logical["height"])
        subprocess.run(["xdotool", "mousemove", "--sync", str(x), str(y)], env=environment, check=True, timeout=3)
        time.sleep(0.02)
        code = {"left": "1", "right": "3", "middle": "2"}[button]
        subprocess.run(["xdotool", "mousedown", code], env=environment, check=True, timeout=3)
        try:
            time.sleep(hold)
        finally:
            subprocess.run(["xdotool", "mouseup", code], env=environment, check=True, timeout=3)
        return
    pointer.move_abs(*coordinates)
    time.sleep(0.02)
    pointer.click(BUTTONS[button], hold=hold)
