"""【滤镜测试】【几何手势】核对八个缩放点、移动、独立旋转参考、草稿取消与删除历史"""

import math
import subprocess
import time

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

from wayland_filter_checks import alive, capture, difference, history, quiet
from wayland_filter_input import click, draw, key_tool, select
import wayland_test_session as session


def rotated(point, center, degrees):
    """【滤镜测试】【独立旋转】把 point 绕 center 旋转 degrees 度，返回参考逻辑坐标"""
    angle = math.radians(degrees)
    dx, dy = point[0] - center[0], point[1] - center[1]
    return center[0] + dx * math.cos(angle) - dy * math.sin(angle), center[1] + dx * math.sin(angle) + dy * math.cos(angle)


def check_rotation(base, changed, annotation, layout, degrees):
    """【滤镜测试】【旋转像素】以 annotation、degrees 独立构造遮罩，并比较 base、changed；layout 限制内容区，无返回值"""
    sx, sy, ex, ey = annotation["logical"]
    center = ((sx + ex) / 2, (sy + ey) / 2)
    polygon = [rotated(point, center, degrees) for point in ((sx, sy), (ex, sy), (ex, ey), (sx, ey))]
    mask = Image.new("L", base.size)
    ImageDraw.Draw(mask).polygon(polygon, fill=255)
    region = layout["content"]
    outside = np.asarray(mask.filter(ImageFilter.MaxFilter(9)).crop(region)) == 0
    inside = np.asarray(mask.filter(ImageFilter.MinFilter(11)).crop(region)) != 0
    delta = np.max(np.abs(np.asarray(base.crop(region), dtype=np.int16) - np.asarray(changed.crop(region), dtype=np.int16)), axis=2)
    if np.any(delta[outside] != 0):
        raise AssertionError(f"the filter painted outside the independent {degrees}-degree rotated polygon")
    if np.count_nonzero((delta > 3) & inside) < np.count_nonzero(inside) * 0.8:
        raise AssertionError(f"the filter did not cover the independent {degrees}-degree rotated polygon")


def effect_bounds(base, image, layout):
    """【滤镜测试】【效果边界】比较 base、image，返回 layout 内容区内滤镜的可见边界；无差异时报告错误"""
    region = layout["content"]
    bbox = difference(base, image, region)
    if bbox is None:
        raise AssertionError("the edited native filter has no visible bounds")
    return bbox[0] + region[0], bbox[1] + region[1], bbox[2] + region[0], bbox[3] + region[1]


def check_drag(process, artifacts, layout, annotation, name, start, end, modifier=None):
    """【滤镜测试】【已有拖动】在 process 中拖动 annotation 的 start 至 end，modifier 约束手势；返回改变界面，layout、artifacts、name 核对单次历史后恢复原区域"""
    key_tool("redact")
    before = quiet(process, artifacts / f"{name}-before.png", layout)
    select(annotation)
    draw(start, end, modifier)
    key_tool("redact")
    changed = quiet(process, artifacts / f"{name}-changed.png", layout)
    history(process, artifacts, layout, name, before, changed)
    session.key("ctrl+z")
    restored = quiet(process, artifacts / f"{name}-restored.png", layout)
    if difference(before, restored, layout["content"]):
        raise AssertionError(f"the {name} probe did not restore the original region")
    return changed


def controls(process, artifacts, layout, annotation, base):
    """【滤镜测试】【完整控制点】验证 process 中 annotation 的移动、八点缩放、Ctrl 等比和两种旋转；base 为无大区域界面，layout、artifacts 保存证据"""
    sx, sy, ex, ey = annotation["logical"]
    center = ((sx + ex) / 2, (sy + ey) / 2)
    margin = 6 * layout["viewport"][2]
    handles = {
        "top-left": ((sx - margin, sy - margin), (14, 12)),
        "top": ((center[0], sy - margin), (0, 12)),
        "top-right": ((ex + margin, sy - margin), (-14, 12)),
        "left": ((sx - margin, center[1]), (14, 0)),
        "right": ((ex + margin, center[1]), (-14, 0)),
        "bottom-left": ((sx - margin, ey + margin), (14, -12)),
        "bottom": ((center[0], ey + margin), (0, -12)),
        "bottom-right": ((ex + margin, ey + margin), (-14, -12)),
    }
    check_drag(process, artifacts, layout, annotation, "move", center, (center[0] + 18, center[1] - 14))
    for name, (start, delta) in handles.items():
        check_drag(process, artifacts, layout, annotation, f"resize-{name}", start, (start[0] + delta[0], start[1] + delta[1]))

    # 1. 【滤镜测试】【完整控制点】同一侧把手的自由和等比缩放必须产生不同高宽行为，不能只验证发生变化
    key_tool("redact")
    original = quiet(process, artifacts / "proportion-original.png", layout)
    start = handles["right"][0]
    end = (start[0] - 35, start[1])
    free = check_drag(process, artifacts, layout, annotation, "free-resize", start, end)
    proportional = check_drag(process, artifacts, layout, annotation, "proportional-resize", start, end, "Ctrl")
    old = effect_bounds(base, original, layout)
    first = effect_bounds(base, free, layout)
    second = effect_bounds(base, proportional, layout)
    ratio = (old[2] - old[0]) / (old[3] - old[1])
    free_ratio = (first[2] - first[0]) / (first[3] - first[1])
    locked_ratio = (second[2] - second[0]) / (second[3] - second[1])
    if abs(locked_ratio / ratio - 1) > 0.025 or abs(free_ratio / ratio - 1) < 0.06:
        raise AssertionError(f"Ctrl did not preserve native proportions independently of free resize: {ratio}, {free_ratio}, {locked_ratio}")

    # 2. 【滤镜测试】【完整控制点】自由旋转使用三十二度参考；同一目标在 Shift 下必须完全吸附到四十五度
    rotation = (center[0], sy - 28 * layout["viewport"][2])
    target = rotated(rotation, center, 32)
    changed = check_drag(process, artifacts, layout, annotation, "free-rotation", rotation, target)
    check_rotation(base, changed, annotation, layout, 32)
    changed = check_drag(process, artifacts, layout, annotation, "snapped-rotation", rotation, target, "Shift")
    check_rotation(base, changed, annotation, layout, 45)


def cancel(process, artifacts, layout, annotation, existing=False):
    """【滤镜测试】【未提交取消】保持 process 草稿或 existing 已有区域拖动并按 Escape；annotation、layout 定位输入，artifacts 核对原像素恢复"""
    key_tool("redact")
    name = "existing-cancel" if existing else "draft-cancel"
    before = quiet(process, artifacts / f"{name}-before.png", layout)
    if existing:
        select(annotation)
        sx, sy, ex, ey = annotation["logical"]
        start = ((sx + ex) / 2, (sy + ey) / 2)
        end = (start[0] + 30, start[1] - 24)
    else:
        start, end = layout["cancel_start"], layout["cancel_end"]
    with subprocess.Popen(["niri-use", "--child", "click", "--logical", *map(str, map(round, start)), "--hold", "3"], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE) as gesture:
        time.sleep(0.2)
        session.control("move", "--logical", *map(round, end))
        active = capture(artifacts / f"{name}-active.png")
        if gesture.poll() is not None or not difference(before, active, layout["content"]):
            raise AssertionError(f"the {name} gesture did not stay observable with its pointer button held")
        session.key("Escape")
        alive(process)
        gesture.wait(timeout=5)
        if gesture.returncode:
            raise RuntimeError(gesture.stderr.read().decode())
    key_tool("redact")
    restored = quiet(process, artifacts / f"{name}-restored.png", layout)
    if difference(before, restored, layout["content"]):
        raise AssertionError(f"the {name} gesture altered committed pixels")


def deletion(process, artifacts, layout, annotation, base):
    """【滤镜测试】【删除历史】删除 process 中 annotation 并与 base 比较，再撤销、重做及恢复；layout、artifacts 保存证据，无返回值"""
    key_tool("redact")
    before = quiet(process, artifacts / "delete-before.png", layout)
    select(annotation)
    session.key("Delete")
    key_tool("redact")
    deleted = quiet(process, artifacts / "delete-removed.png", layout)
    if difference(base, deleted, layout["content"]):
        raise AssertionError("deleting the large filter changed underlying native annotations or left stale cache pixels")
    history(process, artifacts, layout, "delete", before, deleted)
    session.key("ctrl+z")
    restored = quiet(process, artifacts / "delete-restored.png", layout)
    if difference(before, restored, layout["content"]):
        raise AssertionError("the deleted filter did not restore its native appearance")


def empty_creation(process, artifacts, layout):
    """【滤镜测试】【最小创建】在 process 图像内单击零面积区域；layout、artifacts 核对没有正文或空历史，无返回值"""
    key_tool("redact")
    before = quiet(process, artifacts / "zero-area-before.png", layout)
    click(layout["cancel_start"])
    after = quiet(process, artifacts / "zero-area-after.png", layout)
    if difference(before, after, layout["content"]):
        raise AssertionError("zero-area filter input committed native pixels")
