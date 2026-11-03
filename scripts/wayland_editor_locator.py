"""【Wayland 测试】【属性面板定位】通过实际边框连通区域定位原生编辑器属性控件"""

from collections import deque
from wayland_color_locator import color_popover_rectangle as color_popover_rectangle


def border_pixel(pixel, border):
    """【界面定位】【抗锯齿边框】判断 pixel 是否保留 border 的蓝灰色差；返回布尔值，排除相邻灰色选区边框"""
    blue = pixel[2] - pixel[0]
    green = pixel[1] - pixel[0]
    ratio = (border[2] - border[0]) / max(1, border[1] - border[0])
    return max(abs(pixel[channel] - border[channel]) for channel in range(3)) <= 64 and blue >= (border[2] - border[0]) * 0.3 and green >= (border[1] - border[0]) * 0.25 and blue >= green * ratio * 0.9


def _border_components(image, border):
    """【界面定位】【边框片段】在 image 中查找 border 附近颜色的连通区域；返回各片段矩形与像素数，不预先过滤高度"""
    image = image.convert("RGB")
    width = image.width
    pixels = image.load()
    points = {y * width + x for y in range(image.height) for x in range(width) if border_pixel(pixels[x, y], border)}
    components = []
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
            for dx, dy in ((-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)):
                adjacent_x, adjacent_y = x + dx, y + dy
                if not 0 <= adjacent_x < width or not 0 <= adjacent_y < image.height:
                    continue
                adjacent = adjacent_y * width + adjacent_x
                if adjacent in points:
                    points.remove(adjacent)
                    queue.append(adjacent)
        components.append(((left, top, right + 1, bottom + 1), count))
    return components


def _deduplicate_rectangles(rectangles, height):
    """【界面定位】【重复矩形】合并 rectangles 中相差不超过六像素的同一边框；返回最接近 height 的候选，保留不同位置的歧义"""
    groups = []
    for rectangle in rectangles:
        matching = next((group for group in groups if any(max(abs(first - second) for first, second in zip(rectangle, other)) <= 6 for other in group)), None)
        if matching is None:
            groups.append([rectangle])
        else:
            matching.append(rectangle)
    result = [min(group, key=lambda rect: (abs(rect[3] - rect[1] - height), -(rect[2] - rect[0]) * (rect[3] - rect[1]))) for group in groups]
    return sorted(result, key=lambda rect: (rect[1], rect[0]))


def panel_rectangles(image, border, height=42):
    """【界面定位】【面板矩形】在 image 中查找 border 完整边框或几何一致的上下片段；返回 height 高度候选，不猜测缺失边框"""
    components = _border_components(image, border)
    pieces = [rect for rect, count in components if rect[2] - rect[0] >= 120 and count >= 100]
    rectangles = [rect for rect in pieces if abs(rect[3] - rect[1] - height) <= 3]
    # 1. 【界面定位】【面板矩形】缩放可能削弱侧边，但上下长边的重叠、左右位置与总高度仍须一致
    for first in pieces:
        for second in pieces:
            if first[1] >= second[1] or first[3] > second[3]:
                continue
            if abs(first[0] - second[0]) > 24 or abs(first[2] - second[2]) > 24:
                continue
            overlap = min(first[2], second[2]) - max(first[0], second[0])
            if overlap < min(first[2] - first[0], second[2] - second[0]) * 0.85:
                continue
            combined = (min(first[0], second[0]), first[1], max(first[2], second[2]), second[3])
            if abs(combined[3] - combined[1] - height) <= 3:
                rectangles.append(combined)
    # 2. 【界面定位】【纹理分段】完整高度的左侧片段、相接顶部与独立右竖边共同组成外框，不能只凭两条无关线猜测面板
    partials = [rect for rect, count in components if 6 <= rect[2] - rect[0] < 120 and abs(rect[3] - rect[1] - height) <= 3 and count >= 60]
    tops = [rect for rect, count in components if rect[2] - rect[0] >= 120 and rect[3] - rect[1] <= 14 and count >= 80]
    stems = [rect for rect, count in components if rect[2] - rect[0] <= 6 and rect[3] - rect[1] >= height / 2 and count >= 18]
    for partial in partials:
        for top in tops:
            if not 0 <= top[0] - partial[2] <= 3 or abs(top[1] - partial[1]) > 3:
                continue
            for stem in stems:
                if abs(stem[0] - top[2]) > 3 or stem[1] <= top[1] or not 0 <= partial[3] - stem[3] <= 12:
                    continue
                rectangle = (partial[0], min(partial[1], top[1]), max(top[2], stem[2]), partial[3])
                if abs(rectangle[3] - rectangle[1] - height) <= 3:
                    rectangles.append(rectangle)
    return _deduplicate_rectangles(rectangles, height)


def settings_rectangle(image, border):
    """【界面定位】【属性矩形】从 image 的 border 连通框中定位工具栏下方属性面板；返回唯一矩形或抛出错误"""
    panels = panel_rectangles(image, border)
    candidates = [rect for rect in panels if any(other[1] < rect[1] and other[3] <= rect[1] + 3 and other[2] - other[0] > rect[2] - rect[0] for other in panels)]
    if len(candidates) != 1:
        raise ValueError(f"cannot uniquely locate the visible settings panel: {panels}")
    return candidates[0]


def marker_controls(image, border):
    """【界面定位】【标记控件】根据 image 的真实属性矩形返回形状按钮、填充开关与笔宽输入的逻辑坐标"""
    left, top, right, bottom = settings_rectangle(image, border)
    middle = round((top + bottom) / 2)
    return {
        "shape": (round(left + 8 + 28 + 8 + 36 / 2), middle),
        "width": (round(left + 8 + 28 + 8 + 36 + 8 + 16 + 8 + 84 / 3), middle),
        "fill": (round(right - 8 - 24), middle),
    }


def ruler_controls(image, border):
    """【界面定位】【尺子控件】根据 image 的真实属性矩形返回模式按钮与笔宽输入的逻辑坐标"""
    left, top, _right, bottom = settings_rectangle(image, border)
    middle = round((top + bottom) / 2)
    mode_width = 28 + 7 * 7
    return {
        "mode": (round(left + 8 + 28 + 8 + mode_width / 2), middle),
        "width": (round(left + 8 + 28 + 8 + mode_width + 8 + 16 + 84 / 3), middle),
    }


def magnifier_controls(image, border):
    """【界面定位】【放大镜控件】根据 image 的真实属性框返回独立笔宽、倍率输入与矩形开关的逻辑坐标"""
    left, top, right, bottom = settings_rectangle(image, border)
    middle = round((top + bottom) / 2)
    return {
        "width": (round(left + 8 + 28 + 8 + 84 / 3), middle),
        "zoom": (round(left + 8 + 28 + 8 + 84 + 8 + 16 + 84 / 3), middle),
        "shape": (round(right - 8 - 14), middle),
    }


def number_controls(image, border):
    """【界面定位】【序号控件】从 image 的真实 border 属性框定位样式、独立笔宽和重置按钮；返回逻辑坐标字典"""
    left, top, right, bottom = settings_rectangle(image, border)
    middle = round((top + bottom) / 2)
    style_width = 28 + 3 * 7
    return {
        "style": (round(left + 8 + 28 + 8 + style_width / 2), middle),
        "width": (round(left + 8 + 28 + 8 + style_width + 8 + 16 + 84 / 3), middle),
        "reset": (round(right - 8 - 14), middle),
    }


def line_controls(image, border, arrow=True):
    """【界面定位】【线条控件】从 image 的 border 属性框返回线型和笔宽位置；arrow 决定是否包含头部按钮"""
    left, top, _right, bottom = settings_rectangle(image, border)
    middle = round((top + bottom) / 2)
    style_width = 28 + 16
    first_style = left + 8 + 28 + 8
    pattern = first_style + (style_width + 8 if arrow else 0)
    controls = {
        "pattern": (round(pattern + style_width / 2), middle),
        "width": (round(pattern + style_width + 8 + 16 + 84 / 3), middle),
    }
    if arrow:
        controls["head"] = (round(first_style + style_width / 2), middle)
    return controls


def filter_controls(image, border):
    """【界面定位】【滤镜控件】从 image 的真实 border 属性框返回透明度色块、原图强度和模式按钮坐标"""
    left, top, _right, bottom = settings_rectangle(image, border)
    middle = round((top + bottom) / 2)
    return {
        "opacity": (round(left + 8 + 14), middle),
        "strength": (round(left + 8 + 28 + 8 + 16 + 84 / 3), middle),
        "mode": (round(left + 8 + 28 + 8 + 16 + 84 + 8 + 16 + (28 + 16) / 2), middle),
    }
