"""【滤镜测试】【场景数据】生成独立半透明原图、逻辑布局并交叉验证真实提交坐标"""

import json
import math
import re
import subprocess
import time

import numpy as np
from PIL import Image

from wayland_filter_reference import MODES, premultiplied, straight


BORDER = (96, 113, 127)
SOURCE_SIZE = (1600, 900)
COMMIT = re.compile(r"【滤镜】【区域提交】id=(\d+) mode=(\S+) strength=(\S+) source=\[([^]]+)\] logical=\[([^]]+)\]")


def source_image():
    """【滤镜测试】【原图生成】返回具有高频纹理、透明条带和稳定启动标记的 RGBA 图像，不读取应用像素"""
    y, x = np.indices((SOURCE_SIZE[1], SOURCE_SIZE[0]), dtype=np.uint32)
    pixels = np.empty((*x.shape, 4), dtype=np.uint32)
    pixels[..., 0] = 40 + (x * 7 + y * 3) % 180
    pixels[..., 1] = 16 + (x * 5 + y * 11) % 140
    pixels[..., 2] = 8 + (x * 13 + y * 2) % 120
    pixels[..., 3] = np.array([255, 192, 128, 64], dtype=np.uint32)[(x // 96 + y // 128) % 4]
    pixels[:8] = (0, 0, 0, 0)
    pixels[20:43, 20:43] = (255, 0, 0, 128)
    pixels[420:480, 600:650] = (30, 34, 38, 255)
    pixels[800:890, 1470:1580] = (0, 255, 0, 128)
    # 1. 【滤镜测试】【原图生成】选择预乘往返稳定的直通颜色，使未修改像素可以进行完全一致比较
    image = Image.fromarray(pixels.astype(np.uint8))
    return Image.fromarray(straight(premultiplied(image)).astype(np.uint8))


def layout_for(logical, source):
    """【滤镜测试】【布局计算】根据 logical 输出和 source 尺寸返回矩阵、跨块区域及安全位置；过小输出报告错误"""
    width, height = logical["width"], logical["height"]
    scale = min((width - 64) / source.width, (height - 96) / source.height, 1)
    left, top = (width - source.width * scale) / 2, (height - source.height * scale) / 2
    right, bottom = width - left, height - top
    matrix = []
    cell_width = (right - left - 144 - 4 * 18) / 5
    first_y = math.ceil(top) + 106
    for row, strength in enumerate((4, 14, 48)):
        for column, mode in enumerate(MODES):
            start = (round(left + 72 + column * (cell_width + 18)), first_y + row * 38)
            end = (round(start[0] + cell_width), start[1] + 25)
            matrix.append({"start": start, "end": end, "mode": mode, "strength": strength})
    edit_start = (round(width * 0.32), matrix[-1]["end"][1] + 17)
    edit_end = (round(width * 0.67), math.floor(bottom) - 8)
    if min((edit_end[index] - edit_start[index]) / scale for index in (0, 1)) <= 512:
        raise ValueError("the isolated filter layout must cross both 512-pixel native tile boundaries")
    return {
        "matrix": matrix,
        "viewport": (left, top, scale),
        "size": (width, height),
        "source_size": source.size,
        "safe_y": height - 6,
        "content": (48, math.ceil(top) + 94, width - 48, math.floor(bottom) - 2),
        "panel_region": (48, math.floor(top), width - 48, first_y - 4),
        "edit_start": edit_start,
        "edit_end": edit_end,
        "underlay_start": (edit_start[0] + 48, edit_start[1] + 64),
        "underlay_end": (edit_start[0] + 146, edit_start[1] + 135),
        "future_start": (round(left + 80), edit_start[1] + 30),
        "future_end": (round(left + 240), edit_start[1] + 95),
        "cancel_start": (round(right - 180), edit_start[1] + 145),
        "cancel_end": (round(right - 95), edit_start[1] + 215),
        "reload_start": (round(width * 0.25), round(height * 0.40)),
        "reload_end": (round(width * 0.52), round(height * 0.55)),
    }


def geometry(environment, source, visible_size=None):
    """【滤镜测试】【输出查询】从 environment 查询唯一输出，用 visible_size 核对实际逻辑截图尺寸；返回 source 布局，不访问真实桌面"""
    outputs = json.loads(subprocess.check_output(["niri", "msg", "-j", "outputs"], env=environment))
    if len(outputs) != 1:
        raise RuntimeError("the filter regression requires one unambiguous private output")
    logical = next(iter(outputs.values()))["logical"]
    if visible_size is not None:
        if any(abs(actual - logical[key]) > 1 for actual, key in zip(visible_size, ("width", "height"))):
            raise AssertionError("the logical screenshot size disagrees with the private compositor output")
        # 1. 【滤镜测试】【输出查询】分数缩放的截图和应用缓冲向上取整，合成器 JSON 的逻辑尺寸可能向下取整
        logical = dict(logical, width=visible_size[0], height=visible_size[1])
    return layout_for(logical, source)


def read_commits(path):
    """【滤镜测试】【提交读取】从 path 解析已提交滤镜；返回有序记录，格式错误或非有限数值立即报告错误"""
    if not path.exists():
        return []
    result = []
    for match in COMMIT.finditer(path.read_text(errors="replace")):
        identifier, mode, strength, native, logical = match.groups()
        annotation = {
            "id": int(identifier),
            "mode": mode,
            "strength": float(strength),
            "native": tuple(map(int, native.split(","))),
            "logical": tuple(map(float, logical.split(","))),
            "opacity": 255,
        }
        if len(annotation["native"]) != 4 or len(annotation["logical"]) != 4:
            raise ValueError(f"malformed native filter coordinates: {annotation}")
        if not all(math.isfinite(value) for value in (*annotation["logical"], annotation["strength"])):
            raise ValueError(f"non-finite committed filter coordinates: {annotation}")
        result.append(annotation)
    return result


def validate_commit(annotation, start, end, mode, strength, layout):
    """【滤镜测试】【输入交叉核对】用 start、end、mode、strength 和 layout 独立验证 annotation；返回记录，不能仅相信日志区域"""
    if annotation["mode"] != mode or annotation["strength"] != strength:
        raise AssertionError(f"requested filter properties differ from the committed input: {annotation}")
    requested = (*map(round, start), *map(round, end))
    if any(abs(actual - target) > 1.25 for actual, target in zip(annotation["logical"], requested)):
        raise AssertionError(f"the filter did not follow the requested pointer coordinates: {requested} -> {annotation}")
    left, top, scale = layout["viewport"]
    width, height = layout["source_size"]
    sx, sy, ex, ey = annotation["logical"]
    expected = (
        max(0, math.floor((min(sx, ex) - left) / scale)),
        max(0, math.floor((min(sy, ey) - top) / scale)),
        min(width, math.ceil((max(sx, ex) - left) / scale)),
        min(height, math.ceil((max(sy, ey) - top) / scale)),
    )
    native = annotation["native"]
    if not (0 <= native[0] < native[2] <= width and 0 <= native[1] < native[3] <= height):
        raise AssertionError(f"the committed native rectangle exceeds the source: {annotation}")
    if any(abs(actual - target) > 1 for actual, target in zip(native, expected)):
        raise AssertionError(f"the committed native rectangle differs from independent viewport mapping: {expected} -> {native}")
    return annotation


def wait_commit(path, index, start, end, mode, strength, layout):
    """【滤镜测试】【提交等待】等待 path 中 index 对应真实输入；以其余参数核对模式与几何，返回记录，额外提交也报告错误"""
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        commits = read_commits(path)
        if len(commits) > index:
            if len(commits) != index + 1:
                raise AssertionError(f"unexpected extra filter commits: {len(commits)} instead of {index + 1}")
            return validate_commit(commits[index], start, end, mode, strength, layout)
        time.sleep(0.03)
    raise RuntimeError(f"the requested {mode}/{strength} filter did not commit; see {path}")
