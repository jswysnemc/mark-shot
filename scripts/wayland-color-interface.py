#!/usr/bin/env python3
"""【颜色回归】【真实输入】在隔离 Wayland 桌面验证两种取色器、历史事务与原图导出"""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import tomllib

from PIL import Image, ImageChops
from wayland_editor_locator import color_popover_rectangle, settings_rectangle
import wayland_test_session as session


BORDER = (96, 113, 127)
POSITIONS = ((74, 6), (114, 19), (139, 53), (139, 95), (114, 129),
             (74, 142), (34, 129), (9, 95), (9, 53), (34, 19))
COLORS = ((239, 68, 68), (249, 115, 22), (234, 179, 8), (34, 197, 94), (6, 182, 212),
          (59, 130, 246), (139, 92, 246), (236, 72, 153), (248, 250, 252), (15, 23, 42))


def alive(process):
    """【颜色回归】【进程校验】要求 process 保持编辑会话；提前退出时报告状态，无返回值"""
    if process.poll() is not None:
        raise AssertionError(f"the color editor exited unexpectedly: {process.returncode}")


def capture(process, artifacts, name, quiet=True):
    """【颜色回归】【截图证据】保存 process 的 name 截图到 artifacts；quiet 移开指针，返回逻辑 RGB 图像"""
    if quiet:
        session.control("move", "--logical", 5, 600)
    time.sleep(0.2)
    alive(process)
    path = artifacts / f"{name}.png"
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def click(point, button="left", count=1):
    """【颜色回归】【按钮输入】在 point 发送 button 的 count 次完整点击；只操作私有桌面，无返回值"""
    session.control("click", "--logical", *map(round, point), "--button", button, "--count", count)
    time.sleep(0.05)


def property_button(image):
    """【颜色回归】【按钮定位】按 image 的可见属性框返回主颜色按钮中心，不推算工具栏位置"""
    left, top, _right, bottom = settings_rectangle(image, BORDER)
    return left + 22, (top + bottom) / 2


def same(first, second, region):
    """【颜色回归】【像素核对】比较 first、second 的 region；返回区域是否逐像素相同"""
    return ImageChops.difference(first.crop(region), second.crop(region)).getbbox() is None


def hex_input(rectangle, text):
    """【颜色回归】【字段粘贴】在 rectangle 的唯一十六进制字段粘贴 text，不发送确认键，无返回值"""
    left, top, _right, _bottom = rectangle
    click((left + 100, top + 175))
    session.key("ctrl+a")
    session.paste_text(text)


def radial_matches(image, anchor):
    """【颜色回归】【环形验证】检查 image 的 anchor 周围十色按钮；返回是否全部匹配，容许分数缩放通道误差"""
    left, top = anchor[0] - 89, anchor[1] - 89
    return all(max(abs(a - b) for a, b in zip(image.getpixel((left + x + 14, top + y + 14)), color)) <= 2
               for (x, y), color in zip(POSITIONS, COLORS))


def run(binary, artifacts):
    """【颜色回归】【完整场景】使用 binary 执行真实粘贴、手势、快捷色板与导出核对，artifacts 保留全部证据与构建指纹"""
    environment = session.environment()
    artifacts.mkdir(parents=True, exist_ok=True)
    defaults = tomllib.loads((Path(__file__).resolve().parents[1] / "src/config/default.toml").read_text())
    with tempfile.TemporaryDirectory(prefix="mark-shot-color-") as directory:
        root = Path(directory)
        config = root / "config/mark-shot"
        state = root / "state/mark-shot"
        config.mkdir(parents=True)
        state.mkdir(parents=True)
        (config / "config.toml").write_text('[general]\nto = "s"\n[notifications]\nenabled = false\n[theme]\nborder = "#60717FFF"\nborder_on_light = "#60717FFF"\n')
        (state / "tools.toml").write_text('magnifier = false\nmarker_shape = "square"\nmarker_filled = true\nrecent_colors = ["#3B82F6", "#EAB308", "#EC4899", "#0F172A"]\n')
        environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
        source = root / "source.png"
        Image.new("RGBA", (1600, 900), (30, 34, 38, 255)).save(source)
        with (artifacts / "process.log").open("w") as log:
            process = subprocess.Popen([str(binary), "--fullscreen", "--fullscreen-default-tool", "marker", "--default-color", "#FF4D4D", "--to", "s", "--output", str(root / "export"), str(source)], env=environment, stderr=log)
            try:
                deadline = time.monotonic() + 8
                while True:
                    initial = capture(process, artifacts, "initial")
                    if initial.getpixel((initial.width // 2, initial.height // 2)) == (30, 34, 38):
                        break
                    if time.monotonic() >= deadline:
                        raise AssertionError("the source image did not appear")
                session.wait_single_thread(process.pid)
                start = (round(initial.width * 0.72), round(initial.height * 0.65))
                end = (start[0] + 70, start[1] + 55)
                region = (start[0] - 15, start[1] - 15, end[0] + 15, end[1] + 15)
                session.control("drag", "--from", *start, "--to", *end, "--steps", 8)
                session.key(defaults["keys"]["tool_pick"])
                click(((start[0] + end[0]) / 2, (start[1] + end[1]) / 2))
                selected = capture(process, artifacts, "selected")
                toggle = property_button(selected)

                # 1. 【颜色回归】【字段与轨道】文本只在确认时应用，横向 HSV 与 alpha 手势不提前导出
                click(toggle)
                opened = capture(process, artifacts, "property-open")
                rectangle = color_popover_rectangle(opened)
                hex_input(rectangle, "#22C55E80")
                pasted = capture(process, artifacts, "hex-pasted", quiet=False)
                if not same(selected, pasted, region):
                    raise AssertionError("hex typing changed the annotation before confirmation")
                session.key("Return")
                session.key("Return")
                committed = capture(process, artifacts, "hex-committed")
                if same(selected, committed, region):
                    raise AssertionError("the confirmed hex field did not recolor the annotation")
                left, top, _right, _bottom = rectangle
                for name, begin, finish in [("sv", (left + 55, top + 50), (left + 150, top + 75)),
                                             ("hue", (left + 30, top + 132), (left + 130, top + 132)),
                                             ("alpha", (left + 130, top + 150), (left + 70, top + 150))]:
                    session.control("drag", "--from", *map(round, begin), "--to", *map(round, finish), "--steps", 10)
                    capture(process, artifacts, f"{name}-dragged")
                hex_input(rectangle, "#06B6D4A0")
                session.key("Return")
                capture(process, artifacts, "final-hex")
                click(toggle)
                modified = capture(process, artifacts, "property-closed")

                # 2. 【颜色回归】【单次撤销】整次打开共用历史，撤销重做恢复原像素，重新打开的历史色块来自实际记录
                session.key("ctrl+z")
                undone = capture(process, artifacts, "property-undo")
                if not same(selected, undone, region):
                    raise AssertionError("one undo did not restore all gestures from the picker opening")
                session.key("ctrl+shift+z")
                redone = capture(process, artifacts, "property-redo")
                if not same(modified, redone, region):
                    raise AssertionError("one redo did not restore the final picker color")
                click(toggle)
                history = capture(process, artifacts, "history-open")
                history_rect = color_popover_rectangle(history)
                history_left, history_top, _right, _bottom = history_rect
                expected = (6, 182, 212)
                sample = history.getpixel((history_left + 21, history_top + 207))
                if min(sample) >= 160 or sample[2] <= sample[0]:
                    raise AssertionError(f"the current history slot does not show the semi-transparent cyan: {sample}")
                session.wait_single_thread(process.pid)
                time.sleep(1)
                idle_before = session.resources(process.pid)
                time.sleep(1)
                idle_after = session.resources(process.pid)
                if idle_before["threads"] != 1 or idle_after["threads"] != 1 or idle_before["cpu_ticks"] != idle_after["cpu_ticks"]:
                    raise AssertionError(f"the idle color picker retained work: {idle_before} -> {idle_after}")
                click(toggle)

                # 3. 【颜色回归】【快捷职责】两对右键双击切换透明环形色板，左键选择同时更新默认色与选中标注
                anchor = (420, 280)
                click(anchor, "right", 2)
                radial = capture(process, artifacts, "radial-open")
                if not radial_matches(radial, anchor):
                    raise AssertionError("right double-click did not open the original ten-color radial palette")
                click(anchor, "right", 2)
                closed = capture(process, artifacts, "radial-closed")
                if radial_matches(closed, anchor):
                    raise AssertionError("a second right double-click did not close the radial palette")
                click(anchor, "right", 2)
                choice = (anchor[0] - 89 + POSITIONS[4][0] + 14, anchor[1] - 89 + POSITIONS[4][1] + 14)
                click(choice)
                chosen = capture(process, artifacts, "radial-chosen")
                if radial_matches(chosen, anchor):
                    raise AssertionError("a radial color choice left the palette open")
                if chosen.getpixel(((start[0] + end[0]) // 2, (start[1] + end[1]) // 2)) != expected:
                    raise AssertionError("the radial choice did not recolor the selected annotation")

                # 4. 【颜色回归】【原图输出】新标注沿用快捷默认色，导出保持源图尺寸、背景和不透明 cyan，不包含控件或 OCR 模型
                session.key(defaults["keys"]["tool_marker"])
                second = (start[0] - 130, start[1])
                session.control("drag", "--from", *second, "--to", second[0] + 70, second[1] + 55, "--steps", 8)
                capture(process, artifacts, "second-annotation")
                session.key("Return")
                process.wait(timeout=5)
                if process.returncode:
                    raise AssertionError(f"export failed: {process.returncode}")
                files = list((root / "export").glob("*.png"))
                if len(files) != 1:
                    raise AssertionError(f"the color scenario did not export one PNG: {files}")
                with Image.open(files[0]) as output:
                    output = output.convert("RGBA")
                    if output.size != (1600, 900) or output.getpixel((800, 450)) != (30, 34, 38, 255):
                        raise AssertionError("color controls damaged the original image size or unchanged source pixels")
                    count = sum(pixel == (*expected, 255) for pixel in output.get_flattened_data())
                    if count < 1000:
                        raise AssertionError("the opaque radial default did not reach the original-resolution output")
                saved = tomllib.loads((state / "tools.toml").read_text())
                if saved["color"] != "#06B6D4":
                    raise AssertionError(f"the radial default did not persist: {saved['color']}")
                if (root / "data/mark-shot/models").exists():
                    raise AssertionError("color editing created an OCR model directory")
                shutil.copyfile(files[0], artifacts / "export.png")
                shutil.copyfile(state / "tools.toml", artifacts / "tools.toml")
                result = {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "property_size": [200, 228],
                          "radial_size": [178, 178], "property_rectangle": rectangle, "hex_commit": True,
                          "repeat_enter_stays_in_field": True,
                          "sv_drag": True, "hue_drag": True, "alpha_drag": True, "one_opening_undo_redo": True,
                          "two_right_double_click_pairs": True, "radial_selected_and_default": True,
                          "export_size": [1600, 900], "opaque_cyan_pixels": count, "ocr_models_created": False}
                result["picker_idle"] = {"before": idle_before, "after": idle_after, "sample_seconds": 1}
                (artifacts / "result.json").write_text(json.dumps(result, indent=2) + "\n")
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=3)
    print("Wayland detailed picker, radial palette, color transaction and original export checks passed")


def main():
    """【颜色回归】【命令入口】解析构建产物与证据目录，返回真实输入回归状态；失败不覆盖历史结果"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, default=Path("target/wayland-color-interface"))
    options = parser.parse_args()
    run(options.binary.resolve(), options.artifacts.resolve())


if __name__ == "__main__":
    main()
