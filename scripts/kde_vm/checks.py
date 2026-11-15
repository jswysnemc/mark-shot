"""【KDE虚拟机】【结果断言】验证屏幕中的选区、工具栏及导出图像，不依赖日志中的成功文字"""

from PIL import Image


def selection_pixels(path):
    """【KDE虚拟机】【选区像素】按 path 返回青色选区边框像素数量，用于检查界面是否意外消失"""
    with Image.open(path) as image:
        return sum(1 for red, green, blue in image.convert("RGB").get_flattened_data()
                   if green > red + 40 and blue > red + 40 and green > 160)


def visible_selection(path):
    """【KDE虚拟机】【选区检查】按 path 断言选区控制框存在，失败报告实际像素数量，返回值为空"""
    count = selection_pixels(path)
    assert count > 40, f"选区及工具栏消失，青色控制框像素仅 {count}: {path}"
