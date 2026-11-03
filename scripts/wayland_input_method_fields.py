"""【字段输入回归】【真实路由】验证原生字体搜索、完整字号和颜色确认，不操作用户会话"""

import time

from PIL import ImageChops
import wayland_text_format_session as session
from wayland_color_locator import color_popover_rectangle
from wayland_text_pointer import click
from wayland_text_property_locator import font_popup


def field_type(state, purpose):
    """【字段输入回归】【输入用途】按 purpose 校验原生 state 类型，单行字段不能携带多行属性；无返回值"""
    if state["purpose"] != purpose or state["hint"] & 512:
        raise AssertionError(f"native field type mismatch: {state}, expected {purpose}")


def visible(process, root, name, capture, locate):
    """【字段输入回归】【首帧等待】按 locate 校验 process 的真实界面，root 保留 name 画面；原生激活不等同于首帧呈现，返回画面与矩形"""
    deadline = time.monotonic() + 3
    while True:
        image = capture(process, root, name)
        try:
            return image, locate(image)
        except (AssertionError, ValueError):
            if time.monotonic() >= deadline:
                raise
            time.sleep(0.05)


def routing(process, method, root, button, region, capture):
    """【字段输入回归】【场景交付】按 process、method、button 执行字段验证，root 保存 capture 画面，region 隔离标注；返回真实结果"""
    # 1. 【字段输入回归】【搜索拥有者】新字段取消标注候选，搜索候选实际显示，但周边只含已确认文本
    method.batch(preedit="pending")
    time.sleep(0.15)
    before = capture(process, root, "font-before")
    click(button)
    field_type(method.state("", active=True), 0)
    _, (left, top, _right, _bottom) = visible(process, root, "font-open", capture, font_popup)
    method.batch(commit="Inter")
    method.state("Inter")
    time.sleep(0.15)
    prefix = capture(process, root, "search-confirmed-prefix")
    method.batch(preedit="ni")
    time.sleep(0.15)
    candidate = capture(process, root, "search-preedit")
    search = (left + 14, top + 8, left + 270, top + 34)
    bounds = ImageChops.difference(prefix.crop(search), candidate.crop(search)).getbbox()
    if bounds is None or bounds[2] - bounds[0] < 10:
        raise AssertionError(f"native search preedit not drawn: {bounds}")
    method.state("Inter")
    listing = (left + 6, top + 40, left + 354, top + 320)
    if ImageChops.difference(prefix.crop(listing), candidate.crop(listing)).getbbox():
        raise AssertionError("unconfirmed search candidate changed the family list")
    session.wait_single_thread(process.pid)
    idle_before = session.resources(process.pid)
    frames_before = (root / "process.log").read_text().count(".damage_buffer(")
    time.sleep(1.1)
    idle_after = session.resources(process.pid)
    frames_after = (root / "process.log").read_text().count(".damage_buffer(")
    if idle_before["cpu_ticks"] != idle_after["cpu_ticks"] or frames_before != frames_after or idle_after["threads"] != 1:
        raise AssertionError(f"stationary field kept work: {idle_before}, {idle_after}, {frames_before}, {frames_after}")
    method.batch(commit="你", preedit="hao")
    method.state("Inter你")
    method.batch(commit="好")
    confirmed = method.state("Inter你好")
    if confirmed["cursor"] != 11:
        raise AssertionError(f"search cursor is not UTF-8: {confirmed}")
    session.key("ctrl+z")
    method.state("")
    session.key("ctrl+shift+z")
    method.state("Inter你好")
    click(button)
    method.state("base", active=True)
    capture(process, root, "search-closed")
    session.key("ctrl+a")
    session.key("ctrl+c")
    session.wait_clipboard(session.environment(), "base")
    session.key("End")

    # 2. 【字段输入回归】【数字字段】数字用途、整批拒绝与完整后缀通过真实原生投递验证，字号恢复后再退出字段
    click(button)
    method.state("")
    visible(process, root, "size-before", capture, font_popup)
    session.key("Tab")
    size = method.state("20 pt")
    field_type(size, 2)
    if (size["cursor"], size["anchor"]) != (2, 0):
        raise AssertionError(f"initial size selection includes suffix: {size}")
    method.batch(commit="40你", preedit="ni")
    time.sleep(0.12)
    field_type(method.state("20 pt"), 2)
    capture(process, root, "size-invalid-preedit")
    method.batch(preedit="")
    method.state(" pt")
    session.key("ctrl+z")
    method.state("20 pt")
    method.batch(commit="40")
    method.state("40 pt")
    session.key("ctrl+z")
    method.state("20 pt")
    click(button)
    method.state("base")

    # 3. 【字段输入回归】【颜色确认】输入和撤销不提前改色，Return 确认后字段撤销仍保留业务颜色
    color_button = (button[0] - 76, button[1])
    click(color_button)
    color_open, (color_left, color_top, _right, _bottom) = visible(process, root, "hex-open", capture, color_popover_rectangle)
    click((color_left + 100, color_top + 175))
    field_type(method.state("#FF4D4D"), 0)
    session.key("ctrl+a")
    method.batch(commit="#22C55E")
    method.state("#22C55E")
    typed = capture(process, root, "hex-typed")
    if ImageChops.difference(color_open.crop(region), typed.crop(region)).getbbox():
        raise AssertionError("native hex confirmation changed annotation before Return")
    session.key("Return")
    applied = capture(process, root, "hex-confirmed")
    if not ImageChops.difference(color_open.crop(region), applied.crop(region)).getbbox():
        raise AssertionError("Return did not apply native hex color")
    session.key("ctrl+z")
    method.state("#FF4D4D")
    undone = capture(process, root, "hex-text-undo")
    if ImageChops.difference(applied.crop(region), undone.crop(region)).getbbox():
        raise AssertionError("hex field undo changed the confirmed annotation color")
    session.key("Return")
    click(color_button)
    method.state("base")
    capture(process, root, "hex-restored")
    return {"field_native_routing": True, "search_candidate_bounds": bounds,
            "search_candidate_preserves_filtered_pixels": True,
            "search_merged_undo": True, "size_digits_purpose": True,
            "invalid_batch_rejected": True, "hex_return_and_local_undo": True,
            "annotation_clipboard": "base", "before_size": list(before.size),
            "stationary_field_before": idle_before, "stationary_field_after": idle_after,
            "stationary_field_no_redraw": True}
