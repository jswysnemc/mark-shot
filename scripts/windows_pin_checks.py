"""【Windows贴图回归】【交互断言】在私有显示验证菜单事务、原图交付和原生窗口尺寸"""

import subprocess
import time

from PIL import Image
from windows_ocr_actions import click, key, path_w, rectangle, snapshot, visible, window, xdo


SEPARATORS = {2, 6, 8, 18}


def absent(environment, title):
    """【Windows贴图回归】【窗口回收】等待title的实际窗口全部消失，无返回值，超时明确失败"""
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        if not visible(environment, title):
            return
        time.sleep(0.04)
    raise AssertionError(f"window remained visible: {title}")


def size(environment, identity, expected):
    """【Windows贴图回归】【尺寸确认】按identity等待expected客户区尺寸，返回实际几何，禁止以复制图像推测预览尺寸"""
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        actual = rectangle(environment, identity)
        if (actual["width"], actual["height"]) == expected:
            return actual
        time.sleep(0.04)
    raise AssertionError(f"pin size differs: {actual} != {expected}")


def menu(environment, root, identity):
    """【Windows贴图回归】【菜单展开】在identity内部右击并等待独立菜单，返回实际菜单窗口编号"""
    bounds = rectangle(environment, identity)
    xdo(environment, "windowfocus", "--sync", identity)
    # 1. 【Windows贴图回归】【静止指针】同位置移动不会生成Motion，不能使用等待Motion的xdotool同步选项
    xdo(environment, "mousemove", "--window", identity, 30, 30)
    actual = dict(line.split("=", 1) for line in xdo(environment, "getmouselocation", "--shell").splitlines())
    assert (int(actual["X"]), int(actual["Y"])) == (bounds["x"] + 30, bounds["y"] + 30)
    xdo(environment, "click", 3)
    return window(environment, "Mark Shot menu", root)


def row(environment, popup, index, pressed=None):
    """【Windows贴图回归】【菜单选择】按popup与index定位共享菜单行，pressed控制按下、释放或完整点击，无返回值"""
    assert index not in SEPARATORS
    top = 4 + sum(9 if value in SEPARATORS else 28 for value in range(index))
    xdo(environment, "mousemove", "--sync", "--window", popup, 100, top + 14)
    if pressed is None:
        xdo(environment, "click", 1)
    else:
        xdo(environment, "mousedown" if pressed else "mouseup", 1)


def activate(environment, root, identity, index):
    """【Windows贴图回归】【动作完成】在identity打开菜单并确认index，等待独立菜单销毁，无返回值"""
    popup = menu(environment, root, identity)
    row(environment, popup, index)
    absent(environment, "Mark Shot menu")


def copied(probe, environment, root, identity, name, expected):
    """【Windows贴图回归】【完整原图】在identity复制，经probe读取Windows剪贴板并与expected逐字节比较，返回尺寸"""
    key(environment, identity, "ctrl+c")
    target = root / (name + ".png")
    subprocess.run(["wine", probe, "image", path_w(target, environment)], env=environment,
                   cwd=root, check=True, timeout=8)
    actual = Image.open(target).convert("RGBA")
    assert actual.size == expected.size and actual.tobytes() == expected.tobytes(), f"{name}: original pixels differ"
    return actual.size


def save_as(binary, environment, root, identity, expected):
    """【Windows贴图回归】【保存事务】验证菜单打开后取消、快捷键保存和原图一致，返回保存证据，不覆盖用户文件"""
    activate(environment, root, identity, 17)
    dialog = window(environment, "Mark Shot / Save Screenshot", root)
    snapshot(environment, root, "save-cancel-dialog")
    key(environment, dialog, "Escape")
    absent(environment, "Mark Shot / Save Screenshot")
    assert visible(environment, "Pinned Mark Shot") == [identity]
    key(environment, identity, "ctrl+s")
    dialog = window(environment, "Mark Shot / Save Screenshot", root)
    time.sleep(0.2)
    # 1. 【Windows贴图回归】【实际输入】目录与文件名分别填写，不绕过文件名字段的路径校验
    from windows_ocr_actions import set_clipboard
    target = root / "pin-saved.png"
    set_clipboard(binary, environment, root, path_w(root, environment))
    click(environment, dialog, 160, 60)
    key(environment, dialog, "ctrl+a")
    key(environment, dialog, "ctrl+v")
    key(environment, dialog, "Return")
    time.sleep(0.2)
    set_clipboard(binary, environment, root, target.name)
    click(environment, dialog, 160, 366)
    key(environment, dialog, "ctrl+a")
    key(environment, dialog, "ctrl+v")
    snapshot(environment, root, "save-filled-dialog")
    click(environment, dialog, 604, 404)
    absent(environment, "Mark Shot / Save Screenshot")
    actual = Image.open(target).convert("RGBA")
    assert actual.size == expected.size and actual.tobytes() == expected.tobytes(), "saved pin changed original pixels"
    return {"cancel_restores_pin": True, "saved_equals_original": True, "path": str(target)}


def wheel(probe, environment, root, identity, expected):
    """【Windows贴图回归】【滚轮锚点】在identity实测纵向缩放、锚点位置、反向恢复和横向隔离，返回几何证据并核对expected原件"""
    before = rectangle(environment, identity)
    xdo(environment, "windowfocus", "--sync", identity)
    xdo(environment, "mousemove", "--window", identity, 80, 50)
    xdo(environment, "click", 4)
    enlarged = size(environment, identity, (346, 216))
    assert abs(enlarged["x"] - (before["x"] - 6.5)) <= 1
    assert abs(enlarged["y"] - (before["y"] - 4)) <= 1
    copied(probe, environment, root, identity, "wheel-original", expected)
    # 1. 【Windows贴图回归】【纯横向输入】系统水平滚轮不改变贴图尺寸或位置
    xdo(environment, "click", 6)
    time.sleep(0.15)
    assert rectangle(environment, identity) == enlarged
    xdo(environment, "click", 5)
    restored = size(environment, identity, (320, 200))
    assert abs(restored["x"] - before["x"]) <= 1 and abs(restored["y"] - before["y"]) <= 1
    copied(probe, environment, root, identity, "wheel-restored-original", expected)
    # 2. 【Windows贴图回归】【菜单隔离】指针停在菜单外的贴图上，滚轮也不能缩放底层窗口
    menu(environment, root, identity)
    xdo(environment, "mousemove", "--window", identity, 10, 10)
    xdo(environment, "click", 4)
    time.sleep(0.15)
    assert rectangle(environment, identity) == restored
    key(environment, identity, "Escape")
    absent(environment, "Mark Shot menu")
    return {"before": before, "enlarged": enlarged, "restored": restored,
            "original_exact": True, "horizontal_ignored": True, "menu_blocks_zoom": True}
