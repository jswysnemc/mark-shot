"""【Wayland 测试】【模型按钮定位】根据默认主题的推荐条目边框与下载列定位实际按钮"""

from pathlib import Path
import tomllib


PICKER = tomllib.loads((Path(__file__).resolve().parents[1] / "src/config/default.toml").read_text())["model_picker"]


def horizontal_runs(positions):
    """【模型定位】【横线合并】将 positions 中间隔不超过两像素的坐标合并；返回起止坐标迭代器"""
    left = previous = None
    for position in positions:
        if previous is not None and position - previous > 2:
            yield left, previous
            left = None
        if left is None:
            left = position
        previous = position
    if left is not None:
        yield left, previous


def green(pixel):
    """【模型定位】【主题筛选】判断 pixel 是否保留原版半透明青色边框的通道差；返回布尔值，不依赖不透明旧主题"""
    red, green, blue = pixel
    return green >= 40 and green - red >= 10 and blue - red >= 10


def download_column(image, x, first_y, index):
    """【模型定位】【下载列校验】核对 image 中 x 列从 first_y 到 index 的下载图标；返回布尔值"""
    pixels = image.load()
    radius = round(PICKER["icon_size"] / 2 + 4)
    for row in range(index + 1):
        y = round(first_y + row * PICKER["row_height"])
        if not radius <= x < image.width - radius or not radius <= y < image.height - radius:
            return False
        light = 0
        for top in range(y - radius, y + radius + 1):
            for left in range(x - radius, x + radius + 1):
                color = pixels[left, top]
                if min(color) >= 160 and max(color) - min(color) < 75:
                    light += 1
        if light < 12:
            return False
    return True


def download_position(image, index=4):
    """【模型定位】【下载坐标】在 image 中定位 index 对应按钮；测试固定英语区域设置，返回逻辑坐标或抛出错误"""
    if not isinstance(index, int) or index < 0:
        raise ValueError("invalid model index")
    image = image.convert("RGB")
    pixels = image.load()
    row_width = PICKER["width"] - PICKER["padding"] * 2

    # 1. 推荐条目固定为首行；通过实际绿色边框定位，不依赖工具栏按钮数量或弹层绝对位置
    for y in range(image.height):
        candidates = (x for x in range(image.width) if green(pixels[x, y]))
        for left, right in horizontal_runs(candidates):
            if not row_width * 0.8 <= right - left + 1 <= row_width + 4:
                continue
            x = round(right + 1 - PICKER["row_padding"] / 2 - PICKER["button_size"] / 2)
            first_y = y + PICKER["row_height"] / 2

            # 2. 多行下载图标必须对齐，避免把源图中的矩形或正文线条误当作模型列表
            if download_column(image, x, first_y, max(index, 2)):
                return x, round(first_y + index * PICKER["row_height"])
    raise ValueError("cannot locate the visible model download controls")
