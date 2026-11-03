"""【Wayland 测试】【固定键码】通过私有 Xvfb 向嵌套合成器发送键盘输入，不连接真实显示服务器"""

import json
from contextlib import contextmanager
import os
from pathlib import Path
import re
import subprocess


MODIFIERS = {
    "ctrl": "ctrl", "control": "ctrl", "shift": "shift", "alt": "alt",
    "super": "super", "meta": "super", "logo": "super",
}
NAMED_KEYS = {
    "return": "Return", "enter": "Return", "escape": "Escape", "esc": "Escape",
    "space": "space", "tab": "Tab", "backspace": "BackSpace", "delete": "Delete",
    "del": "Delete", "insert": "Insert", "ins": "Insert", "home": "Home", "end": "End",
    "pageup": "Prior", "page_up": "Prior", "pagedown": "Next", "page_down": "Next",
    "left": "Left", "right": "Right", "up": "Up", "down": "Down",
    "bracketleft": "bracketleft", "bracketright": "bracketright", "minus": "minus",
    "equal": "equal", "semicolon": "semicolon", "apostrophe": "apostrophe",
    "comma": "comma", "period": "period", "slash": "slash", "backslash": "backslash",
    "grave": "grave",
}


def canonical_chord(value):
    """【测试键盘】【快捷键转换】将 value 转为固定键码工具接受的组合；单字母不隐含 Shift，返回规范字符串"""
    parts = [part.strip() for part in value.split("+")]
    if not parts or any(not part for part in parts):
        raise ValueError(f"invalid keyboard chord: {value!r}")
    modifiers = []
    for part in parts[:-1]:
        try:
            modifier = MODIFIERS[part.lower()]
        except KeyError as error:
            raise ValueError(f"unknown keyboard modifier: {part!r}") from error
        if modifier not in modifiers:
            modifiers.append(modifier)
    key = parts[-1]
    if len(key) == 1 and key.isascii() and (key.isalnum() or key in "[]-=';,./\\`"):
        key = key.lower()
    elif re.fullmatch(r"[fF](?:[1-9]|1[0-9]|2[0-4])", key):
        key = key.upper()
    else:
        try:
            key = NAMED_KEYS[key.lower()]
        except KeyError as error:
            raise ValueError(f"unknown keyboard key: {key!r}") from error
    return "+".join([*modifiers, key])


def runtime_environment():
    """【测试键盘】【隔离校验】核对环境、私有记录和存活进程；返回隔离环境与合成器 PID，校验失败禁止输入"""
    environment = dict(os.environ)
    display = environment.get("MARK_SHOT_TEST_X11_DISPLAY")
    if not display or display != environment.get("DISPLAY"):
        raise RuntimeError("fixed keyboard input requires the independent Xvfb display")
    root_value = environment.get("XDG_RUNTIME_DIR")
    if not root_value:
        raise RuntimeError("the independent runtime directory is missing")
    root = Path(root_value).resolve()
    runtime = json.loads((root / "runtime.json").read_text())
    if runtime.get("x11_display") != display:
        raise RuntimeError("the independent X11 display does not match the runtime record")
    socket = Path(runtime["niri_socket"])
    if socket.parent.resolve() != root or not socket.exists():
        raise RuntimeError("the independent compositor socket is unavailable")
    if runtime["wayland_display"] != environment.get("WAYLAND_DISPLAY"):
        raise RuntimeError("the Wayland display does not match the independent runtime")
    xvfb_pid, niri_pid = int(runtime["xvfb_pid"]), int(runtime["niri_pid"])
    if xvfb_pid <= 1 or niri_pid <= 1 or xvfb_pid == niri_pid:
        raise RuntimeError("the independent process identities are invalid")
    os.kill(xvfb_pid, 0)
    os.kill(niri_pid, 0)
    return environment, niri_pid


def focused_environment():
    """【测试键盘】【私有焦点】校验并聚焦唯一私有合成器窗口；返回隔离环境，歧义或进程失效时禁止输入"""
    environment, pid = runtime_environment()
    windows = subprocess.check_output(
        ["xdotool", "search", "--pid", str(pid)], env=environment, text=True, timeout=3,
    ).splitlines()
    if len(windows) != 1 or not windows[0].isdecimal() or int(windows[0]) == 0:
        raise RuntimeError(f"the independent compositor window is ambiguous: {windows}")
    # 1. 【测试键盘】【窗口焦点】独立显示服务器没有窗口管理器，输入前明确聚焦本次合成器
    subprocess.run(
        ["xdotool", "windowfocus", "--sync", windows[0]], env=environment, check=True, timeout=3,
    )
    return environment


def send(value):
    """【测试键盘】【按键发送】只向私有合成器窗口发送 value；返回规范快捷键，窗口歧义或失败时不发送输入"""
    chord = canonical_chord(value)
    environment = focused_environment()
    # 2. 【测试键盘】【固定键码】使用私有显示器的标准映射，避免重复虚拟键盘在新会话中漏发映射
    subprocess.run(
        ["xdotool", "key", "--clearmodifiers", chord], env=environment, check=True, timeout=3,
    )
    return chord


@contextmanager
def hold(value):
    """【测试键盘】【修饰键保持】在私有合成器中保持 value 修饰键供鼠标拖动；返回规范名称，异常退出也释放该键"""
    try:
        modifier = MODIFIERS[value.lower()]
    except KeyError as error:
        raise ValueError(f"a supported modifier is required: {value!r}") from error
    environment = focused_environment()
    try:
        subprocess.run(["xdotool", "keydown", modifier], env=environment, check=True, timeout=3)
        yield modifier
    finally:
        subprocess.run(["xdotool", "keyup", modifier], env=environment, check=True, timeout=3)
