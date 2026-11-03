#!/usr/bin/env python3
"""【Wayland 测试】【尺子回归】验证测量模式、控制点、原图导出与空闲资源，仅操作独立桌面"""

import argparse
import io
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import tomllib

from PIL import Image, ImageChops, ImageDraw

from wayland_editor_locator import ruler_controls
import wayland_test_session as session


BORDER = (96, 113, 127)
DEFAULTS = tomllib.loads((Path(__file__).resolve().parents[1] / "src/config/default.toml").read_text())


def capture(path):
    """【尺子测试】【界面截图】把隔离桌面写入 path，返回可比较的 RGB 图像"""
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def alive(process):
    """【尺子测试】【进程检查】确认 process 仍处于编辑会话；提前退出时抛出异常"""
    if process.poll() is not None:
        raise RuntimeError(f"the ruler editor unexpectedly exited: {process.returncode}")


def ready(process, path):
    """【尺子测试】【启动等待】等待 process 绘制源图并把证据写入 path；返回稳定界面，超时抛出异常"""
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        alive(process)
        image = capture(path)
        if image.getpixel((image.width // 2 - 24, image.height // 2 - 24)) == (30, 34, 38):
            time.sleep(0.25)
            return capture(path)
        time.sleep(0.04)
    raise RuntimeError("the ruler source image did not appear")


def click(position, count=1):
    """【尺子测试】【点击动作】在子会话 position 点击 count 次；等待输入进入应用队列，无返回值"""
    session.control("click", "--logical", *position, "--count", count)
    time.sleep(0.04)


def difference(first, second, region):
    """【尺子测试】【区域差异】返回 first、second 在 region 内的非空差异矩形或空值"""
    return ImageChops.difference(first.crop(region), second.crop(region)).getbbox()


def no_hover(process, path, y):
    """【尺子测试】【悬停隔离】将 process 指针移出源图并把稳定界面写入 path；y 指定安全高度，返回像素图"""
    session.control("move", "--logical", 5, y)
    time.sleep(0.08)
    alive(process)
    return capture(path)


def choose_mode(process, artifacts, index):
    """【尺子测试】【模式选择】通过 process 的统一属性弹层选择 index；artifacts 保存界面，Enter 不导出，无返回值"""
    image = capture(artifacts / "before-mode.png")
    controls = ruler_controls(image, BORDER)
    click(controls["mode"])
    time.sleep(0.25)
    opened = capture(artifacts / "mode-choices.png")
    if ImageChops.difference(image, opened).getbbox() is None:
        raise RuntimeError("the ruler mode choice did not open")
    session.control("key", "Home", *("Right" for _ in range(index)), "Return")
    time.sleep(0.25)
    alive(process)


def check_mode_popover(process, artifacts, y):
    """【尺子测试】【面板交互】验证 process 模式弹层的 Escape、按钮关闭与空闲；y 用于排除悬停，artifacts 保存证据"""
    baseline = no_hover(process, artifacts / "before-popover.png", y)
    controls = ruler_controls(baseline, BORDER)
    click(controls["mode"])
    time.sleep(0.3)
    session.wait_single_thread(process.pid)
    before = session.resources(process.pid)
    time.sleep(1)
    after = session.resources(process.pid)
    if before["cpu_ticks"] != after["cpu_ticks"]:
        raise RuntimeError(f"the stable ruler mode popover keeps polling: {before} -> {after}")
    (artifacts / "popover-resources.json").write_text(json.dumps({"before": before, "after": after}, indent=2) + "\n")
    session.key("Escape")
    alive(process)
    time.sleep(0.3)
    closed = no_hover(process, artifacts / "popover-escape.png", y)
    if ImageChops.difference(baseline, closed).getbbox():
        raise RuntimeError("closing the ruler mode popover left stale pixels")
    click(controls["mode"])
    time.sleep(0.3)
    click(controls["mode"])
    time.sleep(0.3)
    closed = no_hover(process, artifacts / "popover-toggle-close.png", y)
    if ImageChops.difference(baseline, closed).getbbox():
        raise RuntimeError("clicking the open ruler mode button reopened it")


def check_length_edits(process, start, end, region, artifacts, y):
    """【尺子测试】【长度编辑】在 process 选中 start、end 长度尺并验证控制点与旋转；region 限定像素比较，artifacts 保存证据"""
    session.key(DEFAULTS["keys"]["tool_pick"])
    click((round((start[0] + end[0]) / 2), start[1] + 1))
    selected = no_hover(process, artifacts / "length-selected.png", y)
    session.control("drag", "--from", *end, "--to", end[0] + 35, end[1] + 25, "--steps", 8)
    resized = no_hover(process, artifacts / "length-resized.png", y)
    if not difference(selected, resized, region):
        raise RuntimeError("the selected length ruler corner did not resize")
    session.key("ctrl+z")
    restored = no_hover(process, artifacts / "length-resize-undo.png", y)
    if difference(selected, restored, region):
        raise RuntimeError("length ruler resize did not undo in one transaction")

    # 1. 【尺子测试】【长度编辑】旋转把手使用独立命中，旋转只改变几何方向，不复制尺子
    center = (round((start[0] + end[0]) / 2), round((start[1] + end[1]) / 2))
    handle = (center[0], start[1] - 52)
    session.control("drag", "--from", *handle, "--to", center[0] + 120, center[1], "--steps", 8)
    rotated = no_hover(process, artifacts / "length-rotated.png", y)
    if not difference(selected, rotated, region):
        raise RuntimeError("the selected length ruler rotation handle did not rotate")
    session.key("ctrl+z")
    if difference(selected, no_hover(process, artifacts / "length-rotation-undo.png", y), region):
        raise RuntimeError("length ruler rotation did not undo in one transaction")


def check_angle_edits(process, vertex, first, second, region, artifacts, y):
    """【尺子测试】【角度编辑】在 process 调整 vertex、first、second 角度端点；region 用于撤销、重做对比，artifacts 保存证据"""
    session.key(DEFAULTS["keys"]["tool_pick"])
    click((round((vertex[0] + first[0]) / 2), vertex[1] + 1))
    selected = no_hover(process, artifacts / "angle-selected.png", y)
    session.control("drag", "--from", *first, "--to", first[0], first[1] + 45, "--steps", 8)
    changed = no_hover(process, artifacts / "angle-arm-modified.png", y)
    if not difference(selected, changed, region):
        raise RuntimeError("the selected angle arm did not move independently")
    session.key("ctrl+z")
    if difference(selected, no_hover(process, artifacts / "angle-arm-undo.png", y), region):
        raise RuntimeError("angle arm adjustment did not undo in one transaction")
    session.key("ctrl+shift+z")
    if difference(changed, no_hover(process, artifacts / "angle-arm-redo.png", y), region):
        raise RuntimeError("angle arm adjustment did not redo in one transaction")
    session.key("ctrl+z")

    # 1. 【尺子测试】【角度编辑】真实剪贴板粘贴修改独立尺子笔宽，字段确认不触发全局导出
    controls = ruler_controls(capture(artifacts / "before-width.png"), BORDER)
    click(controls["width"], 2)
    session.key("ctrl+a")
    session.paste_text("9")
    session.key("Return")
    alive(process)
    modified = no_hover(process, artifacts / "angle-width-modified.png", y)
    if not difference(selected, modified, region):
        raise RuntimeError("pasting the ruler width did not change its style")


def run(binary, artifacts):
    """【尺子测试】【完整回归】使用 binary 验证真实测量、编辑、消除悬停和原图保存；证据写入 artifacts，无返回值"""
    environment = session.environment()
    artifacts.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="mark-shot-ruler-test-") as directory:
        temporary = Path(directory)
        config = temporary / "config/mark-shot"
        state = temporary / "state/mark-shot"
        config.mkdir(parents=True)
        state.mkdir(parents=True)
        (config / "config.toml").write_text('[general]\nto = "cs"\n[notifications]\nenabled = false\n[theme]\nborder = "#60717FFF"\nborder_on_light = "#60717FFF"\n')
        (state / "tools.toml").write_text('magnifier = false\n')
        environment.update(XDG_CONFIG_HOME=str(temporary / "config"), XDG_DATA_HOME=str(temporary / "data"), XDG_STATE_HOME=str(temporary / "state"))
        source = Image.new("RGBA", (1600, 900), (30, 34, 38, 255))
        painter = ImageDraw.Draw(source)
        painter.rectangle((0, 0, 1599, 7), fill=(0, 0, 0, 0))
        painter.rectangle((20, 20, 42, 42), fill=(255, 0, 0, 128))
        source_path = temporary / "source.png"
        source.save(source_path)
        with (artifacts / "process.log").open("w") as log:
            process = subprocess.Popen([str(binary), "--fullscreen", "--fullscreen-default-tool", "ruler", "--default-color", "#FF3B30", "--to", "cs", "--output", str(temporary / "export"), str(source_path)], env=environment, stderr=log)
            try:
                image = ready(process, artifacts / "baseline.png")
                y = round(image.height * 0.85)
                baseline = no_hover(process, artifacts / "without-hover.png", y)
                check_mode_popover(process, artifacts, y)
                start = (round(image.width * 0.21), round(image.height * 0.33))
                end = (start[0] + 160, start[1] + 125)
                first_region = (start[0] - 75, start[1] - 80, end[0] + 120, end[1] + 110)

                # 1. 【尺子测试】【完整回归】长度测量与空手势经过同一原生鼠标输入链路
                click(start)
                if difference(baseline, no_hover(process, artifacts / "empty-length.png", y), first_region):
                    raise RuntimeError("an empty length gesture created a ruler")
                session.control("drag", "--from", *start, "--to", *end, "--steps", 12)
                length = no_hover(process, artifacts / "length.png", y)
                if not difference(baseline, length, first_region):
                    raise RuntimeError("drawing a length ruler did not show its measurement")
                check_length_edits(process, start, end, first_region, artifacts, y)

                # 2. 【尺子测试】【完整回归】角度使用三次单击定点，模式确认不导出，未完成草稿可单独取消
                session.key(DEFAULTS["keys"]["tool_ruler"])
                choose_mode(process, artifacts, 1)
                vertex = (round(image.width * 0.58), round(image.height * 0.30))
                first = (vertex[0] + 170, vertex[1])
                second = (vertex[0], vertex[1] + 155)
                second_region = (vertex[0] - 80, vertex[1] - 90, first[0] + 100, second[1] + 110)
                before_angle = no_hover(process, artifacts / "before-angle.png", y)
                for point in (vertex, first, second):
                    click(point)
                angle = no_hover(process, artifacts / "angle.png", y)
                if not difference(before_angle, angle, second_region):
                    raise RuntimeError("three angle clicks did not create a measured angle")
                click((round(image.width * 0.7), round(image.height * 0.78)))
                session.control("move", "--logical", round(image.width * 0.84), round(image.height * 0.81))
                session.key("Escape")
                alive(process)
                if difference(angle, no_hover(process, artifacts / "angle-draft-canceled.png", y), second_region):
                    raise RuntimeError("canceling a pending angle changed the committed measurement")
                check_angle_edits(process, vertex, first, second, second_region, artifacts, y)

                # 3. 【尺子测试】【完整回归】源图保存和持久剪贴板包括测量，不包括临时十字线或取色提示
                session.key(DEFAULTS["keys"]["tool_ruler"])
                session.control("move", "--logical", round(image.width * 0.90), round(image.height * 0.82))
                time.sleep(1)
                capture(artifacts / "hover-before-export.png")
                before = session.resources(process.pid)
                time.sleep(1)
                after = session.resources(process.pid)
                if before["cpu_ticks"] != after["cpu_ticks"]:
                    raise RuntimeError("a stable ruler hover still schedules CPU work")
                (artifacts / "resources.json").write_text(json.dumps({"before": before, "after": after}, indent=2) + "\n")
                session.key("Return")
                process.wait(timeout=5)
                if process.returncode:
                    raise RuntimeError(f"ruler export failed: {process.returncode}")
                saved = list((temporary / "export").rglob("*.png"))
                if len(saved) != 1:
                    raise RuntimeError(f"expected one ruler PNG, received {len(saved)}")
                encoded = saved[0].read_bytes()
                (artifacts / "saved-image.png").write_bytes(encoded)
                clipboard = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment)
                (artifacts / "clipboard.png").write_bytes(clipboard)
                with Image.open(io.BytesIO(encoded)) as exported, Image.open(io.BytesIO(clipboard)) as copied:
                    exported = exported.convert("RGBA")
                    if exported.size != source.size or copied.convert("RGBA").tobytes() != exported.tobytes():
                        raise RuntimeError("ruler export did not retain original resolution or clipboard pixels")
                    for region in ((0, 0, 1600, 8), (20, 20, 43, 43), (1100, 790, 1550, 875)):
                        if exported.crop(region).tobytes() != source.crop(region).tobytes():
                            raise RuntimeError("ruler export altered source transparency or included transient hover pixels")
                    if ImageChops.difference(exported.convert("RGB"), source.convert("RGB")).getbbox() is None:
                        raise RuntimeError("ruler export omitted persistent measurements")
                saved_state = tomllib.loads((state / "tools.toml").read_text())
                if saved_state["ruler_mode"] != "angle" or saved_state["stroke_widths"]["ruler"] != 9:
                    raise RuntimeError(f"ruler mode or independent width did not persist: {saved_state}")
                shutil.copyfile(state / "tools.toml", artifacts / "tools.toml")
                if (temporary / "data/mark-shot/models").exists():
                    raise RuntimeError("ordinary ruler editing created the OCR model directory")
                (artifacts / "result.json").write_text(json.dumps({"modes": ["length", "angle"], "length_resize": True, "length_rotation": True, "angle_arm": True, "width": 9, "source_size": source.size, "persistent_clipboard": True, "hover_exported": False}, indent=2) + "\n")
                print("Wayland ruler mode, gestures, handles, undo, idle and export checks passed")
            finally:
                application_log = state / "mark-shot.log"
                if application_log.exists():
                    shutil.copyfile(application_log, artifacts / "application.log")
                if process.poll() is None:
                    session.key("Escape")
                    try:
                        process.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        process.terminate()
                        process.wait(timeout=3)


def main():
    """【尺子测试】【入口】解析构建产物与证据目录，执行回归；失败时返回非零退出码"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, default=Path("target/wayland-ruler"))
    options = parser.parse_args()
    run(options.binary.resolve(), options.artifacts.resolve())


if __name__ == "__main__":
    main()
