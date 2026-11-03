"""【文字回归】【透明校验】区分圆角边缘量化与正文泄漏，不放宽文字区域透明度断言"""


def transparent_text_is_absent(image, bounds, source):
    """【文字回归】【透明正文】检查 image 的 bounds 内是否没有红色正文；source 为源图 RGB，只允许四像素圆角区域出现各通道加一的边缘量化，返回布尔值"""
    left, top, right, bottom = bounds
    quantized = tuple(channel + 1 for channel in source)
    for y in range(top, bottom):
        for x in range(left, right):
            pixel = image.getpixel((x, y))
            if pixel[0] <= source[0]:
                continue
            corner = (x < left + 4 or x >= right - 4) and (y < top + 4 or y >= bottom - 4)
            if pixel != quantized or not corner:
                return False
    return True
