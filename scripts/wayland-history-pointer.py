#!/usr/bin/env python3
"""【截图历史测试】【鼠标职责】在私有桌面验证右键选择与快捷键，不以行为通过代替原版布局验证"""

import argparse
import hashlib
import json
from pathlib import Path
import time

from PIL import Image

import wayland_history_actions as actions
from wayland_history_app import capture_image, configuration, launch, wait_exit
import wayland_history_checks as checks
import wayland_test_session as session
from wayland_text_pointer import click, pointer_context
from wayland_virtual_pointer import transform_for_pointer


def drag(pointer, start, end):
    """【截图历史测试】【持续拖动】用已验证 pointer 从 start 拖到 end，坐标转换遵循当前私有输出，始终释放左键，无返回值"""
    environment = session.environment()
    runtime = json.loads((Path(environment["XDG_RUNTIME_DIR"]) / "runtime.json").read_text())
    logical = next(iter(runtime["outputs"].values()))["logical"]
    positions = [transform_for_pointer(point[0] - logical["x"], point[1] - logical["y"], logical["width"], logical["height"], logical.get("pointer_transform", logical["transform"])) for point in (start, end)]
    pointer.move_abs(*positions[0])
    time.sleep(0.04)
    pointer.press()
    try:
        time.sleep(0.04)
        pointer.move_abs(*positions[1])
        time.sleep(0.08)
    finally:
        pointer.release()
    time.sleep(0.08)


def mechanics(binary, environment, config, history, originals, root):
    """【截图历史测试】【分隔与滚动】在私有 history 增加滚动夹具，运行 binary 并检查画面变化及活动原图不变，返回资源摘要"""
    red = next(path for path in checks.owned(history) if Image.open(path).size == Image.open(originals["red"]).size)
    for index in range(30):
        path = history / f"capture-202601010000{index:05d}-{index:032x}.png"
        path.write_bytes(red.read_bytes())
        path.chmod(0o600)
    process = launch(binary, environment, config, ["--history"], root / "mechanics-process.log")
    try:
        window = session.size_window(actions.window(pid=process.pid), 800, 520)
        actions.focus(window)
        checks.idle(process.pid)
        left, top = window["floating_pos"]
        with pointer_context() as pointer:
            # 1. 【截图历史测试】【原版分隔】拖动九十像素后选中行扩展，预览仍在右侧，不触发窗口移动或记录选择
            drag(pointer, (left + 220, top + 300), (left + 310, top + 300))
            session.screenshot(root / "splitter-expanded.png")
            with Image.open(root / "splitter-expanded.png") as frame:
                assert frame.convert("RGB").getpixel((round(left + 295), round(top + 105))) == (23, 57, 54), "splitter drag did not expand the selected row"
            drag(pointer, (left + 310, top + 300), (left + 220, top + 300))
            # 2. 【截图历史测试】【滚动拖动】滚动到底只移动视口，首项缩略图变为红色，复制仍输出屏幕外选中的蓝色原图
            drag(pointer, (left + 217, top + 90), (left + 217, top + 470))
            checks.idle(process.pid)
            session.screenshot(root / "scrollbar-bottom.png")
            with Image.open(root / "scrollbar-bottom.png") as frame:
                pixel = frame.convert("RGB").getpixel((round(left + 45), round(top + 104)))
                assert pixel[0] > pixel[2] * 1.6, "scrollbar drag did not move the visible records to the bottom"
            session.key("ctrl+c")
            checks.clipboard(environment, originals["blue"], root / "scrolled-selection-copy.png")
        resources = checks.idle(process.pid)
        assert len(checks.owned(history)) == 32, "scrolling or splitter movement changed stored history"
        session.key("Escape")
        wait_exit(process)
        return resources
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=3)


def thumbnail_center(image, window, channel):
    """【截图历史测试】【实际缩略图】从 image 的 window 左侧寻找 channel 色块，返回真实全局中心；不复用原生布局常量"""
    left, top = window["floating_pos"]
    width, height = window["size"]
    points = []
    for y in range(round(top + 80), round(top + height - 70)):
        for x in range(round(left + 12), round(left + min(width * 0.25, 150))):
            pixel = image.getpixel((x, y))
            if pixel[channel] > 110 and all(pixel[channel] > pixel[other] * 1.6 for other in range(3) if other != channel):
                points.append((x, y))
    if len(points) < 100:
        raise AssertionError("the real history thumbnail did not show the expected source color")
    return ((min(x for x, _ in points) + max(x for x, _ in points)) / 2,
            (min(y for _, y in points) + max(y for _, y in points)) / 2)


def run(binary, root):
    """【截图历史测试】【完整投递】在当前私有会话运行 binary，root 保存像素和协议结果，无返回值；不连接用户配置"""
    root.mkdir(parents=True, exist_ok=False)
    environment, config, history, default = configuration(root)
    unchanged = default.read_bytes()
    originals = {}
    for name, size, color in [("red", (512, 288), (220, 25, 30, 210)), ("blue", (384, 240), (25, 35, 220, 210))]:
        source = root / (name + ".png")
        Image.new("RGBA", size, color).save(source)
        originals[name] = capture_image(binary, environment, config, source, root / (name + "-export"), root, name)
    checks.wait_count(history, 2)
    process = launch(binary, environment, config, ["--history"], root / "history-process.log")
    try:
        window = session.size_window(actions.window(pid=process.pid), 800, 520)
        actions.focus(window)
        checks.idle(process.pid)
        session.screenshot(root / "history-before.png")
        with Image.open(root / "history-before.png") as frame:
            red = thumbnail_center(frame.convert("RGB"), window, 0)
            blue = thumbnail_center(frame.convert("RGB"), window, 2)
        # 1. 【截图历史测试】【右键选择】重复右键只改变活动记录，不能建立左键双击或打开编辑窗口
        with pointer_context():
            click(red, "right")
            click(red, "right")
            time.sleep(0.1)
            session.key("ctrl+c")
            checks.clipboard(environment, originals["red"], root / "right-selected-copy.png")
            assert [item["id"] for item in session.windows()] == [window["id"]], "right-click opened an unintended editor"
            # 2. 【截图历史测试】【原版快捷键】Ctrl+F 不抢走图像复制；随后左键仍可改变活动记录
            session.key("ctrl+f")
            session.key("ctrl+c")
            checks.clipboard(environment, originals["red"], root / "after-ctrl-f-copy.png")
            click(blue)
            session.key("ctrl+c")
            checks.clipboard(environment, originals["blue"], root / "left-selected-copy.png")
            # 3. 【截图历史测试】【右键按钮】右键不能执行复制，剪贴板哨兵必须保留
            session.control("copy", "right button sentinel")
            left, top = window["floating_pos"]
            click((left + 88, top + 61), "right")
            assert session.read_clipboard(environment) == "right button sentinel", "right-click activated Copy"
        if default.read_bytes() != unchanged or len(checks.owned(history)) != 2:
            raise AssertionError("selection or copying changed configuration or duplicated history")
        session.key("Escape")
        wait_exit(process)
        resources = mechanics(binary, environment, config, history, originals, root)
        assert default.read_bytes() == unchanged, "mechanics changed the default configuration"
        (root / "result.json").write_text(json.dumps({"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "right_selected_original_rgba": True, "right_double_click_did_not_edit": True, "right_copy_button_did_not_activate": True, "ctrl_f_kept_image_copy": True, "left_selection_preserved": True, "splitter_drag_expanded_row": True, "scrollbar_drag_preserved_selected_original": True, "idle": resources, "records": 32, "no_layout_fidelity_claim": True}, indent=2) + "\n")
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=3)


def main():
    """【截图历史测试】【入口】解析二进制与新证据目录，执行真实鼠标验证；无参数，失败以非零状态退出"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, required=True)
    options = parser.parse_args()
    session.environment()
    run(options.binary.resolve(), options.artifacts.resolve())


if __name__ == "__main__":
    main()
