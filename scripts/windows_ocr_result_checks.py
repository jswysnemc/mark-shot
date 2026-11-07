"""【Windows识别回归】【结果操作】验证实际结果窗口的Unicode正文、独立历史和页签切换"""

import time

from wayland_ocr_window_layout import tab_point
from windows_ocr_actions import click, copy_all, key, rectangle, set_clipboard, snapshot, xdo
from windows_ocr_theme_checks import check as check_theme


def result_checks(binary, probe, environment, root, identity, recognized, has_table=False):
    """【Windows识别回归】【正文与页面】按实际identity检查recognized的编辑、撤销、页签和窗口尺寸，返回逐项结果"""
    # 1. 【Windows识别回归】【Unicode粘贴】系统剪贴板提供者先退出，结果窗口再读取中文、多行和补充平面字符
    edited = "Windows result edit\n中文内容与标点：测试\nSupplementary character: 𠮷"
    provider_seconds = set_clipboard(binary, environment, root, edited)
    key(environment, identity, "ctrl+a")
    key(environment, identity, "ctrl+v")
    copy_all(probe, environment, root, identity, "unicode-edited", edited)
    snapshot(environment, root, "unicode-edited")
    key(environment, identity, "ctrl+z")
    copy_all(probe, environment, root, identity, "undo", recognized)
    key(environment, identity, "ctrl+shift+z")
    copy_all(probe, environment, root, identity, "redo", edited)

    # 2. 【Windows识别回归】【原图页签】切换原图再返回正文，全文与编辑历史保持相同
    bounds = rectangle(environment, identity)
    point = tab_point(bounds, "Image", has_table=has_table)
    click(environment, identity, point[0] - bounds["x"], point[1] - bounds["y"])
    time.sleep(0.12)
    snapshot(environment, root, "source-image")
    point = tab_point(bounds, "Text", has_table=has_table)
    click(environment, identity, point[0] - bounds["x"], point[1] - bounds["y"])
    copy_all(probe, environment, root, identity, "after-image-tab", edited)

    # 3. 【Windows识别回归】【紧凑尺寸】请求最小客户区并核对实际几何与完整正文，恢复后保留相同内容
    xdo(environment, "windowsize", "--sync", identity, 380, 340)
    narrow = rectangle(environment, identity)
    assert (narrow["width"], narrow["height"]) == (380, 340), narrow
    copy_all(probe, environment, root, identity, "narrow-text", edited)
    snapshot(environment, root, "narrow-result")
    xdo(environment, "windowsize", "--sync", identity, bounds["width"], bounds["height"])
    copy_all(probe, environment, root, identity, "restored-text", edited)
    theme = check_theme(probe, environment, root, identity, edited)
    return {"unicode_clipboard": True, "clipboard_provider_seconds": provider_seconds,
            "undo_redo": True, "source_tab_preserves_text": True,
            "minimum_client_size": narrow, "edited_text": edited, "theme": theme}
