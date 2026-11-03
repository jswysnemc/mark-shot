"""【文字回归】【会话适配】按本次私有记录选择输入后端，共用剪贴板断言和进程资源读取"""

import json
import os
from pathlib import Path
from contextlib import contextmanager

import wayland_test_session as niri
import wayland_text_sway_session as sway

resources = niri.resources
wait_single_thread = niri.wait_single_thread
read_clipboard = niri.read_clipboard
wait_clipboard = niri.wait_clipboard


@contextmanager
def input_context():
    """【文字回归】【输入范围】管理本次后端的测试设备，无界面 Sway 保留按需键盘；退出时释放资源，无返回值"""
    if backend() is sway:
        with sway.input_context():
            yield
    else:
        yield


def backend():
    """【文字回归】【后端选择】读取本次私有运行记录，返回 Sway 或 niri 适配器；各适配器自行校验进程与套接字"""
    runtime = json.loads((Path(os.environ["XDG_RUNTIME_DIR"]) / "runtime.json").read_text())
    return sway if runtime.get("compositor") == "sway" else niri


def environment():
    """【文字回归】【环境选择】返回当前适配器已经校验的私有环境，不读取或切换用户会话"""
    return backend().environment()


def key(value):
    """【文字回归】【按键选择】把 value 交给本次私有输入后端，无返回值"""
    backend().key(value)


def paste_text(value):
    """【文字回归】【粘贴选择】将 value 提供给本次私有剪贴板并粘贴，无返回值"""
    backend().paste_text(value)


def screenshot(path):
    """【文字回归】【截图选择】将本次私有画面保存到 path，返回后端结果"""
    return backend().screenshot(path)
