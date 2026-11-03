"""【Wayland 测试】【会话工具】隔离桌面的窗口、输入、剪贴板与资源验证"""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import time


def environment():
    """【Wayland 测试】【会话环境】验证子会话进程与套接字；返回隔离环境，不继承结果窗口开关"""
    state = Path(os.environ.get("XDG_RUNTIME_DIR", "/run/user/1000")) / "niri-child/state"
    values = dict(line.split("=", 1) for line in state.read_text().splitlines() if "=" in line)
    os.kill(int(values["CHILD_PID"]), 0)
    if not Path(values["CHILD_SOCKET"]).exists():
        raise RuntimeError("the nested compositor socket is unavailable")
    result = dict(os.environ, WAYLAND_DISPLAY=values["CHILD_DISPLAY"], NIRI_SOCKET=values["CHILD_SOCKET"])
    for name in ("MARK_SHOT_OCR_RESULT_PANEL", "MARK_SHOT_OCR_RESULT_WINDOW"):
        result.pop(name, None)
    return result


def control(*arguments):
    """【Wayland 测试】【桌面动作】在子会话中执行 arguments；返回标准输出，失败时抛出异常"""
    # 1. 后台剪贴板会保留错误描述符，使用普通文件避免命令结束后仍等待输出管道关闭
    with tempfile.TemporaryFile(mode="w+t", encoding="utf-8") as errors:
        process = subprocess.run(["niri-use", "--child", *map(str, arguments)], stdout=subprocess.PIPE, stderr=errors, text=True, check=False)
        if process.returncode:
            errors.seek(0)
            raise RuntimeError(f"nested desktop command failed ({process.returncode}): {errors.read().strip()}")
        return process.stdout


def windows():
    """【Wayland 测试】【窗口列表】返回隔离会话中可供窗口管理器识别的普通窗口"""
    return json.loads(control("windows"))


def wait_window(pid=None, timeout=8):
    """【Wayland 测试】【结果等待】等待 pid 对应的普通 OCR 窗口；pid 为空时接受本次会话中唯一结果窗口"""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        matching = [window for window in windows() if window["app_id"] == "mark-shot-ocr-result" and (pid is None or window["pid"] == pid)]
        if len(matching) == 1:
            return matching[0]
        if len(matching) > 1:
            raise RuntimeError("multiple OCR windows make the test ambiguous")
        time.sleep(0.04)
    raise RuntimeError("a normal OCR result window did not appear")


def size_window(window, width=760, height=460):
    """【Wayland 测试】【窗口尺寸】将 window 设为浮动并调整到 width、height；返回稳定后的实际几何信息"""
    identifier = window["id"]
    if not window["floating"]:
        control("float", identifier)
    control("action", "set-window-width", "--id", identifier, width)
    control("action", "set-window-height", "--id", identifier, height)
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        current = next((item for item in windows() if item["id"] == identifier), None)
        if current and current["size"] == [width, height] and current["floating_pos"]:
            time.sleep(0.08)
            return next(item for item in windows() if item["id"] == identifier)
        time.sleep(0.03)
    raise RuntimeError(f"window did not resize to {width}x{height}")


def click(window, x, y):
    """【Wayland 测试】【窗口点击】点击 window 内本地坐标 x、y；等待输入进入应用队列"""
    left, top = window["floating_pos"]
    control("click", "--logical", round(left + x), round(top + y))
    time.sleep(0.04)


def key(value):
    """【Wayland 测试】【按键动作】发送 value 快捷键；等待事件进入应用队列"""
    if os.environ.get("MARK_SHOT_TEST_X11_DISPLAY"):
        from wayland_test_keyboard import send
        send(value)
    else:
        control("key", value)
    time.sleep(0.04)


def paste_text(value):
    """【Wayland 测试】【文字粘贴】校验私有会话后复制 value 并发送 Ctrl+V；无返回值，不提前替换应用尚未读取的数据"""
    private = environment()
    # 1. 【Wayland 测试】【文字粘贴】选项终止符保护负数和任意文本，后台提供进程不保留测试输出管道
    with tempfile.TemporaryFile(mode="w+t", encoding="utf-8") as errors:
        process = subprocess.run(["wl-copy", "--", value], env=private, stdout=subprocess.DEVNULL, stderr=errors, text=True, check=False, timeout=3)
        if process.returncode:
            errors.seek(0)
            raise RuntimeError(f"private clipboard copy failed ({process.returncode}): {errors.read().strip()}")
    key("ctrl+v")


def read_clipboard(env):
    """【Wayland 测试】【剪贴板读取】从 env 对应会话读取原始 UTF-8 文字；不追加换行"""
    return subprocess.check_output(["wl-paste", "--no-newline"], env=env).decode()


def wait_clipboard(env, expected):
    """【Wayland 测试】【复制验证】等待 env 中剪贴板等于 expected；返回实际内容，超时报告差异"""
    deadline = time.monotonic() + 3
    actual = ""
    while time.monotonic() < deadline:
        actual = read_clipboard(env)
        if actual == expected:
            return actual
        time.sleep(0.03)
    raise RuntimeError(f"clipboard mismatch: expected {expected!r}, received {actual!r}")


def copy_all(env, expected):
    """【Wayland 测试】【全文复制】先替换剪贴板为哨兵，再验证应用按键确实复制 expected"""
    control("copy", "clipboard sentinel")
    key("ctrl+shift+c")
    return wait_clipboard(env, expected)


def screenshot(path):
    """【Wayland 测试】【界面截图】将隔离桌面截图写入 path；返回截图信息"""
    return control("screenshot", "--out", path)


def resources(pid):
    """【Wayland 测试】【资源采样】读取 pid 的 RSS、线程数与 CPU 时钟；返回可比较的字典"""
    status = dict(line.split(":", 1) for line in Path(f"/proc/{pid}/status").read_text().splitlines() if ":" in line)
    fields = Path(f"/proc/{pid}/stat").read_text().rsplit(") ", 1)[1].split()
    return {"pid": pid, "rss_kib": int(status["VmRSS"].split()[0]), "threads": int(status["Threads"]), "cpu_ticks": int(fields[11]) + int(fields[12])}


def wait_single_thread(pid, timeout=8):
    """【Wayland 测试】【后台等待】等待 pid 的启动字体任务结束，timeout 为秒；返回单线程资源快照，超时抛出异常"""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        current = resources(pid)
        if current["threads"] == 1:
            return current
        time.sleep(0.04)
    raise RuntimeError(f"the editor's startup work did not finish: {resources(pid)}")
