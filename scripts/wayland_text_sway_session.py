"""【文字回归】【私有 Sway】仅向本次无界面合成器发送输入，不接受用户会话套接字"""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
from contextlib import contextmanager
from contextvars import ContextVar

from wayland_test_keyboard import canonical_chord

KEYBOARD = ContextVar("private_sway_keyboard", default=None)


@contextmanager
def input_context():
    """【文字回归】【键盘范围】记录本次测试设备，首次按键才启动；退出时只终止本次键盘进程，无参数、无返回值"""
    state = {"process": None}
    token = KEYBOARD.set(state)
    try:
        yield
    finally:
        process = state["process"]
        if process is not None and process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=3)
        KEYBOARD.reset(token)


def keyboard_ready():
    """【文字回归】【持续键盘】在已验证私有会话保留一个无按键设备，避免每次命令撤销键盘能力；没有测试范围时拒绝，无返回值"""
    state = KEYBOARD.get()
    if state is None:
        raise RuntimeError("Sway keyboard requires an owned private input context")
    if state["process"] is None:
        state["process"] = subprocess.Popen(["wtype", "-s", "600000"], env=environment(), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(0.1)
    if state["process"].poll() is not None:
        raise RuntimeError("the private persistent keyboard exited unexpectedly")


def environment():
    """【文字回归】【会话校验】验证本次 Sway 进程、私有套接字和 Wayland 身份，返回隔离环境，不读取用户配置"""
    root = Path(os.environ["XDG_RUNTIME_DIR"]).resolve()
    runtime = json.loads((root / "runtime.json").read_text())
    if runtime.get("compositor") != "sway" or runtime["sway_pid"] <= 1:
        raise RuntimeError("text Sway input requires a verified private runtime")
    socket = Path(runtime["sway_socket"])
    if socket.parent.resolve() != root or not socket.exists() or str(socket) != os.environ.get("SWAYSOCK"):
        raise RuntimeError("private Sway socket does not match this runtime")
    if runtime["wayland_display"] != os.environ.get("WAYLAND_DISPLAY") or not (root / runtime["wayland_display"]).is_socket():
        raise RuntimeError("private Wayland socket does not match this runtime")
    os.kill(runtime["sway_pid"], 0)
    return dict(os.environ)


def key(value):
    """【文字回归】【私有按键】将 value 转为规范键名并送入已校验的 Sway，无返回值；不连接外部显示服务器"""
    chord = canonical_chord(value).split("+")
    keyboard_ready()
    modifiers = ["logo" if item == "super" else item for item in chord[:-1]]
    # 1. 【文字回归】【键盘就绪】无界面座席没有常驻键盘，先等待能力通知与客户端绑定，再发送实际按下
    command = ["wtype", "-s", "100"]
    for modifier in modifiers:
        command += ["-M", modifier]
    command += ["-k", chord[-1]]
    for modifier in reversed(modifiers):
        command += ["-m", modifier]
    command += ["-s", "50"]
    subprocess.run(command, env=environment(), check=True, timeout=3)
    time.sleep(0.04)


def paste_text(value):
    """【文字回归】【私有粘贴】用本次会话剪贴板提供 value 并发送 Ctrl+V，无返回值，不继承外部剪贴板服务"""
    with tempfile.TemporaryFile(mode="w+t") as errors:
        subprocess.run(["wl-copy", "--", value], env=environment(), stdout=subprocess.DEVNULL, stderr=errors, check=True, timeout=3)
    key("ctrl+v")


def screenshot(path):
    """【文字回归】【私有画面】将本次唯一输出以逻辑一倍保存到 path，返回路径，不捕获其他桌面"""
    runtime = json.loads((Path(os.environ["XDG_RUNTIME_DIR"]) / "runtime.json").read_text())
    if len(runtime["outputs"]) != 1:
        raise RuntimeError("text screenshot requires one private output")
    output = next(iter(runtime["outputs"]))
    subprocess.run(["grim", "-s", "1", "-o", output, str(path)], env=environment(), check=True, timeout=3)
    return str(path)
