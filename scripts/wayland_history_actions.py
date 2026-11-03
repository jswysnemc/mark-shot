"""【截图历史测试】【真实输入】隔离窗口定位、原版操作栏、列表和保存对话框操作"""

from pathlib import Path
import json
import time

import wayland_test_session as session


def window(app_id="mark-shot-history", pid=None, timeout=8):
    """【截图历史测试】【窗口定位】等待 app_id、pid 对应的唯一私有窗口；返回当前窗口信息"""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        matching = [item for item in session.windows() if item["app_id"] == app_id and (pid is None or item["pid"] == pid)]
        if len(matching) == 1:
            return matching[0]
        if len(matching) > 1:
            raise AssertionError(f"ambiguous private window: {app_id}")
        time.sleep(0.04)
    raise AssertionError(f"private window did not appear: {app_id}")


def focus(item):
    """【截图历史测试】【窗口激活】只激活 item 对应私有窗口；返回最新窗口信息"""
    session.control("focus", item["id"])
    time.sleep(0.08)
    return next(current for current in session.windows() if current["id"] == item["id"])


def list_focus(item, index=0):
    """【截图历史测试】【列表选择】在 item 的真实列表选择 index 并取得焦点，等待一次性预览结束，无返回值"""
    focus(item)
    session.wait_single_thread(item["pid"], timeout=15)
    time.sleep(0.45)
    session.click(item, 45, 104)
    session.key("Home")
    for _ in range(index):
        session.key("Down")
    time.sleep(0.12)
    session.wait_single_thread(item["pid"], timeout=15)


def focus_control(item, label):
    """【截图历史测试】【原版焦点】按真实 Qt 样本从 item 列表移动到可用 label，Space 在释放时执行，不假设状态行位置"""
    list_focus(item)
    reference = json.loads((Path(__file__).resolve().parents[1] / "tests/fixtures/qt-history-window.json").read_text())
    steps = reference["layouts"][0]["focus_sequence"].index(label)
    for _ in range(steps):
        session.key("Tab")
    session.key("Space")
    time.sleep(0.12)


def footer(item, action):
    """【截图历史测试】【原版操作】在 item 执行 action，顶部按钮坐标来自 Qt 实测，清空采用原版焦点顺序，无返回值"""
    if action == "clear":
        focus_control(item, "Clear History")
        return
    reference = json.loads((Path(__file__).resolve().parents[1] / "tests/fixtures/qt-history-window.json").read_text())
    layout = next(case for case in reference["layouts"] if case["size"][0] == item["size"][0])
    labels = {"copy": "Copy", "edit": "Edit", "pin": "Pin", "save": "Save As", "delete": "Delete"}
    x, y, width, height = next(button["rect"] for button in layout["actions"] if button["label"] == labels[action])
    session.wait_single_thread(item["pid"], timeout=15)
    session.click(item, x + width / 2, y + height / 2)
    time.sleep(0.12)


def recording(item):
    """【截图历史测试】【记录开关】在有可用图像操作的 item 中切换原版记录复选框，无返回值"""
    focus_control(item, "enabled")


def field_text(environment):
    """【截图历史测试】【字段读取】复制当前字段全文并从 environment 读取；返回实际编辑文字"""
    session.key("ctrl+a")
    session.key("ctrl+c")
    return session.read_clipboard(environment)


def picker_geometry(item):
    """【截图历史测试】【保存布局】按 item 实际尺寸返回居中对话框区域"""
    width, height = item["size"]
    w, h = min(width - 16, 660), min(height - 16, 440)
    return (width - w) / 2, (height - h) / 2, w, h


def navigate(item, environment, directory):
    """【截图历史测试】【目录导航】输入 directory 并验证规范路径已经应用；不使用固定等待替代目录完成断言"""
    directory = directory.resolve()
    session.key("ctrl+l")
    session.paste_text(str(directory) + "/.")
    session.key("Return")
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if field_text(environment) == str(directory):
            return
        time.sleep(0.05)
    raise AssertionError("the chosen folder was not applied")


def filename(item, environment, name):
    """【截图历史测试】【文件名输入】在 item 的保存对话框输入 name，校验实际字段值"""
    x, y, width, height = picker_geometry(item)
    session.click(item, x + width / 2, y + height - 66)
    session.key("ctrl+a")
    session.paste_text(name)
    if field_text(environment) != name:
        raise AssertionError("filename input lost Unicode text")


def save(item):
    """【截图历史测试】【保存确认】点击 item 保存对话框的 Save，不绕过覆盖确认"""
    x, y, width, height = picker_geometry(item)
    session.click(item, x + width - 48, y + height - 28)
    time.sleep(0.15)


def overwrite(item, replace):
    """【截图历史测试】【覆盖选择】在 item 的确认弹层明确选择 replace 或取消"""
    x, y, width, height = picker_geometry(item)
    panel = min(width - 24, 360)
    left, top = x + (width - panel) / 2, y + (height - 148) / 2
    session.click(item, left + panel - (48 if replace else 126), top + 118)
    time.sleep(0.15)


def wait_file(path, timeout=5):
    """【截图历史测试】【保存等待】等待 path 成为非空普通文件；返回文件路径"""
    path = Path(path)
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if path.is_file() and path.stat().st_size:
            return path
        time.sleep(0.04)
    raise AssertionError(f"saved file did not appear: {path}")
