"""【结果回归】【单行坐标】集中保存 Qt 标题栏期望命中点，不从旧的两行布局定位控件"""

from functools import lru_cache
import json
from pathlib import Path


def window_rect(window):
    """【结果回归】【矩形统一】按 window 返回普通矩形或 niri 浮动窗口的逻辑位置和尺寸，不访问外部桌面"""
    if "width" in window:
        return window
    left, top = window["floating_pos"]
    width, height = window["size"]
    return {"x": left, "y": top, "width": width, "height": height}


def tab_point(window, view, has_table=True, language="en"):
    """【结果回归】【页签位置】按 window、view、has_table 和 language 返回私有全局命中点，采用原版语言宽度并避开窄窗口裁切"""
    window = window_rect(window)
    left = 32 if window["width"] >= 500 else 8
    text_width, table_width = (42, 42) if language == "zh" else (43, 48)
    offset = {"Text": 0, "Table": text_width, "Image": text_width + (table_width if has_table else 0)}[view]
    return window["x"] + left + offset + 10, window["y"] + 19


def source_right(window, view="Text"):
    """【结果回归】【原文边界】按 window 和 view 返回原文复制右边界，非文字页省略翻译槽位，反馈只改变左边界"""
    window = window_rect(window)
    return window["x"] + window["width"] - (136 if view == "Text" else 104)


def source_point(window, view="Text"):
    """【结果回归】【原文复制】按 window 和 view 返回复制内部点，成功及失败文案变化后仍位于按钮内"""
    window = window_rect(window)
    return source_right(window, view) - 10, window["y"] + 19


def table_point(window):
    """【结果回归】【表格复制】按 window 返回独立表格按钮内部点，不借用标题栏复制入口"""
    window = window_rect(window)
    return window["x"] + window["width"] - 20, window["y"] + 54


@lru_cache(maxsize=3)
def table_reference(width):
    """【结果回归】【表格记录】按 width 读取原版固定内容表格的初始几何，返回状态，未知宽度拒绝猜测坐标"""
    fixture = Path(__file__).resolve().parents[1] / "tests/fixtures/qt-ocr-table-grid.json"
    cases = json.loads(fixture.read_text())["cases"]
    for case in cases:
        if case["theme"] == "dark" and case["kind"] == "content" and case["width"] == width:
            return case["states"][0]["state"]
    raise ValueError(f"No original table geometry for width {width}")


def table_cell_point(window, row=0, column=0):
    """【结果回归】【单元格坐标】按 window、row、column 返回真实原版单元格中心，避开数字表头与分隔线，不使用旧网格坐标"""
    window = window_rect(window)
    reference = table_reference(window["width"])
    cell = next(cell for cell in reference["cells"] if cell["row"] == row and cell["column"] == column)
    table, viewport, rect = reference["table"], reference["viewport"], cell["rect"]
    return (window["x"] + table[0] + viewport[0] + rect[0] + rect[2] / 2,
            window["y"] + table[1] + viewport[1] + rect[1] + rect[3] / 2)
