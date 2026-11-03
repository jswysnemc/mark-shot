#!/usr/bin/env python3
"""【文字菜单回归】【实际输入】只连接私有 Wayland，验证字符菜单、局部历史和菜单输入独占"""

import argparse
import hashlib
import json
import re
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import tomllib

from PIL import Image, ImageChops
import wayland_test_session as session
from wayland_history_actions import window

SOURCE = (30, 34, 38)
ROWS = json.loads((Path(__file__).resolve().parent.parent / "tests/fixtures/qt-text-menu.json").read_text())[0]["rows"]


def capture(process, root, name):
    """【文字菜单回归】【画面记录】将 process 当前画面保存到 root 的 name，返回 RGB 图；提前退出视为失败"""
    time.sleep(0.12)
    if process.poll() is not None:
        raise AssertionError(f"editor exited before menu transaction completed: {process.returncode}")
    path = root / f"{name}.png"
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def click(point, button="left"):
    """【文字菜单回归】【按钮输入】向私有 point 发送完整 button 点击，等待事件处理，无返回值"""
    session.control("click", "--logical", *map(round, point), "--button", button)
    time.sleep(0.04)


def menu_row(anchor, index):
    """【文字菜单回归】【原版行定位】按菜单 anchor 与原版 index 返回行中心，不通过修改断言适配错误布局"""
    x, y, width, height = ROWS[index]["geometry"]
    return anchor[0] + x + width / 2, anchor[1] + y + height / 2


def open_menu(process, root, point, name):
    """【文字菜单回归】【右键菜单】在 point 打开真实菜单并保存 name，返回菜单锚点；不改变字符选区"""
    anchor = point[0] + 5, point[1] + 10
    click(anchor, "right")
    image = capture(process, root, name)
    x, y = map(round, anchor)
    image.crop((x, y, x + 225, y + 229)).save(root / f"{name}-menu.png")
    return anchor


def copy_by_menu(process, root, point, environment, expected, name):
    """【文字菜单回归】【复制验证】全选 process 编辑器并经实际菜单复制，root 保存 name，核验 environment 中 expected，无返回值"""
    session.key("ctrl+a")
    anchor = open_menu(process, root, point, name)
    click(menu_row(anchor, 4))
    session.wait_clipboard(environment, expected)


def body_bounds(image, initial, point):
    """【文字菜单回归】【持久正文】比较 image 与 initial 在 point 周围的完整文字区域，返回差异边界；排除工具栏和原图留白"""
    bounds = (max(50, point[0] - 100), max(160, point[1] - 100),
              min(image.width - 50, point[0] + 300), min(image.height - 60, point[1] + 200))
    return ImageChops.difference(image.crop(bounds), initial.crop(bounds)).getbbox()


def scenario(binary, root, require_native=False, overlay="layer"):
    """【文字菜单回归】【完整场景】用 binary、overlay 在私有会话验证菜单，root 保存证据；require_native 要求真实抓取，返回实际结果"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    with tempfile.TemporaryDirectory(prefix="mark-shot-text-menu-") as directory:
        private = Path(directory)
        config, state = private / "config/mark-shot", private / "state/mark-shot"
        config.mkdir(parents=True)
        state.mkdir(parents=True)
        (config / "config.toml").write_text(f'[general]\nto = "s"\noverlay = "{overlay}"\n[notifications]\nenabled = false\n[history]\nenabled = false\n')
        (state / "tools.toml").write_text('magnifier = false\nfont_size = 20.0\nfont_size_unit = "pt"\ntext_family = "Inter"\nbold = false\nitalic = false\n')
        environment.update(XDG_CONFIG_HOME=str(private / "config"), XDG_DATA_HOME=str(private / "data"), XDG_STATE_HOME=str(private / "state"))
        process_environment = dict(environment, WAYLAND_DEBUG="client") if require_native else environment
        source = private / "source.png"
        Image.new("RGB", (1600, 900), SOURCE).save(source)
        with (root / "process.log").open("w") as log:
            process = subprocess.Popen([str(binary), "--fullscreen", "--fullscreen-default-tool", "text", "--default-color", "#FF4D4D", "--to", "s", "--output", str(private / "export"), str(source)], env=process_environment, stderr=log)
            try:
                initial = capture(process, root, "initial")
                session.wait_single_thread(process.pid)
                point = round(initial.width * 0.58), round(initial.height * 0.52)
                click(point)
                session.paste_text("alpha beta")
                # 1. 【文字菜单回归】【按钮序列隔离】菜单关闭后的首次左键只定位光标，不把旧左键序列延续为整词双击
                caret = point[0] + 15, point[1] + 13
                click(caret)
                click(caret, "right")
                session.key("Escape")
                click(caret)
                session.key("x")
                session.key("ctrl+a")
                session.key("ctrl+c")
                observed = session.read_clipboard(environment)
                deadline = time.monotonic() + 3
                while "x" not in observed and time.monotonic() < deadline:
                    time.sleep(0.03)
                    observed = session.read_clipboard(environment)
                if observed.count("x") != 1 or observed.replace("x", "", 1) != "alpha beta":
                    raise AssertionError(f"right menu retained an old left double-click sequence: {observed!r}")
                session.paste_text("alpha beta")
                session.key("ctrl+a")
                session.control("copy", "menu sentinel")
                # 2. 【文字菜单回归】【菜单独占】菜单中复制与保存快捷键不穿透，Esc 只关闭菜单，外部点击不提交
                anchor = open_menu(process, root, point, "selected")
                if require_native:
                    protocol = (root / "process.log").read_text()
                    if "get_popup(" not in protocol or not any("xdg_popup" in line and ".grab(" in line for line in protocol.splitlines()):
                        raise AssertionError("text menu has no native xdg-popup and real seat grab")
                    configure = re.search(r"xdg_popup@\d+\.configure, \((-?\d+), (-?\d+), (\d+), (\d+)\)", protocol)
                    requested = re.search(r"xdg_positioner@\d+\.set_anchor_rect\((-?\d+), (-?\d+), 1, 1\)", protocol)
                    expected = tuple(map(int, requested.groups())) + (225, 229) if requested else None
                    actual = tuple(map(int, configure.groups())) if configure else None
                    if actual is None or expected is None or actual != expected:
                        raise AssertionError(f"native popup changed original menu anchor: {actual} != {expected}")
                session.key("ctrl+c")
                session.wait_clipboard(environment, "menu sentinel")
                session.key("ctrl+s")
                capture(process, root, "shortcuts-consumed")
                session.key("Escape")
                session.key("ctrl+c")
                session.wait_clipboard(environment, "alpha beta")
                deactivation = None
                if require_native and overlay == "window":
                    # 3. 【文字菜单回归】【失活关闭】普通窗口失去激活时合成器结束菜单抓取，返回后保留编辑器选区与正文
                    editor_window = window("mark-shot", process.pid)
                    open_menu(process, root, point, "deactivation-menu")
                    auxiliary = None
                    try:
                        with (root / "auxiliary.log").open("w") as auxiliary_log:
                            auxiliary = subprocess.Popen([str(binary), "--history"], env=environment, stdout=subprocess.DEVNULL, stderr=auxiliary_log)
                        other = window("mark-shot-history", auxiliary.pid)
                        session.control("focus", other["id"])
                        capture(process, root, "deactivated")
                        protocol = (root / "process.log").read_text()
                        if not any("xdg_popup" in line and ".popup_done" in line for line in protocol.splitlines()):
                            raise AssertionError("compositor did not dismiss popup when parent lost activation")
                        session.control("focus", editor_window["id"])
                        time.sleep(0.2)
                        session.control("copy", "deactivation sentinel")
                        session.key("ctrl+c")
                        session.wait_clipboard(environment, "alpha beta")
                        capture(process, root, "reactivated")
                        deactivation = True
                    finally:
                        if auxiliary and auxiliary.poll() is None:
                            auxiliary.terminate()
                            auxiliary.wait(timeout=3)
                anchor = open_menu(process, root, point, "outside")
                click((point[0] - 100, point[1] + 100))
                session.key("ctrl+c")
                session.wait_clipboard(environment, "alpha beta")
                # 4. 【文字菜单回归】【剪切删除】剪切改变私有剪贴板，删除不改变剪贴板，两者可经菜单局部撤销
                anchor = open_menu(process, root, point, "cut")
                click(menu_row(anchor, 3))
                session.wait_clipboard(environment, "alpha beta")
                anchor = open_menu(process, root, point, "cut-empty")
                click(menu_row(anchor, 0))
                copy_by_menu(process, root, point, environment, "alpha beta", "cut-undo")
                session.control("copy", "delete sentinel")
                anchor = open_menu(process, root, point, "delete")
                click(menu_row(anchor, 6))
                session.wait_clipboard(environment, "delete sentinel")
                anchor = open_menu(process, root, point, "delete-empty")
                click(menu_row(anchor, 0))
                anchor = open_menu(process, root, point, "delete-undo")
                click(menu_row(anchor, 1))
                anchor = open_menu(process, root, point, "delete-redo")
                click(menu_row(anchor, 0))
                copy_by_menu(process, root, point, environment, "alpha beta", "delete-restored")
                # 5. 【文字菜单回归】【粘贴全选】粘贴替换选区并规范换行，禁用删除和菜单滚轮不修改正文或字号
                session.control("copy", "paste\r\nvalue")
                anchor = open_menu(process, root, point, "paste")
                click(menu_row(anchor, 5))
                copy_by_menu(process, root, point, environment, "paste\nvalue", "paste-result")
                session.key("Left")
                anchor = open_menu(process, root, point, "unselected")
                click(menu_row(anchor, 6))
                session.control("scroll", "--to", *map(round, menu_row(anchor, 0)), "--notches", 3, "--dy", -1)
                capture(process, root, "disabled-delete")
                before = session.wait_single_thread(process.pid)
                time.sleep(1)
                after = session.resources(process.pid)
                if before["cpu_ticks"] != after["cpu_ticks"] or after["threads"] != 1:
                    raise AssertionError(f"open menu consumed idle CPU: {before}, {after}")
                click(menu_row(anchor, 8))
                session.key("ctrl+c")
                session.wait_clipboard(environment, "paste\nvalue")
                # 6. 【文字菜单回归】【持久提交】菜单全部编辑最终只提交一次标注历史，撤销清空画布，重做后保留原图分辨率
                session.key("Escape")
                committed = capture(process, root, "committed")
                session.key("ctrl+z")
                if body_bounds(capture(process, root, "global-undo"), initial, point):
                    raise AssertionError("menu edits leaked into global annotation history")
                session.key("ctrl+y")
                restored = capture(process, root, "global-redo")
                if not body_bounds(restored, initial, point) or ImageChops.difference(committed, restored).getbbox():
                    raise AssertionError("menu text was not restored exactly")
                session.key("ctrl+s")
                process.wait(timeout=6)
                files = list((private / "export").glob("*.png"))
                if process.returncode or len(files) != 1:
                    raise AssertionError(f"menu export failed: {process.returncode}, {files}")
                with Image.open(files[0]) as image:
                    if image.size != (1600, 900):
                        raise AssertionError(f"menu export changed image size: {image.size}")
                shutil.copyfile(files[0], root / "export.png")
                shutil.copyfile(state / "tools.toml", root / "tools.toml")
                saved = tomllib.loads((state / "tools.toml").read_text())
                if saved["font_size"] != 20 or (private / "data/mark-shot/models").exists():
                    raise AssertionError("menu wheel changed font size or menu actions initialized OCR")
                return {"overlay": overlay, "native_popup_grab": True if require_native else None, "deactivation_dismisses": deactivation,
                        "menu_interrupts_left_double_click": True,
                        "menu_shortcuts_consumed": True, "escape_only_dismisses": True, "outside_click_only_dismisses": True,
                        "cut_copy_delete_verified": True, "menu_undo_redo_local": True, "paste_normalizes_newlines": True,
                        "disabled_delete_preserves_text": True, "select_all_verified": True, "one_global_commit": True,
                        "export_size": [1600, 900], "idle_before": before, "idle_after": after}
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=3)


def main():
    """【文字菜单回归】【程序入口】解析 binary 与 artifacts，保存当前构建指纹和实际验证结果，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--require-native", action="store_true")
    parser.add_argument("--overlay", choices=("layer", "window"), default="layer")
    options = parser.parse_args()
    binary = options.binary.resolve()
    result = {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "scenario": scenario(binary, options.artifacts.resolve(), options.require_native, options.overlay)}
    (options.artifacts / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
