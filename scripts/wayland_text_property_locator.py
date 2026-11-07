"""【文字属性测试】【控件定位】从真实颜色按钮和搜索边框定位面板，不读取应用内部状态"""

from collections import deque


def components(image, matches):
    """【文字属性测试】【颜色区域】按 matches 找到 image 中连通颜色区域，返回矩形和像素数"""
    width, height = image.size
    pixels = image.load()
    points = {y * width + x for y in range(height) for x in range(width) if matches(pixels[x, y])}
    result = []
    while points:
        first = points.pop()
        queue = deque([first])
        left = right = first % width
        top = bottom = first // width
        count = 0
        while queue:
            index = queue.popleft()
            x, y = index % width, index // width
            left, right = min(left, x), max(right, x)
            top, bottom = min(top, y), max(bottom, y)
            count += 1
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)):
                if not 0 <= x + dx < width or not 0 <= y + dy < height:
                    continue
                adjacent = (y + dy) * width + x + dx
                if adjacent in points:
                    points.remove(adjacent)
                    queue.append(adjacent)
        result.append(((left, top, right + 1, bottom + 1), count))
    return result


def font_button(image):
    """【文字属性测试】【字体按钮】由 image 的唯一不透明红色属性按钮返回字体按钮中心，定位失败抛出异常"""
    found = [rect for rect, count in components(image, lambda pixel: abs(pixel[0] - 255) <= 2 and abs(pixel[1] - 77) <= 2 and abs(pixel[2] - 77) <= 2)
             if 22 <= rect[2] - rect[0] <= 32 and 22 <= rect[3] - rect[1] <= 32 and count > 250 and count / ((rect[2] - rect[0]) * (rect[3] - rect[1])) > 0.70]
    if len(found) != 1:
        raise AssertionError(f"the text color button is ambiguous: {found}")
    left, top, right, bottom = found[0]
    return (left + right) / 2 + 76, (top + bottom) / 2


def font_popup(image):
    """【文字属性测试】【弹层边框】由 image 的搜索边框或真实字号输入底板返回弹层矩形，拒绝填充色块和其他面板"""
    found = [rect for rect, count in components(image, lambda pixel: pixel[1] > 120 and pixel[1] - pixel[0] > 30 and pixel[2] - pixel[0] > 25)
             if 342 <= rect[2] - rect[0] <= 350 and 24 <= rect[3] - rect[1] <= 30 and 500 <= count <= 1400]
    if not found:
        fields = [rect for rect, count in components(image, lambda pixel: (232 <= min(pixel) <= 245 and max(pixel) - min(pixel) <= 7) or (25 <= min(pixel) <= 65 and max(pixel) - min(pixel) <= 16))
                  if 282 <= rect[2] - rect[0] <= 288 and 21 <= rect[3] - rect[1] <= 26 and 4500 <= count <= 7800]
        if len(fields) == 1:
            left, top, _right, _bottom = fields[0]
            return left - 7, top - 327, left - 7 + 360, top - 327 + 360
    if len(found) != 1:
        raise AssertionError(f"the font search border is ambiguous: {found}")
    left, top, _right, _bottom = found[0]
    return left - 6, top - 7, left - 6 + 360, top - 7 + 360
