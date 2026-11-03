"""【取色器定位】【可见特征】使用横向色相与十六进制框定位原版布局，不依赖旧主题蓝灰边框"""

from colorsys import hsv_to_rgb


OFFSETS = (24, 52, 82, 112, 142, 172)
SAMPLES = [tuple(round(value * 255) for value in hsv_to_rgb((x - 8.5) / 182, 1, 1)) for x in OFFSETS]


def color_popover_rectangle(image, _border=None):
    """【取色器定位】【颜色矩形】在 image 中验证完整色相轨道及对应文本框；返回唯一二百乘二百二十八矩形，缺失或歧义时报错"""
    image = image.convert("RGB")
    pixels = image.load()
    matches = []
    # 1. 【取色器定位】【轨道候选】青色中点只产生候选，六处色相样本排除普通颜色色块与任意矩形
    for y in range(128, image.height - 90):
        for x in range(100, image.width - 99):
            red, green, blue = pixels[x, y]
            if red > 40 or min(green, blue) < 200 or abs(green - blue) > 35:
                continue
            left = x - 100
            errors = sorted(sum(abs(a - b) for a, b in zip(pixels[left + offset, y], expected)) / 3 for offset, expected in zip(OFFSETS, SAMPLES))
            score = sum(errors[:5]) / 5
            if score <= 20:
                matches.append((left, y, score))
    groups = []
    for match in matches:
        left, y, _score = match
        group = next((group for group in groups if abs(left - group[-1][0]) <= 6 and 0 <= y - group[-1][1] <= 1), None)
        if group is None:
            groups.append([match])
        else:
            group.append(match)
    candidates = []
    # 2. 【取色器定位】【独立核对】色相高度必须吻合，右侧文本框必须与相邻底板形成实际填充差，孤立色阶不提供猜测坐标
    for group in groups:
        rows = {match[1] for match in group}
        if not 5 <= len(rows) <= 11:
            continue
        left = min(group, key=lambda match: match[2])[0]
        top = min(rows) - 128
        if top < 0 or top + 228 > image.height:
            continue
        field = pixels[left + 180, top + 175]
        background = pixels[left + 180, top + 160]
        if sum(abs(a - b) for a, b in zip(field, background)) < 20:
            continue
        rectangle = (left, top, left + 200, top + 228)
        if not any(max(abs(a - b) for a, b in zip(rectangle, other)) <= 4 for other in candidates):
            candidates.append(rectangle)
    if len(candidates) != 1:
        raise ValueError(f"cannot uniquely locate the native color popover: {candidates}")
    return candidates[0]
