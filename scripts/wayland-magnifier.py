#!/usr/bin/env python3
"""【Wayland 测试】【双框放大镜】验证真实双框、原图取样、图层顺序、属性与导出，仅操作独立桌面"""

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

from wayland_editor_locator import magnifier_controls
import wayland_test_session as session


BORDER = (96, 113, 127)
DEFAULTS = tomllib.loads((Path(__file__).resolve().parents[1] / "src/config/default.toml").read_text())


def capture(path):
    """【放大镜测试】【界面截图】把独立桌面写入 path；返回可比较的 RGB 图像"""
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def alive(process):
    """【放大镜测试】【进程检查】确认 process 仍处于编辑会话；提前退出时抛出异常"""
    if process.poll() is not None:
        raise RuntimeError(f"the magnifier editor unexpectedly exited: {process.returncode}")


def ready(process, path):
    """【放大镜测试】【启动等待】等待 process 绘制源图，将证据写入 path；返回稳定界面，超时抛出异常"""
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        alive(process)
        image = capture(path)
        if image.getpixel((image.width // 2 - 110, image.height // 2)) == (30, 34, 38):
            session.wait_single_thread(process.pid)
            time.sleep(0.3)
            return capture(path)
        time.sleep(0.04)
    raise RuntimeError("the magnifier source image did not appear")


def no_hover(process, path, safe_y):
    """【放大镜测试】【悬停隔离】将 process 指针移出源图，safe_y 指定安全高度；返回稳定截图并写入 path"""
    session.control("move", "--logical", 5, safe_y)
    time.sleep(0.3)
    alive(process)
    return capture(path)


def click(point, count=1):
    """【放大镜测试】【单击动作】在独立桌面 point 单击 count 次；无返回值"""
    session.control("click", "--logical", *point, "--count", count)
    time.sleep(0.04)


def difference(first, second, region):
    """【放大镜测试】【区域差异】返回 first、second 在 region 内的非空差异矩形或空值"""
    return ImageChops.difference(first.crop(region), second.crop(region)).getbbox()


def geometry(environment, source):
    """【放大镜测试】【原图几何】根据 environment 的单输出和 source 计算视口、双框与旋转把手；返回逻辑坐标，不使用截图取整尺寸"""
    outputs = json.loads(subprocess.check_output(["niri", "msg", "-j", "outputs"], env=environment))
    if len(outputs) != 1:
        raise RuntimeError("the magnifier regression requires one unambiguous output")
    logical = next(iter(outputs.values()))["logical"]
    width, height = logical["width"], logical["height"]
    scale = min((width - 64) / source.width, (height - 96) / source.height, 1)
    viewport = ((width - source.width * scale) / 2, (height - source.height * scale) / 2, scale)
    source_point = (round(width * 0.24), round(height * 0.48))
    lens_point = (round(width * 0.58), round(height * 0.52))
    diameter = 320 * scale
    source_radius = diameter / 2 / 2.75
    left = min(source_point[0] - source_radius, lens_point[0] - diameter / 2)
    right = max(source_point[0] + source_radius, lens_point[0] + diameter / 2)
    top = min(source_point[1] - source_radius, lens_point[1] - diameter / 2)
    bottom = max(source_point[1] + source_radius, lens_point[1] + diameter / 2)
    pivot = ((left + right) / 2, (top + bottom) / 2)
    handle = (round(pivot[0]), round(top - 32))
    angle = math.radians(34)
    delta = (handle[0] - pivot[0], handle[1] - pivot[1])
    rotated_handle = (round(pivot[0] + delta[0] * math.cos(angle) - delta[1] * math.sin(angle)), round(pivot[1] + delta[0] * math.sin(angle) + delta[1] * math.cos(angle)))
    return {"source": source_point, "lens": lens_point, "diameter": diameter, "viewport": viewport, "handle": handle, "rotated_handle": rotated_handle, "safe_y": height - 6, "region": (50, math.ceil(viewport[1]) + 90, width - 50, height - 50)}


def check_drag(process, artifacts, name, start, end, layout, unchanged=None):
    """【放大镜测试】【双框编辑】在 process 中从 start 拖动到 end；layout 限定比较区域，unchanged 可指定不变区域，artifacts 保存撤销与重做证据"""
    before = no_hover(process, artifacts / f"{name}-before.png", layout["safe_y"])
    session.control("drag", "--from", *start, "--to", *end, "--steps", 10)
    after = no_hover(process, artifacts / f"{name}-changed.png", layout["safe_y"])
    if not difference(before, after, layout["region"]):
        raise RuntimeError(f"{name} did not change the selected magnifier")
    if unchanged and difference(before, after, unchanged):
        raise RuntimeError(f"{name} changed the other frame's image")
    session.key("ctrl+z")
    restored = no_hover(process, artifacts / f"{name}-undo.png", layout["safe_y"])
    if difference(before, restored, layout["region"]):
        raise RuntimeError(f"{name} did not undo in one transaction")
    session.key("ctrl+shift+z")
    redone = no_hover(process, artifacts / f"{name}-redo.png", layout["safe_y"])
    if difference(after, redone, layout["region"]):
        raise RuntimeError(f"{name} did not redo in one transaction")
    session.key("ctrl+z")


def edit_field(process, artifacts, name, value, layout):
    """【放大镜测试】【字段编辑】在 process 中粘贴 name 字段的 value 并确认；layout 提供区域，artifacts 保存单次撤销与重做证据"""
    before = no_hover(process, artifacts / f"{name}-before.png", layout["safe_y"])
    controls = magnifier_controls(before, BORDER)
    click(controls[name], 2)
    session.key("ctrl+a")
    session.paste_text(value)
    session.key("Return")
    alive(process)
    after = no_hover(process, artifacts / f"{name}-changed.png", layout["safe_y"])
    if not difference(before, after, layout["region"]):
        raise RuntimeError(f"the {name} field did not update the selected annotation")
    session.key("ctrl+z")
    restored = no_hover(process, artifacts / f"{name}-undo.png", layout["safe_y"])
    if difference(before, restored, layout["region"]):
        raise RuntimeError(f"the {name} field did not undo in one transaction")
    session.key("ctrl+shift+z")
    redone = no_hover(process, artifacts / f"{name}-redo.png", layout["safe_y"])
    if difference(after, redone, layout["region"]):
        raise RuntimeError(f"the {name} field did not redo correctly")


def toggle_shape(process, artifacts, layout):
    """【放大镜测试】【框形修改】在 process 中把已选圆形变成矩形并验证撤销；layout 提供比较范围，artifacts 保存证据"""
    before = no_hover(process, artifacts / "shape-before.png", layout["safe_y"])
    click(magnifier_controls(before, BORDER)["shape"])
    after = no_hover(process, artifacts / "shape-rectangle.png", layout["safe_y"])
    if not difference(before, after, layout["region"]):
        raise RuntimeError("the rectangle toggle did not change the circular lens")
    radius = layout["diameter"] / 2
    corner = (round(layout["lens"][0] - radius + 15), round(layout["lens"][1] - radius + 15))
    pixel = after.getpixel(corner)
    if pixel[1] <= pixel[2]:
        raise RuntimeError(f"the rectangular lens corner did not include the green source: {pixel}")
    session.key("ctrl+z")
    restored = no_hover(process, artifacts / "shape-undo.png", layout["safe_y"])
    if difference(before, restored, layout["region"]):
        raise RuntimeError("the shape toggle did not undo cleanly")
    session.key("ctrl+shift+z")


def held_pointer(start):
    """【放大镜测试】【保持按钮】在独立桌面 start 保持左键六秒；返回可等待的输入进程，不操作真实桌面"""
    return subprocess.Popen(["niri-use", "--child", "click", "--logical", *map(str, start), "--hold", "6"], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)


def check_stack(process, artifacts, layout):
    """【放大镜测试】【暂存图层】在 process 中新增较新矩形并保持镜框拖动；layout 提供坐标，artifacts 验证临时镜框不能覆盖新标注"""
    center = layout["lens"]
    session.key(DEFAULTS["keys"]["tool_rectangle"])
    session.control("drag", "--from", center[0] - 14, center[1] - 10, "--to", center[0] + 14, center[1] + 10, "--steps", 6)
    session.key(DEFAULTS["keys"]["tool_pick"])
    click(layout["source"])
    before = no_hover(process, artifacts / "stack-before.png", layout["safe_y"])
    expected = before.getpixel(center)
    if expected[0] < 200 or expected[1] > 100:
        raise RuntimeError(f"the newer red annotation did not cover the lens: {expected}")
    start = (center[0] - 55, center[1])
    with held_pointer(start) as gesture:
        time.sleep(0.2)
        session.control("move", "--logical", start[0] + 35, start[1] + 15)
        active = capture(artifacts / "stack-drag-active.png")
        if gesture.poll() is not None:
            raise RuntimeError("the stack test released before observing the pending lens")
        if active.getpixel(center) != expected:
            raise RuntimeError(f"a pending magnifier covered the newer annotation: {expected} -> {active.getpixel(center)}")
        gesture.wait(timeout=8)
        if gesture.returncode:
            raise RuntimeError(gesture.stderr.read().decode())
    alive(process)
    session.key("ctrl+z")
    restored = no_hover(process, artifacts / "stack-drag-undo.png", layout["safe_y"])
    if difference(before, restored, layout["region"]):
        raise RuntimeError("the held lens drag did not undo to its original layer order")


def check_cancel(process, artifacts, layout):
    """【放大镜测试】【草稿取消】在 process 中保持新双框手势并用 Escape 取消；layout 提供区域，artifacts 记录无残影结果"""
    session.key(DEFAULTS["keys"]["tool_magnifier"])
    before = no_hover(process, artifacts / "cancel-before.png", layout["safe_y"])
    start = (round(before.width * 0.85), round(before.height * 0.25))
    end = (round(before.width * 0.85), round(before.height * 0.75))
    with held_pointer(start) as gesture:
        time.sleep(0.2)
        session.control("move", "--logical", *end)
        active = capture(artifacts / "cancel-draft.png")
        if gesture.poll() is not None or not difference(before, active, layout["region"]):
            raise RuntimeError("the pending magnifier was not observable while the button remained down")
        session.key("Escape")
        alive(process)
        gesture.wait(timeout=8)
        if gesture.returncode:
            raise RuntimeError(gesture.stderr.read().decode())
    after = no_hover(process, artifacts / "cancel-restored.png", layout["safe_y"])
    if difference(before, after, layout["region"]):
        raise RuntimeError("canceling a draft altered committed annotations or left sampled pixels")


def verify_export(encoded, clipboard, source, layout):
    """【放大镜测试】【原图输出】验证 encoded、clipboard 保留 source 原始尺寸和透明度；layout 映射镜框中心，返回输出 RGBA 图像"""
    with Image.open(io.BytesIO(encoded)) as image, Image.open(io.BytesIO(clipboard)) as copied:
        output = image.convert("RGBA")
        if output.size != source.size or copied.convert("RGBA").tobytes() != output.tobytes():
            raise RuntimeError("the saved and copied magnifier images differ or lost original resolution")
    for region in ((0, 0, 1600, 8), (20, 20, 43, 43), (1100, 790, 1550, 875)):
        if output.crop(region).tobytes() != source.crop(region).tobytes():
            raise RuntimeError("magnifier export altered untouched original pixels or transparency")
    left, top, scale = layout["viewport"]
    point = (round((layout["lens"][0] - 45 - left) / scale), round((layout["lens"][1] - top) / scale))
    pixel = output.getpixel(point)
    if pixel[1] <= pixel[0] or pixel[1] <= pixel[2]:
        raise RuntimeError(f"native export did not include the frozen green source in the lens: {pixel}")
    return output


def run(binary, artifacts):
    """【放大镜测试】【完整回归】用 binary 验证真实手势、框形、原图、撤销与导出；artifacts 保存证据，普通流程不初始化识别模型"""
    environment = session.environment()
    artifacts.mkdir(parents=True, exist_ok=True)
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    with tempfile.TemporaryDirectory(prefix="mark-shot-magnifier-test-") as directory:
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
        painter.rectangle((250, 350, 520, 610), fill=(20, 180, 110, 200))
        painter.rectangle((780, 300, 1150, 700), fill=(22, 80, 180, 255))
        source_path = temporary / "source.png"
        source.save(source_path)
        layout = geometry(environment, source)
        with (artifacts / "process.log").open("w") as log:
            process = subprocess.Popen([str(binary), "--fullscreen", "--fullscreen-default-tool", "magnifier", "--default-color", "#FF3B30", "--to", "cs", "--output", str(temporary / "export"), str(source_path)], env=environment, stderr=log)
            try:
                ready(process, artifacts / "baseline.png")
                start, center = layout["source"], layout["lens"]
                # 1. 【放大镜测试】【完整回归】旧标注覆盖取样位置，镜框仍只能读取冻结原图
                session.key(DEFAULTS["keys"]["tool_rectangle"])
                session.control("drag", "--from", start[0] - 18, start[1] - 16, "--to", start[0] + 18, start[1] + 16, "--steps", 6)
                session.key(DEFAULTS["keys"]["tool_magnifier"])
                before = no_hover(process, artifacts / "before-magnifier.png", layout["safe_y"])
                click(start)
                session.control("drag", "--from", *start, "--to", start[0] + 20, start[1] + 10, "--steps", 4)
                empty = no_hover(process, artifacts / "empty-gestures.png", layout["safe_y"])
                if difference(before, empty, layout["region"]):
                    raise RuntimeError("an empty or short magnifier gesture created persistent pixels")
                session.control("drag", "--from", *start, "--to", *center, "--steps", 12)
                drawn = no_hover(process, artifacts / "circle.png", layout["safe_y"])
                sampled = drawn.getpixel(center)
                if sampled[1] <= sampled[0] or sampled[1] <= sampled[2]:
                    raise RuntimeError(f"the lens sampled marked or preview pixels instead of the green original: {sampled}")
                session.key(DEFAULTS["keys"]["tool_pick"])
                click(start)
                # 2. 【放大镜测试】【完整回归】两框移动和两组控制点缩放均经过真实选择与单次历史提交
                unchanged = (center[0] - 24, center[1] - 24, center[0] + 24, center[1] + 24)
                check_drag(process, artifacts, "source-move", start, (start[0] + 40, start[1]), layout, unchanged)
                check_drag(process, artifacts, "lens-move", center, (center[0] + 45, center[1] + 25), layout)
                lens_right = (round(center[0] + layout["diameter"] / 2), center[1])
                source_right = (round(start[0] + layout["diameter"] / 2 / 2.75), start[1])
                check_drag(process, artifacts, "lens-resize", lens_right, (lens_right[0] + 40, lens_right[1]), layout)
                check_drag(process, artifacts, "source-resize", source_right, (source_right[0] + 20, source_right[1]), layout)
                check_drag(process, artifacts, "rotation", layout["handle"], layout["rotated_handle"], layout)
                # 3. 【放大镜测试】【完整回归】属性修改、暂存层序和草稿取消不污染较新的标注或冻结原图
                toggle_shape(process, artifacts, layout)
                edit_field(process, artifacts, "zoom", "4.25", layout)
                edit_field(process, artifacts, "width", "9", layout)
                check_stack(process, artifacts, layout)
                check_cancel(process, artifacts, layout)
                session.wait_single_thread(process.pid)
                time.sleep(0.2)
                before = session.resources(process.pid)
                time.sleep(1)
                after = session.resources(process.pid)
                if after["threads"] != 1 or before["cpu_ticks"] != after["cpu_ticks"]:
                    raise RuntimeError(f"a stable magnifier editor keeps polling: {before} -> {after}")
                (artifacts / "resources.json").write_text(json.dumps({"before": before, "after": after}, indent=2) + "\n")
                session.key("Return")
                process.wait(timeout=5)
                if process.returncode:
                    raise RuntimeError(f"magnifier export failed: {process.returncode}")
                files = list((temporary / "export").rglob("*.png"))
                if len(files) != 1:
                    raise RuntimeError(f"expected one saved magnifier image, received {len(files)}")
                encoded = files[0].read_bytes()
                clipboard = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment)
                (artifacts / "saved-image.png").write_bytes(encoded)
                (artifacts / "clipboard.png").write_bytes(clipboard)
                verify_export(encoded, clipboard, source, layout)
                saved = tomllib.loads((state / "tools.toml").read_text())
                if saved["magnifier_shape"] != "rectangle" or saved["magnifier_zoom"] != 4.25 or saved["stroke_widths"]["magnifier"] != 9 or saved["magnifier"]:
                    raise RuntimeError(f"magnifier defaults did not persist independently: {saved}")
                shutil.copyfile(state / "tools.toml", artifacts / "tools.toml")
                if (temporary / "data/mark-shot/models").exists():
                    raise RuntimeError("ordinary magnifier editing initialized the OCR model directory")
                if fingerprint != hashlib.sha256(binary.read_bytes()).hexdigest():
                    raise RuntimeError("the executable changed during the magnifier regression")
                (artifacts / "result.json").write_text(json.dumps({"sha256": fingerprint, "shapes": ["circle", "rectangle"], "zoom": 4.25, "width": 9, "source_size": source.size, "source_sampling": "frozen original", "source_move": True, "lens_move": True, "linked_resize": True, "rotation": True, "pending_z_order": True, "draft_cancellation": True, "persistent_clipboard": True}, indent=2) + "\n")
                print("Wayland magnifier dual frames, frozen sampling, gestures, properties, layer order, idle and export checks passed")
            finally:
                application_log = state / "mark-shot.log"
                if application_log.exists():
                    shutil.copyfile(application_log, artifacts / "application.log")
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=3)


def main():
    """【放大镜测试】【入口】解析二进制与证据目录并执行回归；失败时返回非零退出码"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, default=Path("target/wayland-magnifier"))
    options = parser.parse_args()
    run(options.binary.resolve(), options.artifacts.resolve())


if __name__ == "__main__":
    main()
