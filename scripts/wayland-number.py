#!/usr/bin/env python3
"""【Wayland 测试】【独立序号】验证七种样式、计数重置、定点、历史、原图和剪贴板，仅操作隔离桌面"""

import argparse
import hashlib
import io
import json
import math
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import tomllib

from PIL import Image, ImageChops, ImageDraw

from wayland_editor_locator import number_controls
import wayland_test_session as session


BORDER = (96, 113, 127)
DEFAULTS = tomllib.loads((Path(__file__).resolve().parents[1] / "src/config/default.toml").read_text())
STYLES = ["arabic", "upper_alpha", "lower_alpha", "upper_roman", "lower_roman", "heavenly_stem", "chinese"]


def alive(process):
    """【序号测试】【进程检查】确认 process 仍在编辑，提前退出时报告实际状态；无返回值"""
    if process.poll() is not None:
        raise RuntimeError(f"the number editor unexpectedly exited: {process.returncode}")


def capture(path):
    """【序号测试】【界面截图】将隔离桌面写入 path，返回用于比较的 RGB 图像"""
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def quiet(process, path, layout):
    """【序号测试】【悬停隔离】把 process 指针移到 layout 安全位置，返回稳定截图并写入 path"""
    session.control("move", "--logical", 5, layout["safe_y"])
    time.sleep(0.3)
    alive(process)
    return capture(path)


def click(point, count=1):
    """【序号测试】【单击动作】在隔离桌面 point 单击 count 次，等待输入进入应用；无返回值"""
    session.control("click", "--logical", *point, "--count", count)
    time.sleep(0.05)


def difference(first, second, region):
    """【序号测试】【区域比较】返回 first、second 在 region 内的差异矩形，无差异时返回空值"""
    return ImageChops.difference(first.crop(region), second.crop(region)).getbbox()


def geometry(environment, source):
    """【序号测试】【界面布局】按 environment 的单输出和 source 计算视口、定点、样式网格及比较区域；返回逻辑坐标"""
    outputs = json.loads(subprocess.check_output(["niri", "msg", "-j", "outputs"], env=environment))
    if len(outputs) != 1:
        raise RuntimeError("the numbering regression requires one unambiguous output")
    logical = next(iter(outputs.values()))["logical"]
    width, height = logical["width"], logical["height"]
    scale = min((width - 64) / source.width, (height - 96) / source.height, 1)
    left, top = (width - source.width * scale) / 2, (height - source.height * scale) / 2
    point = lambda x, y: (round(width * x / 5) * 5, round(height * y / 5) * 5)
    return {
        "first": point(0.23, 0.38),
        "bubble": point(0.44, 0.42),
        "tip": point(0.62, 0.5),
        "styles": [point(0.19 + index * 0.09, 0.71) for index in range(7)],
        "cancel_start": point(0.82, 0.31),
        "cancel_end": point(0.84, 0.55),
        "safe_y": height - 6,
        "region": (50, math.ceil(top) + 90, width - 50, height - 50),
        "viewport": (left, top, scale),
    }


def ready(process, path):
    """【序号测试】【启动等待】等待 process 绘制源图，将证据写入 path；返回稳定界面，超时抛出异常"""
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        alive(process)
        image = capture(path)
        if image.getpixel((image.width // 2 - 110, image.height // 2)) == (30, 34, 38):
            session.wait_single_thread(process.pid)
            time.sleep(0.3)
            return capture(path)
        time.sleep(0.04)
    raise RuntimeError("the numbering source image did not appear")


def choose_style(process, artifacts, layout, index):
    """【序号测试】【样式选择】在 process 的真实弹层中选择 index 样式；layout 提供安全位置，artifacts 保存可见面板和结果"""
    image = quiet(process, artifacts / f"style-{index}-before.png", layout)
    click(number_controls(image, BORDER)["style"])
    time.sleep(0.2)
    capture(artifacts / f"style-{index}-popover.png")
    session.key("Home")
    for _ in range(index):
        session.key("Right")
    session.key("Return")
    alive(process)


def reset_sequence(process, artifacts, layout, name):
    """【序号测试】【重置按钮】在 process 中点击真实重置按钮；layout 和 artifacts 记录重置不修改持久像素，无返回值"""
    before = quiet(process, artifacts / f"{name}-before.png", layout)
    click(number_controls(before, BORDER)["reset"])
    after = quiet(process, artifacts / f"{name}-after.png", layout)
    if difference(before, after, layout["region"]):
        raise RuntimeError("resetting the numbering sequence altered existing annotations")


def check_counter_history(process, artifacts, layout):
    """【序号测试】【计数历史】在 process 中以同坐标标签验证重置、撤销和重做；layout 提供定点，artifacts 保存像素证据"""
    click(layout["first"])
    original = quiet(process, artifacts / "number-one.png", layout)
    session.control("drag", "--from", *layout["bubble"], "--to", *layout["tip"], "--steps", 10)
    baseline = quiet(process, artifacts / "number-two.png", layout)
    x, y = layout["first"]
    label = (x - 11, y - 11, x + 12, y + 12)
    reset_sequence(process, artifacts, layout, "reset-undo")
    session.key("ctrl+z")
    click(layout["first"])
    third = quiet(process, artifacts / "counter-restored-three.png", layout)
    if not difference(original, third, label):
        raise RuntimeError("undoing reset did not restore the next sequence number")
    session.key("ctrl+z")
    restored = quiet(process, artifacts / "counter-new-undo.png", layout)
    if difference(baseline, restored, layout["region"]):
        raise RuntimeError("the new sequence number did not undo in one transaction")
    session.key("ctrl+shift+z")
    redone = quiet(process, artifacts / "counter-new-redo.png", layout)
    if difference(third, redone, layout["region"]):
        raise RuntimeError("the new sequence number did not redo in one transaction")
    session.key("ctrl+z")
    reset_sequence(process, artifacts, layout, "reset-to-one")
    click(layout["first"])
    first_again = quiet(process, artifacts / "counter-reset-one.png", layout)
    if difference(original, first_again, label):
        raise RuntimeError("reset did not render the original number one at the same coordinate")
    session.key("ctrl+z")
    session.key("ctrl+z")


def check_drag(process, artifacts, layout, name, start, end):
    """【序号测试】【已有拖动】在 process 中从 start 拖到 end 并验证单次历史；layout 限定比较区域，artifacts 保存三个状态"""
    before = quiet(process, artifacts / f"{name}-before.png", layout)
    session.control("drag", "--from", *start, "--to", *end, "--steps", 10)
    changed = quiet(process, artifacts / f"{name}-changed.png", layout)
    if not difference(before, changed, layout["region"]):
        raise RuntimeError(f"{name} did not update the existing number")
    session.key("ctrl+z")
    undone = quiet(process, artifacts / f"{name}-undo.png", layout)
    if difference(before, undone, layout["region"]):
        raise RuntimeError(f"{name} did not undo exactly")
    session.key("ctrl+shift+z")
    redone = quiet(process, artifacts / f"{name}-redo.png", layout)
    if difference(changed, redone, layout["region"]):
        raise RuntimeError(f"{name} did not redo exactly")
    session.key("ctrl+z")


def edit_width(process, artifacts, layout):
    """【序号测试】【独立笔宽】在 process 的真实输入框粘贴九像素笔宽，验证确认及历史；layout、artifacts 保存证据"""
    before = quiet(process, artifacts / "width-before.png", layout)
    click(number_controls(before, BORDER)["width"], 2)
    session.key("ctrl+a")
    session.paste_text("9")
    session.key("Return")
    changed = quiet(process, artifacts / "width-nine.png", layout)
    if not difference(before, changed, layout["region"]):
        raise RuntimeError("the numbering width input did not update the selected number")
    session.key("ctrl+z")
    undone = quiet(process, artifacts / "width-undo.png", layout)
    if difference(before, undone, layout["region"]):
        raise RuntimeError("numbering width did not undo in one transaction")
    session.key("ctrl+shift+z")
    redone = quiet(process, artifacts / "width-redo.png", layout)
    if difference(changed, redone, layout["region"]):
        raise RuntimeError("numbering width did not redo exactly")


def check_cancel(process, artifacts, layout):
    """【序号测试】【取消手势】在 process 中保持新序号并取消；layout 给出定点，artifacts 验证旧图层精确恢复"""
    session.key(DEFAULTS["keys"]["tool_number"])
    before = quiet(process, artifacts / "cancel-before.png", layout)
    with subprocess.Popen(["niri-use", "--child", "click", "--logical", *map(str, layout["cancel_start"]), "--hold", "6"], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE) as gesture:
        time.sleep(0.2)
        session.control("move", "--logical", *layout["cancel_end"])
        draft = capture(artifacts / "cancel-active.png")
        if gesture.poll() is not None or not difference(before, draft, layout["region"]):
            raise RuntimeError("the pending number was not observable while the button stayed pressed")
        session.key("Escape")
        alive(process)
        gesture.wait(timeout=8)
        if gesture.returncode:
            raise RuntimeError(gesture.stderr.read().decode())
    restored = quiet(process, artifacts / "cancel-restored.png", layout)
    if difference(before, restored, layout["region"]):
        raise RuntimeError("canceling a numbering draft altered committed pixels")


def check_persistence(binary, environment, temporary, artifacts, source_path, layout):
    """【序号测试】【偏好重开】以 binary、environment 重开 source_path，验证九像素中文默认值和新会话从一开始；temporary、artifacts 保存输出"""
    with (artifacts / "reload.log").open("w") as log:
        process = subprocess.Popen([str(binary), "--fullscreen", "--fullscreen-default-tool", "number", "--to", "cs", "--output", str(temporary / "reload-export"), str(source_path)], env=environment, stderr=log)
        try:
            ready(process, artifacts / "reload-baseline.png")
            click(layout["first"])
            image = quiet(process, artifacts / "reload-chinese-one.png", layout)
            x, y = layout["first"]
            pixel = image.getpixel((x + 22, y))
            if pixel[0] < 200 or pixel[1] > 150:
                raise RuntimeError(f"the reloaded default width did not enlarge the circle: {pixel}")
            session.key("Return")
            process.wait(timeout=5)
            if process.returncode:
                raise RuntimeError(f"reloaded numbering export failed: {process.returncode}")
            files = list((temporary / "reload-export").rglob("*.png"))
            if len(files) != 1:
                raise RuntimeError("the reloaded numbering session did not produce exactly one image")
            encoded = files[0].read_bytes()
            clipboard = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment)
            with Image.open(io.BytesIO(encoded)) as saved, Image.open(io.BytesIO(clipboard)) as copied:
                output = saved.convert("RGBA")
                if output.tobytes() != copied.convert("RGBA").tobytes():
                    raise RuntimeError("the reloaded numbering clipboard did not survive editor exit")
                left, top, scale = layout["viewport"]
                center = (round((x - left) / scale), round((y - top) / scale))
                points = [(px, py) for py in range(-20, 21) for px in range(-20, 21) if min(output.getpixel((center[0] + px, center[1] + py))[:3]) > 230]
                if not points:
                    raise RuntimeError("the reloaded Chinese numbering label has no white pixels")
                label_width = max(px for px, _ in points) - min(px for px, _ in points) + 1
                label_height = max(py for _, py in points) - min(py for _, py in points) + 1
                if label_width <= label_height * 3:
                    raise RuntimeError(f"the new numbering session did not start at Chinese one: {label_width}x{label_height}")
            (artifacts / "reload-image.png").write_bytes(encoded)
            (artifacts / "reload-clipboard.png").write_bytes(clipboard)
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)


def run(binary, artifacts):
    """【序号测试】【完整回归】用 binary 验证全部序号流程，artifacts 保存指纹和证据；普通编辑不初始化识别模型"""
    environment = session.environment()
    artifacts.mkdir(parents=True, exist_ok=True)
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    with tempfile.TemporaryDirectory(prefix="mark-shot-number-test-") as directory:
        temporary = Path(directory)
        config, state = temporary / "config/mark-shot", temporary / "state/mark-shot"
        config.mkdir(parents=True)
        state.mkdir(parents=True)
        (config / "config.toml").write_text('[general]\nto = "cs"\n[notifications]\nenabled = false\n[theme]\nborder = "#60717FFF"\nborder_on_light = "#60717FFF"\n[tools]\nfill = true\n[tools.stroke]\nwidth = 3.0\n')
        (state / "tools.toml").write_text('magnifier = false\n')
        environment.update(XDG_CONFIG_HOME=str(temporary / "config"), XDG_DATA_HOME=str(temporary / "data"), XDG_STATE_HOME=str(temporary / "state"))
        source = Image.new("RGBA", (1600, 900), (30, 34, 38, 255))
        painter = ImageDraw.Draw(source)
        painter.rectangle((0, 0, 1599, 7), fill=(0, 0, 0, 0))
        painter.rectangle((20, 20, 42, 42), fill=(255, 0, 0, 128))
        source_path = temporary / "source.png"
        source.save(source_path)
        layout = geometry(environment, source)
        with (artifacts / "process.log").open("w") as log:
            process = subprocess.Popen([str(binary), "--fullscreen", "--fullscreen-default-tool", "number", "--to", "cs", "--output", str(temporary / "export"), str(source_path)], env=environment, stderr=log)
            try:
                ready(process, artifacts / "baseline.png")
                check_counter_history(process, artifacts, layout)
                for index, point in enumerate(layout["styles"]):
                    choose_style(process, artifacts, layout, index)
                    before = quiet(process, artifacts / f"style-{index}-empty.png", layout)
                    click(point)
                    drawn = quiet(process, artifacts / f"style-{index}-drawn.png", layout)
                    if not difference(before, drawn, (point[0] - 22, point[1] - 22, point[0] + 23, point[1] + 23)):
                        raise RuntimeError(f"the {STYLES[index]} numbering style did not render")
                session.key(DEFAULTS["keys"]["tool_pick"])
                click(layout["bubble"])
                bubble, tip = layout["bubble"], layout["tip"]
                check_drag(process, artifacts, layout, "bubble", bubble, (bubble[0] + 40, bubble[1] - 15))
                check_drag(process, artifacts, layout, "tip", tip, (tip[0] + 25, tip[1] - 40))
                radius = 13 + 3 * 1.35 + 1
                corner = (round(tip[0] + 6), round(min(bubble[1] - radius, tip[1]) - 6))
                check_drag(process, artifacts, layout, "resize", corner, (corner[0] + 25, corner[1] - 35))
                handle = (round((bubble[0] - radius + tip[0]) / 2), round(bubble[1] - radius - 28))
                delta = (handle[0] - bubble[0], handle[1] - bubble[1])
                angle = math.radians(32)
                rotated = (round(bubble[0] + delta[0] * math.cos(angle) - delta[1] * math.sin(angle)), round(bubble[1] + delta[0] * math.sin(angle) + delta[1] * math.cos(angle)))
                check_drag(process, artifacts, layout, "rotation", handle, rotated)
                choose_style(process, artifacts, layout, 3)
                edit_width(process, artifacts, layout)
                check_cancel(process, artifacts, layout)
                session.wait_single_thread(process.pid)
                time.sleep(1)
                before = session.resources(process.pid)
                time.sleep(1)
                after = session.resources(process.pid)
                if after["threads"] != 1 or before["cpu_ticks"] != after["cpu_ticks"]:
                    raise RuntimeError(f"the stable numbering editor keeps polling: {before} -> {after}")
                (artifacts / "resources.json").write_text(json.dumps({"before": before, "after": after}, indent=2) + "\n")
                session.key("Return")
                process.wait(timeout=5)
                if process.returncode:
                    raise RuntimeError(f"numbering export failed: {process.returncode}")
                files = list((temporary / "export").rglob("*.png"))
                if len(files) != 1:
                    raise RuntimeError(f"expected one saved numbering image, received {len(files)}")
                encoded = files[0].read_bytes()
                clipboard = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment)
                with Image.open(io.BytesIO(encoded)) as saved, Image.open(io.BytesIO(clipboard)) as copied:
                    output = saved.convert("RGBA")
                    if output.size != source.size or copied.convert("RGBA").tobytes() != output.tobytes():
                        raise RuntimeError("numbering save and clipboard output differ or lost original resolution")
                    for region in [(0, 0, 1600, 8), (20, 20, 43, 43), (1000, 790, 1550, 875)]:
                        if output.crop(region).tobytes() != source.crop(region).tobytes():
                            raise RuntimeError("numbering export changed untouched pixels or original transparency")
                (artifacts / "saved-image.png").write_bytes(encoded)
                (artifacts / "clipboard.png").write_bytes(clipboard)
                saved = tomllib.loads((state / "tools.toml").read_text())
                if saved["number_style"] != "chinese" or saved["stroke_widths"]["number"] != 9 or saved["stroke_widths"]["numbered"] != 3:
                    raise RuntimeError(f"numbering preferences did not persist independently: {saved}")
                if "number_sequence" in saved or "next_number" in saved:
                    raise RuntimeError("the session sequence leaked into persistent tool preferences")
                shutil.copyfile(state / "tools.toml", artifacts / "tools.toml")
                check_persistence(binary, environment, temporary, artifacts, source_path, layout)
                if (temporary / "data/mark-shot/models").exists():
                    raise RuntimeError("ordinary numbering editing initialized the OCR model directory")
                if fingerprint != hashlib.sha256(binary.read_bytes()).hexdigest():
                    raise RuntimeError("the executable changed during the numbering regression")
                (artifacts / "result.json").write_text(json.dumps({"sha256": fingerprint, "styles": STYLES, "width": 9, "source_size": source.size, "sequence_reset": True, "counter_undo_redo": True, "bubble_move": True, "tip_move": True, "resize": True, "rotation": True, "draft_cancellation": True, "preference_reload": True, "persistent_clipboard": True}, indent=2) + "\n")
                print("Wayland numbering styles, sequence history, controls, properties, idle and native export checks passed")
            finally:
                application_log = state / "mark-shot.log"
                if application_log.exists():
                    shutil.copyfile(application_log, artifacts / "application.log")
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=3)


def main():
    """【序号测试】【入口】解析二进制和证据目录并执行回归；失败时返回非零状态"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, default=Path("target/wayland-number"))
    options = parser.parse_args()
    run(options.binary.resolve(), options.artifacts.resolve())


if __name__ == "__main__":
    main()
