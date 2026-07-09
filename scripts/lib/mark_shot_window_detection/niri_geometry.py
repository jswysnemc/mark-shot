import os


def env_int(name):
    """读取整数环境变量。
    参数 name: 环境变量名称
    返回: 整数值，缺失或格式错误时返回 None
    """
    value = os.environ.get(name)
    if value is None or value == "":
        return None
    try:
        return int(value)
    except ValueError:
        return None


def intersects(left, right):
    """判断两个矩形是否相交。
    参数 left: 第一个矩形; right: 第二个矩形
    返回: 相交时返回 True
    """
    ax, ay, aw, ah = left
    bx, by, bw, bh = right
    return ax < bx + bw and bx < ax + aw and ay < by + bh and by < ay + ah


def clipped_rect(rect, bounds):
    """将矩形裁剪到边界内。
    参数 rect: 待裁剪矩形; bounds: 边界矩形
    返回: 裁剪后的矩形，无有效交集时返回 None
    """
    ax, ay, aw, ah = rect
    bx, by, bw, bh = bounds
    x = max(ax, bx)
    y = max(ay, by)
    right = min(ax + aw, bx + bw)
    bottom = min(ay + ah, by + bh)
    width = right - x
    height = bottom - y
    if width <= 1 or height <= 1:
        return None
    return int(round(x)), int(round(y)), int(round(width)), int(round(height))


def rect_adjustment():
    """读取 niri 窗口矩形手工偏移。
    返回: X、Y、宽度和高度调整值
    """
    return (
        env_int("MARK_SHOT_NIRI_OFFSET_X") or 0,
        env_int("MARK_SHOT_NIRI_OFFSET_Y") or 0,
        env_int("MARK_SHOT_NIRI_OFFSET_WIDTH") or 0,
        env_int("MARK_SHOT_NIRI_OFFSET_HEIGHT") or 0,
    )


def adjusted_rect(rect):
    """应用 niri 窗口矩形手工偏移。
    参数 rect: 原始矩形
    返回: 调整后的矩形，无效时返回 None
    """
    dx, dy, dw, dh = rect_adjustment()
    x, y, width, height = rect
    adjusted = (x + dx, y + dy, width + dw, height + dh)
    if adjusted[2] <= 1 or adjusted[3] <= 1:
        return None
    return adjusted


def append_rect(result, seen, rect):
    """将有效且未重复的矩形追加到输出列表。
    参数 result: 输出列表; seen: 去重集合; rect: 矩形
    返回: 无返回值
    """
    if rect[2] <= 1 or rect[3] <= 1:
        return
    key = tuple(rect)
    if key in seen:
        return
    seen.add(key)
    result.append({
        "x": rect[0],
        "y": rect[1],
        "width": rect[2],
        "height": rect[3],
    })


def append_candidate(result, seen, rect, capture):
    """调整、裁剪并追加窗口候选矩形。
    参数 result: 输出列表; seen: 去重集合; rect: 原始矩形; capture: 捕获边界
    返回: 无返回值
    """
    rect = adjusted_rect(rect)
    if rect is None:
        return
    if capture is not None:
        if not intersects(rect, capture):
            return
        rect = clipped_rect(rect, capture)
        if rect is None:
            return
    append_rect(result, seen, rect)


def output_origin(outputs, output_name):
    """读取输出的全局逻辑原点。
    参数 outputs: niri 输出字典; output_name: 输出名称
    返回: X、Y 坐标
    """
    output = outputs.get(output_name)
    if not isinstance(output, dict):
        return 0, 0
    logical = output.get("logical")
    if not isinstance(logical, dict):
        return 0, 0
    return int(round(logical.get("x", 0))), int(round(logical.get("y", 0)))


def output_size(outputs, output_name):
    """读取输出的逻辑尺寸。
    参数 outputs: niri 输出字典; output_name: 输出名称
    返回: 宽度和高度
    """
    output = outputs.get(output_name)
    if not isinstance(output, dict):
        return 0, 0
    logical = output.get("logical")
    if not isinstance(logical, dict):
        return 0, 0
    return int(round(logical.get("width", 0))), int(round(logical.get("height", 0)))


def output_rect(outputs, output_name):
    """读取输出的完整逻辑矩形。
    参数 outputs: niri 输出字典; output_name: 输出名称
    返回: 输出矩形，无效时返回 None
    """
    width, height = output_size(outputs, output_name)
    if width <= 0 or height <= 0:
        return None
    x, y = output_origin(outputs, output_name)
    return x, y, width, height


def union_rect(rects):
    """计算多个矩形的并集。
    参数 rects: 矩形迭代器
    返回: 并集矩形，没有有效矩形时返回 None
    """
    valid = [rect for rect in rects if rect is not None]
    if not valid:
        return None
    left = min(rect[0] for rect in valid)
    top = min(rect[1] for rect in valid)
    right = max(rect[0] + rect[2] for rect in valid)
    bottom = max(rect[1] + rect[3] for rect in valid)
    return left, top, right - left, bottom - top
