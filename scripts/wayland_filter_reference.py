"""【滤镜测试】【独立像素参考】用整数数组复核原图滤镜与预乘覆盖，不调用应用实现"""

import math

import numpy as np
from PIL import Image


MODES = ["pixelate", "blur", "grayscale", "invert", "brighten"]


def premultiplied(image):
    """【滤镜参考】【预乘转换】把 image 转为预乘 RGBA 整数数组；返回独立数组，透明像素清除隐藏颜色"""
    values = np.asarray(image.convert("RGBA"), dtype=np.uint32).copy()
    values[..., :3] = (values[..., :3] * values[..., 3:4] + 127) // 255
    return values


def straight(values):
    """【滤镜参考】【直通转换】反预乘 values，返回直通 RGBA 数组，完全透明通道保持为零"""
    result = values.astype(np.uint32).copy()
    alpha = result[..., 3:4]
    result[..., :3] = np.minimum(255, (result[..., :3] * 255 + alpha // 2) // np.maximum(alpha, 1))
    result[..., :3] = np.where(alpha != 0, result[..., :3], 0)
    return result


def box_pass(values, radius, axis):
    """【滤镜参考】【盒式处理】沿 axis 以 radius 处理 values；返回逐次向下取整、边缘夹取的预乘数组"""
    padding = [(0, 0), (0, 0), (0, 0)]
    padding[axis] = (radius, radius)
    padded = np.pad(values.astype(np.int64), padding, mode="edge")
    summed = np.cumsum(padded, axis=axis, dtype=np.int64)
    shape = list(summed.shape)
    shape[axis] = 1
    summed = np.concatenate([np.zeros(shape, dtype=np.int64), summed], axis=axis)
    window = radius * 2 + 1
    upper = [slice(None)] * 3
    lower = [slice(None)] * 3
    upper[axis] = slice(window, None)
    lower[axis] = slice(None, -window)
    return ((summed[tuple(upper)] - summed[tuple(lower)]) // window).astype(np.uint32)


def gaussian(values, radius):
    """【滤镜参考】【三次模糊】按原版 radius 推导三次盒式半径，返回预乘 RGBA；单行或单列保持不变"""
    if values.shape[0] < 2 or values.shape[1] < 2 or radius < 1:
        return values.copy()
    sigma = max(radius / 2, 1)
    lower = math.floor(math.sqrt(4 * sigma * sigma + 1))
    if lower % 2 == 0:
        lower -= 1
    lower = max(lower, 1)
    count = math.floor((12 * sigma * sigma - 3 * lower * lower - 12 * lower - 9) / (-4 * lower - 4) + 0.5)
    result = values.copy()
    for index in range(3):
        half = ((lower if index < count else lower + 2) - 1) // 2
        if half:
            result = box_pass(box_pass(result, half, 1), half, 0)
    return result


def filtered(source, rect, mode, strength):
    """【滤镜参考】【区域效果】按 rect、mode、strength 处理冻结 source；返回预乘区域数组，未知模式报告错误"""
    left, top, right, bottom = rect
    if not math.isfinite(strength) or not 4 <= strength <= 48 or mode not in MODES:
        raise ValueError("invalid filter mode or native strength")
    values = premultiplied(source)
    region = values[top:bottom, left:right].copy()
    if mode == "pixelate":
        block = math.floor(strength + 0.5)
        for y in range(0, region.shape[0], block):
            for x in range(0, region.shape[1], block):
                part = region[y:y + block, x:x + block]
                count = part.shape[0] * part.shape[1]
                part[:] = (part.sum(axis=(0, 1), dtype=np.uint64) + count // 2) // count
        return region
    if mode == "blur":
        radius = math.floor(strength + 0.5)
        padded = (max(0, left - radius * 2), max(0, top - radius * 2),
                  min(source.width, right + radius * 2), min(source.height, bottom + radius * 2))
        blurred = gaussian(values[padded[1]:padded[3], padded[0]:padded[2]], radius)
        return blurred[top - padded[1]:bottom - padded[1], left - padded[0]:right - padded[0]].copy()
    unmultiplied = straight(region)
    rgb = unmultiplied[..., :3]
    if mode == "grayscale":
        gray = (rgb[..., 0] * 11 + rgb[..., 1] * 16 + rgb[..., 2] * 5) // 32
        rgb = np.repeat(gray[..., None], 3, axis=2)
    elif mode == "invert":
        rgb = 255 - rgb
    else:
        rgb = np.floor(rgb + (255 - rgb) * min(0.9, max(0.05, strength / 60)) + 0.5).astype(np.uint32)
    product = rgb * unmultiplied[..., 3:4] + 128
    region[..., :3] = (product + (product >> 8)) >> 8
    return region


def overlay(destination, effect, opacity=255):
    """【滤镜参考】【覆盖合成】把预乘 effect 以 opacity 合成到 destination；返回新数组，保留透明度小数直到最终量化"""
    if not math.isfinite(opacity) or not 0 <= opacity <= 255:
        raise ValueError("invalid annotation opacity")
    # 1. 【滤镜参考】【覆盖合成】绘制器在浮点通道中应用透明度与覆盖，不提前把效果通道量化为八位整数
    effect = effect.astype(np.float64) * (opacity / 255)
    combined = effect + destination.astype(np.float64) * (1 - effect[..., 3:4] / 255)
    return np.floor(np.minimum(255, combined) + 0.5).astype(np.uint32)


def expected_export(source, annotations):
    """【滤镜参考】【完整导出】按 annotations 原图区域与层序合成 source；返回 RGBA 图像，不把先前标注当作滤镜来源"""
    result = premultiplied(source)
    for annotation in annotations:
        left, top, right, bottom = annotation["native"]
        effect = filtered(source, (left, top, right, bottom), annotation["mode"], annotation["strength"])
        result[top:bottom, left:right] = overlay(result[top:bottom, left:right], effect, annotation.get("opacity", 255))
    return Image.fromarray(straight(result).astype(np.uint8))


def assert_region(actual, expected, region, tolerance=1):
    """【滤镜参考】【区域断言】验证 actual、expected 在 region 内的颜色差与完全相同的透明度；tolerance 仅允许编码舍入"""
    actual = np.asarray(actual.convert("RGBA").crop(region), dtype=np.int16)
    expected = np.asarray(expected.convert("RGBA").crop(region), dtype=np.int16)
    difference = np.abs(actual - expected)
    failures = np.any(difference[..., :3] > tolerance, axis=2) | (difference[..., 3] != 0)
    if np.any(failures):
        y, x = np.argwhere(failures)[0]
        raise AssertionError(f"native pixel mismatch at {(int(x) + region[0], int(y) + region[1])}: {actual[y, x].tolist()} != {expected[y, x].tolist()}")
