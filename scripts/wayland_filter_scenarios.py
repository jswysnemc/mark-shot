"""【滤镜测试】【属性场景】验证完整算法矩阵、模式弹层、强度范围、透明度事务及后续默认值"""

import time

from wayland_editor_locator import filter_controls
from wayland_filter_checks import alive, capture, difference, history, idle, quiet
from wayland_filter_fixture import BORDER, read_commits, wait_commit
from wayland_filter_input import choose, click, draw, key_tool, opacity, select, strength
import wayland_test_session as session


def create(process, artifacts, layout, log, start, end, mode, native_strength, name, alpha=255, modifier=None):
    """【滤镜测试】【区域提交】在 process 中绘制 start、end 区域并以 log 核对 mode、native_strength；返回含 alpha 的记录，其余参数保存证据"""
    index = len(read_commits(log))
    key_tool("redact")
    before = quiet(process, artifacts / f"{name}-empty.png", layout)
    draw(start, end, modifier)
    annotation = wait_commit(log, index, start, end, mode, native_strength, layout)
    annotation["opacity"] = alpha
    after = quiet(process, artifacts / f"{name}-drawn.png", layout)
    region = (*map(lambda value: round(value) - 2, start), *map(lambda value: round(value) + 2, end))
    if not difference(before, after, region):
        raise AssertionError(f"the requested {mode}/{native_strength} filter has no visible native effect")
    return annotation


def matrix(process, artifacts, layout, log):
    """【滤镜测试】【五模式矩阵】在 process 中创建 layout 全部十五区域；返回逐项交叉核对记录，artifacts、log 保存真实输入证据"""
    annotations = []
    original_count = len(read_commits(log))
    for index, cell in enumerate(layout["matrix"]):
        mode, value = cell["mode"], cell["strength"]
        name = f"matrix-{mode}-{value}"
        key_tool("redact")
        choose(process, artifacts, layout, mode, name)
        strength(process, artifacts, layout, value, name)
        annotation = create(process, artifacts, layout, log, cell["start"], cell["end"], mode, value, name)
        annotations.append(annotation)
        if len(read_commits(log)) != original_count + index + 1:
            raise AssertionError("filter property input unexpectedly committed an extra native region")
    return annotations


def panel_lifecycle(process, artifacts, layout):
    """【滤镜测试】【弹层生命周期】验证 process 模式弹层重点击、滚轮隔离、Escape 与确认；返回空闲记录，layout、artifacts 保存证据"""
    key_tool("redact")
    before = quiet(process, artifacts / "mode-toggle-before.png", layout)
    controls = filter_controls(before, BORDER)
    region = (48, int(layout["viewport"][1]), layout["size"][0] - 48, layout["matrix"][0]["start"][1] + 170)
    click(controls["mode"])
    time.sleep(0.3)
    opened = capture(artifacts / "mode-toggle-open.png")
    if not difference(before, opened, region):
        raise AssertionError("the filter mode popover did not visibly open")
    resources = idle(process, artifacts, "mode-popover")
    session.control("scroll", "--to", *controls["strength"], "--dy", -1, "--notches", 3)
    click(controls["mode"])
    closed = quiet(process, artifacts / "mode-toggle-closed.png", layout)
    if difference(before, closed, region) or difference(before, closed, layout["content"]):
        raise AssertionError("repeated mode click or isolated scrolling changed the filter settings or pixels")
    click(controls["mode"])
    time.sleep(0.2)
    session.key("Escape")
    alive(process)
    escaped = quiet(process, artifacts / "mode-escape-closed.png", layout)
    if difference(before, escaped, region):
        raise AssertionError("Escape did not close only the filter mode popover")
    click(controls["mode"])
    time.sleep(0.2)
    session.key("End")
    session.key("Return")
    alive(process)
    confirmed = quiet(process, artifacts / "mode-confirmed.png", layout)
    if difference(before, confirmed, region):
        raise AssertionError("confirming the current filter mode left a popover or changed the underlying image")
    return resources


def large_region(process, artifacts, layout, log):
    """【滤镜测试】【冻结原图覆盖】先提交反色底层再创建跨块像素化区域；返回两条记录及底层界面，process、artifacts、layout、log 提供真实会话"""
    key_tool("redact")
    choose(process, artifacts, layout, "invert", "underlay-mode")
    strength(process, artifacts, layout, 4, "underlay-strength")
    underlay = create(process, artifacts, layout, log, layout["underlay_start"], layout["underlay_end"], "invert", 4, "underlay")
    choose(process, artifacts, layout, "pixelate", "large-mode")
    strength(process, artifacts, layout, 14, "large-strength")
    base = quiet(process, artifacts / "large-base.png", layout)
    annotation = create(process, artifacts, layout, log, layout["edit_start"], layout["edit_end"], "pixelate", 14, "large", modifier="Shift")
    sx, sy, ex, ey = annotation["logical"]
    if abs((ex - sx) / (ey - sy) - 1) < 0.08:
        raise AssertionError("Shift incorrectly forced the initial filter rectangle into a square")
    left, top, right, bottom = annotation["native"]
    if min(right - left, bottom - top) <= 512:
        raise AssertionError("the actual large filter failed to cross both native core-tile boundaries")
    return underlay, annotation, base


def property_history(process, artifacts, layout, annotation):
    """【滤镜测试】【属性历史】修改 process 中 annotation 的原图强度、模式及透明度并核对单次历史；返回最终界面，layout、artifacts 保存失败证据"""
    key_tool("redact")
    before = quiet(process, artifacts / "properties-original.png", layout)
    for raw, expected, name in ((42, 42, "strength-42"), (-9, 4, "strength-clamp-lower"), (999, 48, "strength-clamp-upper"), (42, 42, "strength-final")):
        select(annotation)
        strength(process, artifacts, layout, raw, name)
        key_tool("redact")
        changed = quiet(process, artifacts / f"{name}-changed.png", layout)
        history(process, artifacts, layout, name, before, changed)
        annotation["strength"] = expected
        before = changed

    select(annotation)
    choose(process, artifacts, layout, "blur", "existing-blur")
    key_tool("redact")
    changed = quiet(process, artifacts / "existing-blur-changed.png", layout)
    history(process, artifacts, layout, "existing-mode", before, changed)
    annotation["mode"] = "blur"
    select(annotation)
    choose(process, artifacts, layout, "blur", "existing-blur-repeat")
    key_tool("redact")
    repeated = quiet(process, artifacts / "existing-blur-repeated.png", layout)
    if difference(changed, repeated, layout["content"]):
        raise AssertionError("selecting the current filter mode changed its pixels")
    history(process, artifacts, layout, "mode-after-repeat", before, changed)

    # 1. 【滤镜测试】【属性历史】透明度使用实际横向 alpha 轨道，撤销与重做都只能合成一次覆盖层
    select(annotation)
    opacity(process, artifacts, layout, 128, "opacity-128")
    key_tool("redact")
    translucent = quiet(process, artifacts / "opacity-128-changed.png", layout)
    history(process, artifacts, layout, "opacity-128", changed, translucent)
    annotation["opacity"] = 128
    return translucent


def future_default(process, artifacts, layout, log):
    """【滤镜测试】【后续默认】在 process 中创建像素化四十二强度区域，保持原版不透明默认色；layout、artifacts、log 核对模式与 alpha 隔离，返回记录"""
    return create(process, artifacts, layout, log, layout["future_start"], layout["future_end"], "pixelate", 42, "future-default", alpha=255)
