"""【OCR 表格滚动】【证据断言】原版坐标、滚动后单元格和实际滚动条像素，不读取应用内部状态"""

import json
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parents[1]


def reference(theme):
    """【OCR 表格滚动】【原版状态】按 theme 返回原版 760 像素双向表格状态，不用原生布局构造期望值"""
    cases = json.loads((ROOT / "tests/fixtures/qt-ocr-table-scroll.json").read_text())["cases"]
    case = next(case for case in cases if case["theme"] == theme and case["width"] == 760 and case["kind"] == "both")
    return {entry["name"]: entry["state"] for entry in case["states"]}


def frame():
    """【OCR 表格滚动】【结果传输】返回与原版探针对应的四十行八列实际文本及 PNG，不初始化 OCR 模型"""
    lines = [{"text": f"A long column value {row * 8 + column}", "bounds": [38 + column * 3000, 32 + row * 66, 40, 24]}
             for row in range(40) for column in range(8)]
    header = json.dumps({"text": "business source", "lines": lines}).encode()
    png = (ROOT / "tests/fixtures/ocr-table.png").read_bytes()
    return b"MSOCR001" + struct.pack("<II", len(header), len(png)) + header + png


def point(window, state, axis, part):
    """【OCR 表格滚动】【控件中心】按 window、state、axis、part 返回真实 Qt 滚动条子区域中心，零大小部件拒绝定位"""
    bar = state[axis]
    rect = bar["controls"][part]
    assert rect[2] > 0 and rect[3] > 0, f"{axis} {part} has no original hit area"
    return (window["x"] + 8 + bar["rect"][0] + rect[0] + rect[2] / 2,
            window["y"] + 75 + bar["rect"][1] + rect[1] + rect[3] / 2)


def front(window, state):
    """【OCR 表格滚动】【可见单元格】按 window、state 返回当前视区内部点及原版期望完整单元格文字，不只比较滑块移动"""
    viewport, offset = state["viewport"], state["offset"]
    x, y = offset[0] + 15, offset[1] + 14
    row = int(y // 28)
    left = 0
    for column, width in enumerate(state["columns"]):
        if left <= x < left + width:
            break
        left += width
    else:
        raise AssertionError("the original point did not land in a column")
    return ((window["x"] + 8 + viewport[0] + 15, window["y"] + 75 + viewport[1] + 14),
            f"A long column value {row * 8 + column}")


def horizontal_pixels(image, window, state, theme, pressed=False):
    """【OCR 表格滚动】【横向条带】按 image、window、state、theme、pressed 比较原版滑块中心十四行实际像素，返回证据"""
    profiles = json.loads((ROOT / "tests/fixtures/qt-ocr-table-scroll-paint.json").read_text())[theme]
    slider = state["horizontal"]["controls"]["slider"]
    bounds = state["horizontal"]["rect"]
    x = round(window["x"] + 8 + bounds[0] + slider[0] + slider[2] / 2)
    y = round(window["y"] + 75 + bounds[1])
    actual = [list(image.getpixel((x, y + row))) for row in range(14)]
    expected = [color[:3] for color in profiles["pressed" if pressed else "normal"]]
    assert actual == expected, f"{theme} horizontal thumb pixels differ: {actual} != {expected}"
    return {"x": x, "y": y, "actual": actual, "expected": expected}


def vertical_pixel(image, window, state, theme, visible):
    """【OCR 表格滚动】【纵向悬停】按 image、window、state、theme、visible 检查滑块中心实际像素，返回证据，不以存在滚动条代替显示"""
    position = point(window, state, "vertical", "slider")
    actual = image.getpixel(tuple(round(value) for value in position))
    expected = (203, 213, 225) if theme == "light" else (51, 65, 85)
    if not visible:
        expected = (255, 255, 255) if theme == "light" else (15, 23, 42)
    assert actual == expected, f"{theme} vertical visible={visible}: {actual} != {expected}"
    return {"point": position, "actual": actual, "expected": expected}
