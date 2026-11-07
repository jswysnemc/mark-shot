"""【Windows贴图粘贴】【独立交接】只在私有会话验证剪贴板创建新贴图，不替换原窗口"""

import time

from windows_ocr_actions import key, snapshot, visible
from windows_pin_checks import copied, menu, size


def check(probe, environment, root, identity, expected):
    """【Windows贴图粘贴】【图像交接】按identity复制expected后粘贴新窗口，确认原件和原窗口不变，返回证据"""
    copied(probe, environment, root, identity, "paste-source", expected)
    # 1. 【Windows贴图粘贴】【菜单隔离】菜单持有键盘时Ctrl+V不能创建窗口或关闭菜单
    popup = menu(environment, root, identity)
    key(environment, identity, "ctrl+v")
    time.sleep(0.3)
    assert visible(environment, "Pinned Mark Shot") == [identity]
    assert visible(environment, "Mark Shot menu") == [popup]
    key(environment, identity, "Escape")
    key(environment, identity, "ctrl+v")
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        windows = visible(environment, "Pinned Mark Shot")
        if len(windows) == 2 and identity in windows:
            child = next(window for window in windows if window != identity)
            break
        time.sleep(0.05)
    else:
        snapshot(environment, root, "paste-window-failure")
        raise AssertionError("Ctrl+V did not create a second independent pin")
    size(environment, child, expected.size)
    copied(probe, environment, root, child, "paste-child-original", expected)
    snapshot(environment, root, "paste-two-windows")
    key(environment, child, "Escape")
    deadline = time.monotonic() + 6
    while visible(environment, "Pinned Mark Shot") != [identity]:
        assert time.monotonic() < deadline, "closing the pasted pin did not preserve its parent"
        time.sleep(0.04)
    copied(probe, environment, root, identity, "paste-parent-preserved", expected)
    return {"second_window": True, "original_pixels_exact": True,
            "parent_preserved": True, "menu_blocks_paste": True}
