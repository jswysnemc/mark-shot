#!/usr/bin/env python3
"""【Wayland 测试】【标记回归】验证形状选择、已有标注、取消绘图和原图导出，仅操作隔离桌面"""

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

from wayland_editor_locator import marker_controls
import wayland_test_session as session


DEFAULTS = tomllib.loads((Path(__file__).resolve().parents[1] / "src/config/default.toml").read_text())
BORDER = (96, 113, 127)
SHAPES = ("triangle", "star", "check", "cross", "diamond", "heart", "spade", "club", "circle", "square", "pentagon", "hexagon", "octagon", "plus", "arrow-up", "lightning", "ban", "crescent", "pin", "flag", "bookmark", "shield", "exclamation", "speech-bubble")


def capture(path):
    """【标记测试】【界面截图】将隔离桌面保存到 path；返回 RGB 像素图像"""
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def difference(first, second, region):
    """【标记测试】【区域比较】返回 first、second 在 region 内的非空像素差异矩形或空值"""
    return ImageChops.difference(first.crop(region), second.crop(region)).getbbox()


def click(position, count=1):
    """【标记测试】【点击动作】在隔离桌面点击 position 共 count 次；等待事件进入编辑器，无返回值"""
    session.control("click", "--logical", *position, "--count", count)
    time.sleep(0.05)


def alive(process):
    """【标记测试】【进程检查】确认 process 仍在编辑；提前退出时抛出异常"""
    if process.poll() is not None:
        raise RuntimeError(f"an editor interaction unexpectedly exited with status {process.returncode}")


def wait_ready(process, output):
    """【标记测试】【启动等待】等待 process 显示测试源图并写入 output；返回稳定界面或抛出异常"""
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        alive(process)
        image = capture(output)
        if image.getpixel((image.width // 2, image.height // 2)) == (30, 34, 38):
            time.sleep(0.3)
            return capture(output)
        time.sleep(0.04)
    raise RuntimeError("the test image did not appear")


def choose(process, controls, index, previous=None):
    """【标记测试】【形状确认】用 controls 打开 process 的面板并确认 index；previous 为上次确认索引，Enter 不导出图片"""
    click(controls["shape"])
    time.sleep(0.2)
    if previous is not None and index == previous + 1:
        session.control("key", "Right", "Return")
    else:
        session.control("key", "Home", *("Right" for _ in range(index)), "Return")
    time.sleep(0.25)
    alive(process)


def check_popover(process, baseline, controls, region, artifacts):
    """【标记测试】【弹层交互】检查 process 的 controls 可打开、Escape 关闭和再次点击关闭；region、artifacts 用于验证和留证"""
    click(controls["shape"])
    time.sleep(0.3)
    visible = capture(artifacts / "shape-choices.png")
    changed = ImageChops.difference(baseline, visible).getbbox()
    if changed is None or changed[3] - changed[1] < 100:
        raise RuntimeError("the shape grid did not visibly open")
    before = session.resources(process.pid)
    time.sleep(1)
    after = session.resources(process.pid)
    if before["cpu_ticks"] != after["cpu_ticks"]:
        raise RuntimeError("a stable open shape grid still schedules CPU work")
    (artifacts / "popover-resources.json").write_text(json.dumps({"before": before, "after": after}, indent=2) + "\n")
    session.key("Escape")
    alive(process)
    time.sleep(0.3)
    if difference(baseline, capture(artifacts / "popover-escape.png"), region):
        raise RuntimeError("closing the shape grid left an unrefreshed area")
    click(controls["shape"])
    time.sleep(0.3)
    click(controls["shape"])
    time.sleep(0.3)
    if difference(baseline, capture(artifacts / "popover-toggle-close.png"), region):
        raise RuntimeError("clicking the open shape toggle reopened the grid")


def cells(image, source):
    """【标记测试】【绘图网格】按 image 的实际大小与 source 原始尺寸计算安全绘图位置；返回视口与 24 个单元"""
    scale = min((image.width - 64) / source.width, (image.height - 96) / source.height, 1)
    width, height = source.width * scale, source.height * scale
    left, top = (image.width - width) / 2, (image.height - height) / 2
    result = []
    for index in range(24):
        x = left + width * (0.14 + index % 6 * 0.125)
        y = top + height * (0.17 + index // 6 * 0.18)
        start = (round(x), round(y))
        end = (round(x + width * 0.065), round(y + height * 0.11))
        result.append((start, end, (start[0] - 12, start[1] - 12, end[0] + 12, end[1] + 12)))
    return (left, top, scale), result


def check_marker_gestures(process, environment, baseline, viewport, grid, region, artifacts):
    """【标记测试】【手势约束】在 process 中验证空手势、源图外点击和 Ctrl 等比例；environment、viewport、grid、region 提供隔离坐标"""
    start, end, _ = grid[0]
    click(start)
    session.control("drag", "--from", *start, "--to", start[0] + 1, start[1], "--steps", 4)
    click((max(0, round(viewport[0] - 12)), start[1] + 100))
    if difference(baseline, capture(artifacts / "empty-gestures.png"), region):
        raise RuntimeError("an empty gesture or outside-source click created a marker")

    # 1. 独立虚拟键盘保持 Ctrl，指针仍限定到已验证的隔离会话
    side = min(end[0] - start[0], end[1] - start[1])
    with subprocess.Popen(["wtype", "-M", "ctrl", "-s", "10000", "-m", "ctrl"], env=environment, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE) as modifier:
        time.sleep(0.15)
        session.control("drag", "--from", *start, "--to", start[0] + side, start[1] + round(side / 3), "--steps", 8)
        modifier.wait(timeout=15)
        if modifier.returncode:
            raise RuntimeError(modifier.stderr.read().decode())
    alive(process)
    time.sleep(0.2)
    square = capture(artifacts / "ctrl-square.png")
    changed = difference(baseline, square, region)
    if changed is None or abs((changed[2] - changed[0]) - (changed[3] - changed[1])) > 5:
        raise RuntimeError("Ctrl did not constrain marker drawing to a square")

    # 2. 约束手势只记录一次标注，撤销后恢复干净源图
    session.key("ctrl+z")
    time.sleep(0.2)
    if difference(baseline, capture(artifacts / "ctrl-square-undo.png"), region):
        raise RuntimeError("one undo did not remove the constrained marker")


def cancel_pen(process, baseline, region, artifacts):
    """【标记测试】【缓存取消】在 process 的长画笔手势中切换标记；验证 region 恢复 baseline 并写入 artifacts"""
    start = (region[0] + 20, region[1] + 20)
    end = (region[2] - 20, region[3] - 20)
    arguments = ["niri-use", "--child", "click", "--logical", *map(str, start), "--hold", "30"]
    with subprocess.Popen(arguments, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE) as gesture:
        time.sleep(0.2)
        for step in range(1, 4):
            point = (round(start[0] + (end[0] - start[0]) * step / 3), round(start[1] + (end[1] - start[1]) * step / 3))
            session.control("move", "--logical", *point)
        draft = capture(artifacts / "pen-draft.png")
        if gesture.poll() is not None or not difference(baseline, draft, region):
            raise RuntimeError("the held pen draft was not observable before release")
        session.key(DEFAULTS["keys"]["tool_marker"])
        if gesture.poll() is not None:
            raise RuntimeError("the test button released before tool switching completed")
        gesture.wait(timeout=40)
        if gesture.returncode:
            raise RuntimeError(gesture.stderr.read().decode())
    alive(process)
    time.sleep(0.25)
    canceled = capture(artifacts / "pen-canceled.png")
    if difference(baseline, canceled, region):
        raise RuntimeError("canceling the baked pen changed committed markers or left a draft")


def edit_marker(process, artifacts, cell):
    """【标记测试】【已有标注】选择 cell 标记并修改形状、填充及笔宽；验证撤销与重做，保存界面到 artifacts"""
    session.key(DEFAULTS["keys"]["tool_pick"])
    start, end, region = cell
    click((round((start[0] + end[0]) / 2), round((start[1] + end[1]) / 2)))
    selected = capture(artifacts / "selected.png")
    controls = marker_controls(selected, BORDER)
    choose(process, controls, 9)
    modified = capture(artifacts / "shape-modified.png")
    if not difference(selected, modified, region):
        raise RuntimeError("choosing a new shape did not update the selected marker")
    session.key("ctrl+z")
    time.sleep(0.2)
    restored = capture(artifacts / "shape-undo.png")
    if difference(selected, restored, region):
        raise RuntimeError("undo did not restore the selected marker shape")
    session.key("ctrl+shift+z")
    time.sleep(0.2)
    redone = capture(artifacts / "shape-redo.png")
    if difference(modified, redone, region):
        raise RuntimeError("redo did not restore the modified marker shape")

    # 1. 属性改变属于同一标注，空心中部恢复原图颜色
    controls = marker_controls(redone, BORDER)
    click(controls["fill"])
    hollow = capture(artifacts / "fill-disabled.png")
    center = (round((start[0] + end[0]) / 2), round((start[1] + end[1]) / 2))
    if hollow.getpixel(center) != (30, 34, 38):
        raise RuntimeError("disabling marker fill did not expose the source image")
    session.key("ctrl+z")
    time.sleep(0.1)
    if capture(artifacts / "fill-undo.png").getpixel(center) == (30, 34, 38):
        raise RuntimeError("fill undo did not restore the marker interior")
    session.key("ctrl+shift+z")
    time.sleep(0.1)

    # 2. 笔宽字段确认只提交属性，不触发全局导出
    controls = marker_controls(capture(artifacts / "before-width.png"), BORDER)
    click(controls["width"], 2)
    session.key("ctrl+a")
    session.paste_text("9")
    session.key("Return")
    alive(process)
    capture(artifacts / "width-modified.png")


def check_export(encoded, source, viewport, grid):
    """【标记测试】【输出像素】检查 encoded 保留 source 的分辨率、透明边缘及 24 项标记；viewport、grid 提供坐标映射"""
    with Image.open(io.BytesIO(encoded)) as image:
        output = image.convert("RGBA")
    if output.size != source.size:
        raise RuntimeError("export downsampled the original image")
    for region in ((0, 0, source.width, 8), (20, 20, 43, 43)):
        if output.crop(region).tobytes() != source.crop(region).tobytes():
            raise RuntimeError("export changed untouched transparent source pixels")
    left, top, scale = viewport
    for index, (start, end, _) in enumerate(grid):
        region = (max(0, round((start[0] - left - 10) / scale)), max(0, round((start[1] - top - 10) / scale)), min(source.width, round((end[0] - left + 10) / scale)), min(source.height, round((end[1] - top + 10) / scale)))
        changed = ImageChops.difference(output.convert("RGB").crop(region), source.convert("RGB").crop(region)).getbbox()
        if changed is None:
            raise RuntimeError(f"export omitted marker {index}: {SHAPES[index]}")
    start, end, _ = grid[-1]
    center = (round(((start[0] + end[0]) / 2 - left) / scale), round(((start[1] + end[1]) / 2 - top) / scale))
    if output.getpixel(center) != source.getpixel(center):
        raise RuntimeError("export did not preserve the edited hollow marker")
    return output


def run(binary, artifacts):
    """【标记测试】【完整回归】使用 binary 验证原生标记工作流；将图片、日志、配置和资源证据写入 artifacts"""
    environment = session.environment()
    artifacts.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="mark-shot-marker-test-") as directory:
        temporary = Path(directory)
        config = temporary / "config/mark-shot"
        state = temporary / "state/mark-shot"
        config.mkdir(parents=True)
        state.mkdir(parents=True)
        (config / "config.toml").write_text('[general]\nto = "cs"\n[notifications]\nenabled = false\n[theme]\nborder = "#60717FFF"\nborder_on_light = "#60717FFF"\n')
        (state / "tools.toml").write_text("magnifier = false\n")
        environment.update(XDG_CONFIG_HOME=str(temporary / "config"), XDG_DATA_HOME=str(temporary / "data"), XDG_STATE_HOME=str(temporary / "state"))
        source = Image.new("RGBA", (1600, 900), (30, 34, 38, 255))
        painter = ImageDraw.Draw(source)
        painter.rectangle((0, 0, 1599, 7), fill=(0, 0, 0, 0))
        painter.rectangle((20, 20, 42, 42), fill=(255, 0, 0, 128))
        source_path = temporary / "source.png"
        source.save(source_path)
        export_directory = temporary / "export"
        with (artifacts / "process.log").open("w") as stderr:
            process = subprocess.Popen([str(binary), "--fullscreen", "--fullscreen-default-tool", "marker", "--default-color", "#FF3B30", "--to", "cs", "--output", str(export_directory), str(source_path)], env=environment, stderr=stderr)
            try:
                baseline = wait_ready(process, artifacts / "baseline.png")
                controls = marker_controls(baseline, BORDER)
                viewport, grid = cells(baseline, source)
                canvas_region = (grid[0][2][0], grid[0][2][1], grid[-1][2][2], grid[-1][2][3])
                check_popover(process, baseline, controls, canvas_region, artifacts)
                check_marker_gestures(process, environment, baseline, viewport, grid, canvas_region, artifacts)

                # 1. 选择弹层确认全部目录项，Enter 只选择，已完成标记持续保留
                for index, (start, end, region) in enumerate(grid):
                    choose(process, controls, index, index - 1 if index else None)
                    before = capture(artifacts / "before-marker.png")
                    session.control("drag", "--from", *start, "--to", *end, "--steps", 12)
                    time.sleep(0.2)
                    after = capture(artifacts / f"marker-{index:02}-{SHAPES[index]}.png")
                    if not difference(before, after, region):
                        raise RuntimeError(f"drawing marker {index} produced no visible shape")
                    print(f"Marker {index + 1}/24 verified", flush=True)
                edit_marker(process, artifacts, grid[-1])

                # 2. 缓存画笔取消不删除已有标记，关闭弹层后事件循环停止定时刷新
                session.key(DEFAULTS["keys"]["tool_pen"])
                time.sleep(0.3)
                before_cancel = capture(artifacts / "before-pen-cancel.png")
                cancel_pen(process, before_cancel, canvas_region, artifacts)
                before = session.resources(process.pid)
                time.sleep(1)
                after = session.resources(process.pid)
                if before["cpu_ticks"] != after["cpu_ticks"]:
                    raise RuntimeError(f"the stable editor still consumes CPU ticks: {before} -> {after}")
                (artifacts / "resources.json").write_text(json.dumps({"before": before, "after": after}, indent=2) + "\n")

                # 3. 原始尺寸图片保存与持久 PNG 剪贴板一致，普通标注不创建模型目录
                session.key("Return")
                process.wait(timeout=5)
                if process.returncode:
                    raise RuntimeError(f"export failed with status {process.returncode}")
                saved_images = list(export_directory.rglob("*.png"))
                if len(saved_images) != 1:
                    raise RuntimeError(f"expected one saved image, received {len(saved_images)}")
                saved = saved_images[0].read_bytes()
                (artifacts / "saved-image.png").write_bytes(saved)
                output = check_export(saved, source, viewport, grid)
                clipboard = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment)
                (artifacts / "clipboard.png").write_bytes(clipboard)
                with Image.open(io.BytesIO(clipboard)) as image:
                    if image.convert("RGBA").tobytes() != output.tobytes() or image.size != output.size:
                        raise RuntimeError("saved image and persistent image clipboard differ")
                saved_state = tomllib.loads((state / "tools.toml").read_text())
                if saved_state["marker_shape"] != "square" or saved_state["marker_filled"] or saved_state["stroke_widths"]["marker"] != 9:
                    raise RuntimeError(f"marker defaults did not persist: {saved_state}")
                shutil.copyfile(state / "tools.toml", artifacts / "tools.toml")
                if (temporary / "data/mark-shot/models").exists():
                    raise RuntimeError("ordinary marker editing created the OCR model directory")
                (artifacts / "result.json").write_text(json.dumps({"shapes": list(SHAPES), "edited_shape": "square", "filled": False, "width": 9, "source_size": source.size, "export_size": output.size}, indent=2) + "\n")
                print("Wayland marker selection, editing, cancellation, idle and export checks passed")
            finally:
                log = state / "mark-shot.log"
                if log.exists():
                    shutil.copyfile(log, artifacts / "application.log")
                if process.poll() is None:
                    session.key("Escape")
                    try:
                        process.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        process.terminate()
                        process.wait(timeout=3)


def main():
    """【标记测试】【入口】解析构建产物与证据目录，执行回归；失败时返回非零退出码"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, default=Path("target/wayland-marker"))
    options = parser.parse_args()
    run(options.binary.resolve(), options.artifacts.resolve())


if __name__ == "__main__":
    main()
