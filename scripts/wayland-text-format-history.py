#!/usr/bin/env python3
"""【文字格式回归】【实际输入】只操作私有 Wayland，核验字号、字形和字体的局部恢复与提交来源"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time
import tomllib

from PIL import Image, ImageChops
import wayland_text_format_session as session
from wayland_text_property_locator import font_button, font_popup
from wayland_text_pointer import pointer_context, click as pointer_click
from wayland_text_glyph_capture import capture

SOURCE = (30, 34, 38)


def click(point, button="left"):
    """【文字格式回归】【指针输入】向私有 point 发送完整 button 点击，等待应用处理，无返回值"""
    pointer_click(tuple(map(round, point)), button)
    time.sleep(0.04)


def glyphs(image, point):
    """【文字格式回归】【正文像素】从 image 的 point 周围提取红色字形，返回 RGB 缓冲；排除纯白光标与工具栏"""
    region = image.crop((point[0] - 10, point[1] - 10, min(image.width, point[0] + 380), min(image.height, point[1] + 170)))
    result = Image.new("RGB", region.size)
    result.putdata([pixel if pixel[0] > 100 and pixel[0] > pixel[1] * 1.7 and pixel[0] > pixel[2] * 1.7 else (0, 0, 0) for pixel in region.getdata()])
    return result


def same(actual, expected, name):
    """【文字格式回归】【像素恢复】比较 actual 与 expected 完整字形缓冲，name 标识动作；存在任何差异就失败，无返回值"""
    difference = ImageChops.difference(actual, expected).getbbox()
    if difference:
        raise AssertionError(f"{name} did not restore exact document glyphs: {difference}")


def open_font(process, root, name):
    """【文字格式回归】【字体打开】从 process 实际颜色按钮定位入口，root 保存 name；返回当前字体面板几何"""
    click(font_button(capture(process, root, f"{name}-before")))
    return font_popup(capture(process, root, f"{name}-open"))


def close_font(process, root, name):
    """【文字格式回归】【字体关闭】再次点击 process 字体按钮关闭面板，root 保存 name；不发送正文提交按键，无返回值"""
    click(font_button(capture(process, root, f"{name}-closing")))


def menu_history(process, root, point, redo):
    """【文字格式回归】【菜单历史】在 process 的 point 打开原生菜单并按 redo 选择历史方向，root 保存证据，无返回值"""
    click((point[0] + 10, point[1] + 12), "right")
    capture(process, root, "menu-redo" if redo else "menu-undo")
    session.key("Home")
    if redo:
        session.key("Down")
    session.key("Return")


def focused_resources(before, after, protocol):
    """【文字格式回归】【聚焦资源】检查 before、after 与 protocol 的一秒聚焦状态；只允许一至三次窄光标更新及最多 2% 单核时钟，返回帧数"""
    frames = [tuple(map(int, match)) for match in re.findall(r"wl_surface@\d+\.damage_buffer\((-?\d+), (-?\d+), (\d+), (\d+)\)", protocol)]
    budget = max(1, round(os.sysconf("SC_CLK_TCK") * 0.02))
    if after["threads"] != 1 or not 0 <= after["cpu_ticks"] - before["cpu_ticks"] <= budget:
        raise AssertionError(f"focused caret exceeded its resource bound: {before}, {after}, {budget}")
    if not 1 <= len(frames) <= 3 or any(not 1 <= width < 32 or not 10 <= height < 90 for _x, _y, width, height in frames):
        raise AssertionError(f"focused caret did not remain a low-frequency narrow redraw: {frames}")
    return len(frames)


def run(binary, root, overlay="layer", menu_hold=0.04, pointer_backend="virtual"):
    """【文字格式回归】【完整场景】以 binary、overlay、menu_hold、pointer_backend 验证历史，root 保存证据；返回像素、默认值和资源结果"""
    root.mkdir(parents=True, exist_ok=False)
    (root / "binary.json").write_text(json.dumps({"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                                                "bytes": binary.stat().st_size}, indent=2) + "\n")
    environment = session.environment()
    with session.input_context(), pointer_context(menu_hold, pointer_backend), tempfile.TemporaryDirectory(prefix="mark-shot-format-test-") as temporary:
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        private = Path(temporary)
        config, state = private / "config/mark-shot", private / "state/mark-shot"
        config.mkdir(parents=True)
        state.mkdir(parents=True)
        (config / "config.toml").write_text(f'[general]\nto = "s"\noverlay = "{overlay}"\n[notifications]\nenabled = false\n[history]\nenabled = false\n')
        (state / "tools.toml").write_text('magnifier = false\nfont_size = 20.0\nfont_size_unit = "pt"\ntext_family = "Inter"\nbold = false\nitalic = false\n')
        environment.update(XDG_CONFIG_HOME=str(private / "config"), XDG_DATA_HOME=str(private / "data"), XDG_STATE_HOME=str(private / "state"), WAYLAND_DEBUG="client")
        source = private / "source.png"
        Image.new("RGB", (1600, 900), SOURCE).save(source)
        with (root / "process.log").open("w") as log:
            process = subprocess.Popen([str(binary), "--fullscreen", "--fullscreen-default-tool", "text", "--default-color", "#FF4D4D", "--to", "s", "--output", str(private / "export"), str(source)], env=environment, stderr=log)
            try:
                initial = capture(process, root, "initial")
                session.wait_single_thread(process.pid)
                point = round(initial.width * 0.62), round(initial.height * 0.52)
                click(point)
                # 1. 【文字格式回归】【空闲绑定】先让新建预览稳定，再增加无界面键盘，避免启动重绘掩盖未刷新设备请求
                time.sleep(0.3)
                for character in "alpha":
                    session.key(character)
                baseline = glyphs(capture(process, root, "twenty-points"), point)
                if not baseline.getbbox():
                    raise AssertionError("initial text glyphs are missing")
                # 1. 【文字格式回归】【字号恢复】字段导航不追加同值历史，撤销恢复二十点但默认值保留三十点
                open_font(process, root, "size")
                session.key("Tab")
                session.paste_text("30")
                session.key("Left")
                close_font(process, root, "size")
                large = glyphs(capture(process, root, "thirty-points"), point)
                if not ImageChops.difference(large, baseline).getbbox():
                    raise AssertionError("size change did not alter actual glyphs")
                session.key("ctrl+z")
                same(glyphs(capture(process, root, "size-undo"), point), baseline, "size undo")
                session.key("ctrl+shift+z")
                same(glyphs(capture(process, root, "size-redo"), point), large, "size redo")
                session.key("ctrl+z")
                session.key("x")
                session.key("ctrl+z")
                if glyphs(capture(process, root, "resumed-typing-undo"), point).getbbox():
                    raise AssertionError("typing after format undo did not resume the original input group")
                session.key("ctrl+shift+z")
                current = glyphs(capture(process, root, "resumed-typing-redo"), point)
                actions = []
                # 2. 【文字格式回归】【字体字形】字体面板每次实际动作独立可撤销，恢复比较完整像素而不是只看控件数值
                for action in ("bold", "italic", "family"):
                    left, top, _right, _bottom = open_font(process, root, action)
                    if action == "family":
                        session.paste_text("Liberation Sans")
                        capture(process, root, "family-search")
                        click((left + 55, top + 54))
                    else:
                        click((left + (311 if action == "bold" else 341), top + 339))
                        close_font(process, root, action)
                    changed = glyphs(capture(process, root, f"{action}-changed"), point)
                    if not ImageChops.difference(changed, current).getbbox():
                        raise AssertionError(f"{action} change did not alter actual glyphs")
                    session.key("ctrl+z")
                    same(glyphs(capture(process, root, f"{action}-undo"), point), current, f"{action} undo")
                    session.key("ctrl+shift+z")
                    same(glyphs(capture(process, root, f"{action}-redo"), point), changed, f"{action} redo")
                    actions.append({"action": action, "undo_exact": True, "redo_exact": True, "glyph_bounds": changed.getbbox()})
                    if action == "family":
                        menu_history(process, root, point, False)
                        same(glyphs(capture(process, root, "family-menu-undo"), point), current, "menu font undo")
                        menu_history(process, root, point, True)
                        same(glyphs(capture(process, root, "family-menu-redo"), point), changed, "menu font redo")
                    current = changed
                session.key("ctrl+a")
                session.key("ctrl+c")
                session.wait_clipboard(environment, "alphax")
                # 3. 【文字格式回归】【光标分离】斜体首末字符可能与光标相交，移到尾部空格后再比较提交像素；提交按原版裁掉尾部空白，不放宽断言
                session.key("End")
                session.key("space")
                current = glyphs(capture(process, root, "precommit-with-separated-caret"), point)
                before = session.wait_single_thread(process.pid)
                protocol_start = len((root / "process.log").read_text())
                time.sleep(1)
                after = session.resources(process.pid)
                focused_frames = focused_resources(before, after, (root / "process.log").read_text()[protocol_start:])
                # 4. 【文字格式回归】【成稿来源】格式与正文操作不泄漏全局历史，提交后只有一个文字对象并使用最后实际属性
                session.key("ctrl+Return")
                committed = glyphs(capture(process, root, "committed"), point)
                same(committed, current, "font commit")
                session.key("ctrl+z")
                if glyphs(capture(process, root, "global-undo"), point).getbbox():
                    raise AssertionError("local format edits leaked into global annotation history")
                session.key("ctrl+shift+z")
                same(glyphs(capture(process, root, "global-redo"), point), committed, "global redo")
                # 5. 【文字格式回归】【透明度分流】新草稿的透明度翻页不新增字体历史，正文一次撤销仍清空，空草稿不形成第二个标注
                session.key("ctrl+Return")
                same(glyphs(capture(process, root, "restored-text-committed"), point), committed, "unchanged restored text commit")
                opacity_point = round(initial.width * 0.32), round(initial.height * 0.75)
                click(opacity_point)
                for character in "alpha":
                    session.key(character)
                opacity_typed = capture(process, root, "opacity-typed")
                if not glyphs(opacity_typed, opacity_point).getbbox():
                    raise AssertionError("opacity draft glyphs are missing")
                button = font_button(opacity_typed)
                click((button[0] - 217, button[1] - 1))
                opacity = glyphs(capture(process, root, "opacity-adjusted"), opacity_point)
                session.key("ctrl+z")
                if glyphs(capture(process, root, "opacity-local-undo"), opacity_point).getbbox():
                    raise AssertionError("opacity slider inserted an extra document format undo")
                session.key("ctrl+shift+z")
                same(glyphs(capture(process, root, "opacity-local-redo"), opacity_point), opacity, "opacity text redo")
                session.key("ctrl+z")
                session.key("ctrl+Return")
                same(glyphs(capture(process, root, "opacity-empty-committed"), point), committed, "empty opacity draft commit")
                # 6. 【文字格式回归】【字段提交】独立新草稿的字号 Return 提交文字但保留面板；已有编辑入口隐藏属性栏，不伪造可见字段
                click(opacity_point)
                for character in "field":
                    session.key(character)
                session.key("space")
                field_before = glyphs(capture(process, root, "field-thirty-points"), opacity_point)
                # 7. 【文字格式回归】【透明按钮定位】90% 按钮不能匹配不透明红色定位器，复用本次真实按钮位置并严格确认弹层
                click(button)
                font_popup(capture(process, root, "field-submit-open"))
                session.key("Tab")
                # 8. 【文字格式回归】【渲染量化】点字号先乘原图比例再取整，选择四十点确保与三十点跨越实际字形栅格
                session.paste_text("40")
                session.key("Return")
                field_submitted = capture(process, root, "field-return-submitted")
                font_popup(field_submitted)
                submitted = glyphs(field_submitted, opacity_point)
                if not submitted.getbbox():
                    raise AssertionError("field Return did not retain its actual text")
                if not ImageChops.difference(submitted, field_before).getbbox():
                    raise AssertionError("field point-size change did not alter the document glyphs")
                click(button)
                capture(process, root, "field-submit-closed")
                session.key("ctrl+z")
                field_undo = capture(process, root, "field-global-undo")
                if glyphs(field_undo, opacity_point).getbbox():
                    raise AssertionError("field Return did not commit into global annotation history")
                same(glyphs(field_undo, point), committed, "field submission global undo")
                session.key("ctrl+z")
                if glyphs(capture(process, root, "field-initial-global-undo"), point).getbbox():
                    raise AssertionError("field submission created duplicate global history entries")
                session.key("ctrl+shift+z")
                same(glyphs(capture(process, root, "field-initial-global-redo"), point), committed, "field initial document redo")
                session.key("ctrl+shift+z")
                same(glyphs(capture(process, root, "field-global-redo"), opacity_point), submitted, "field submission global redo")
                session.key("ctrl+s")
                process.wait(timeout=6)
                files = list((private / "export").glob("*.png"))
                if process.returncode or len(files) != 1:
                    raise AssertionError(f"format export failed: {process.returncode}, {files}")
                with Image.open(files[0]) as image:
                    output = image.convert("RGB")
                if output.size != (1600, 900):
                    raise AssertionError(f"format export changed source resolution: {output.size}")
                saved = tomllib.loads((state / "tools.toml").read_text())
                if saved["font_size"] != 40 or saved["text_family"] != "Liberation Sans" or not saved["bold"] or not saved["italic"]:
                    raise AssertionError(f"local undo changed tool defaults: {saved}")
                if len(saved["color"]) != 9 or int(saved["color"][-2:], 16) != 230:
                    raise AssertionError(f"opacity route did not preserve its actual color value: {saved['color']}")
                if (private / "data/mark-shot/models").exists():
                    raise AssertionError("text format editing initialized OCR models")
                shutil.copyfile(files[0], root / "export.png")
                shutil.copyfile(state / "tools.toml", root / "tools.toml")
                protocol = (root / "process.log").read_text()
                if "xdg_popup" not in protocol or ".grab(" not in protocol:
                    raise AssertionError("format history menu did not use a native popup grab")
                return {"overlay": overlay, "right_press_seconds": menu_hold, "pointer_backend": pointer_backend,
                        "size_undo_redo_exact": True, "field_navigation_has_no_duplicate_history": True,
                        "opacity_has_no_format_undo": True, "opacity_value_preserved": True,
                        "typing_group_resumed": True, "format_actions": actions, "native_menu_undo_redo_exact": True,
                        "defaults_preserved": True, "initial_document_has_one_global_commit": True, "copy_text": "alphax",
                        "field_return_commits_without_export": True, "field_return_preserves_panel": True,
                        "field_submission_global_undo_redo_exact": True, "field_submission_has_one_global_entry": True, "field_points": 40,
                        "export_size": list(output.size), "focused_before": before, "focused_after": after, "focused_caret_frames": focused_frames}
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=3)


def main():
    """【文字格式回归】【入口】解析 binary 与 artifacts，保存同一构建指纹和实际场景结果，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--overlay", choices=("layer", "window"), default="layer")
    parser.add_argument("--menu-hold", type=float, default=0.04)
    parser.add_argument("--pointer-backend", choices=("virtual", "x11"), default="virtual")
    options = parser.parse_args()
    binary = options.binary.resolve()
    result = {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "scenario": run(binary, options.artifacts.resolve(), options.overlay, options.menu_hold, options.pointer_backend)}
    (options.artifacts / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
