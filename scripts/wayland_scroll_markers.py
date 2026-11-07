"""【长截图双框回归】【实际像素】在已观察面板内定位连续黄色及青色边线，排除原图随机颜色"""


def bounds(image, buttons, color):
    """【长截图双框回归】【边线定位】按image、buttons和color查找面板连续边线，返回像素范围；无足够边线时失败"""
    left = max(0, round(buttons["axis"][0] - 30))
    right = min(image.width, round(buttons["cancel"][0] + 30))
    bottom = round(buttons["save"][1] - 20)
    points = []
    for vertical in (False, True):
        for fixed in (range(left, right) if vertical else range(bottom)):
            start = None
            begin, end = (0, bottom) if vertical else (left, right)
            for varying in range(begin, end + 1):
                matched = False
                if varying < end:
                    x, y = (fixed, varying) if vertical else (varying, fixed)
                    r, g, b = image.getpixel((x, y))
                    matched = (r > 180 and 150 < g < 235 and b < 110) if color == "yellow" else (r < 90 and g > 165 and b > 145)
                if matched and start is None:
                    start = varying
                if not matched and start is not None:
                    if varying - start >= 12:
                        points.extend([(fixed, start), (fixed, varying - 1)] if vertical else [(start, fixed), (varying - 1, fixed)])
                    start = None
    assert len(points) >= 2, f"{color} overview marker has no continuous visible edges"
    return [min(x for x, y in points), min(y for x, y in points),
            max(x for x, y in points) + 1, max(y for x, y in points) + 1]


def check(image, buttons):
    """【长截图双框回归】【双色共存】返回实际黄色捕获框与青色浏览框范围，缺失任一框立即失败"""
    return {color: bounds(image, buttons, color) for color in ("yellow", "cyan")}
