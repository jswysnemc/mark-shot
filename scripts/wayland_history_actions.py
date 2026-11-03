"""【截图历史测试】【真实输入】隔离窗口定位、搜索和保存对话框操作"""

from pathlib import Path
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


def footer(item, action):
    """【截图历史测试】【操作栏】点击 item 内 action 按钮；使用实际窗口尺寸，不假定桌面比例"""
    controls = ("copy", "edit", "pin", "save", "delete", "clear")
    width, height = item["size"]
    button = min((width - 36) / 6, 78)
    session.click(item, 8 + controls.index(action) * (button + 4) + button / 2, height - 48)
    time.sleep(0.12)


def search(item, query):
    """【截图历史测试】【搜索输入】在 item 中真实粘贴 query；确认后把焦点交给结果列表"""
    focus(item)
    session.key("ctrl+f")
    if query:
        session.paste_text(query)
    else:
        session.key("BackSpace")
    time.sleep(0.12)
    session.key("Return")
    time.sleep(0.12)


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
