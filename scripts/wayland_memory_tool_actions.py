"""【工具内存对照】【相同手势】用实际正文变化确认两版执行同一工具操作"""

import time

from wayland_tool_frame_stability import capture_stable


SHORTCUTS = {"rectangle": "r", "pen": "p", "ruler": "u", "ellipse": "c",
             "highlighter": "h", "marker": "k", "line": "d", "arrow": "a",
             "filter": "m", "number": "n",
             "magnifier": "z", "text": "t"}
CASES = tuple(SHORTCUTS)
QT_SHORTCUTS = {**SHORTCUTS, "ellipse": "e", "line": "l"}
UNMATCHED_QT_TOOLS = ("numbered-arrow",)
BODY = (740, 540, 1690, 1090)
START = (800, 600)
END = (1600, 1000)
PARK = (3400, 2000)


def wait_for_text(support, before, timeout=3):
    """【工具内存对照】【文字交付】按support和before等待两版默认红色正文实际呈现，timeout限制秒数，返回新增实色文字像素，未交付时失败"""
    color = (255, 77, 77)
    initial = sum(pixel == color for pixel in before.crop(BODY).get_flattened_data())
    deadline = time.monotonic() + timeout
    # 1. 【工具内存对照】【可见确认】两版异步读取剪贴板，必须先看到正文再结束编辑，空输入框和光标不能替代文字
    while True:
        image = support.raw_capture()
        added = sum(pixel == color for pixel in image.crop(BODY).get_flattened_data()) - initial
        if added >= 200:
            return added
        assert time.monotonic() < deadline, "the pasted text did not become visible before committing"
        time.sleep(0.04)


def draw(support, before, destination, tool):
    """【工具内存对照】【正文绘制】按support和tool执行固定手势，以before检查正文，destination保存结果，返回变化像素和可见耗时"""
    started = time.monotonic()
    # 1. 【工具内存对照】【真实创建】两版使用同一点击、粘贴或拖动，文字提交后只结束文字编辑，不结束截图
    if tool == "text":
        support.SCROLL.INPUT.click(START)
        support.session.paste_text("Memory comparison")
        text_pixels = wait_for_text(support, before)
        support.session.key("Escape")
    elif tool == "number":
        support.SCROLL.INPUT.click(START)
    else:
        support.SCROLL.INPUT.drag(START, END)
    support.SCROLL.INPUT.move(PARK)
    result = capture_stable(support.raw_capture, before, BODY, destination)
    if tool == "text":
        result["visible_text_pixels_before_commit"] = text_pixels
    result["visible_seconds_including_test_capture"] = time.monotonic() - started
    return result
