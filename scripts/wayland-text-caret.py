#!/usr/bin/env python3
"""【文字光标回归】【真实时序】只操作私有 Wayland，验证相位、颜色、字段与菜单暂停及局部协议损坏"""

import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time

from PIL import Image, ImageChops
import wayland_text_format_session as session
from wayland_text_pointer import pointer_context, click
from wayland_text_property_locator import font_button, font_popup


def capture(process, root, name):
    """【文字光标回归】【画面采集】保存 process 的实时 name 画面到 root，返回 RGB 图像；提前退出失败"""
    if process.poll() is not None:
        raise AssertionError(f"editor exited before caret checks: {process.returncode}")
    path = root / f"{name}.png"
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def sample(process, root, name, region, count=17):
    """【文字光标回归】【周期采样】在 root 记录 process 的 name 系列，region 限制正文区域，count 控制次数；返回实际时刻与像素图"""
    time.sleep(0.12)
    start = time.monotonic()
    frames = []
    for index in range(count):
        time.sleep(max(0, start + index * 0.125 - time.monotonic()))
        image = capture(process, root, f"{name}-{index}").crop(region)
        frames.append((time.monotonic() - start, image))
    return frames


def blink(frames):
    """【文字光标回归】【相位断言】检查 frames 只有两个稳定相位、差异为红色窄光标，返回时序与差异矩形；拒绝字形叠加"""
    phases = {}
    transitions = []
    previous = None
    for instant, image in frames:
        digest = hashlib.sha256(image.tobytes()).hexdigest()
        phases.setdefault(digest, image)
        if previous is not None and digest != previous:
            transitions.append(instant)
        previous = digest
    if len(phases) != 2:
        raise AssertionError(f"caret must have exactly two stable phases, received {len(phases)}")
    first, second = phases.values()
    bounds = ImageChops.difference(first, second).getbbox()
    if bounds is None or not 1 <= bounds[2] - bounds[0] <= 3 or not 10 <= bounds[3] - bounds[1] <= 64:
        raise AssertionError(f"blink changed pixels outside the insertion caret: {bounds}")
    y = (bounds[1] + bounds[3]) // 2
    colors = max(((first.getpixel((x, y)), second.getpixel((x, y))) for x in range(bounds[0], bounds[2])),
                 key=lambda pair: max(pixel[0] for pixel in pair))
    if not any(red > 100 and red > green * 1.7 and red > blue * 1.7 for red, green, blue in colors):
        raise AssertionError(f"annotation caret does not use its actual red foreground: {colors}")
    intervals = [end - start for start, end in zip(transitions, transitions[1:])]
    if len(transitions) < 3 or not all(0.30 <= interval <= 0.70 for interval in intervals):
        raise AssertionError(f"caret did not follow a sampled 500 ms half-period: {transitions}")
    return {"stable_phases": len(phases), "bounds": bounds, "colors": colors,
            "transitions_seconds": transitions, "half_periods_seconds": intervals}


def stationary(frames):
    """【文字光标回归】【停止断言】检查 frames 像素不再变化，返回样本数；字段或菜单期间的正文不得继续闪烁"""
    expected = frames[0][1]
    for _instant, actual in frames[1:]:
        difference = ImageChops.difference(expected, actual).getbbox()
        if difference:
            raise AssertionError(f"unfocused annotation continued changing pixels: {difference}")
    return len(frames)


def wait_font(process, root):
    """【文字光标回归】【字体就绪】等待 process 实际字体弹层出现，root 保留每次观察；返回真实几何，不用扩大尺寸容差替代就绪"""
    deadline = time.monotonic() + 3
    attempts = 0
    while True:
        attempts += 1
        try:
            return font_popup(capture(process, root, f"font-open-{attempts}"))
        except AssertionError:
            if time.monotonic() >= deadline:
                raise
            time.sleep(0.05)


def run(binary, root, overlay, focus_window):
    """【文字光标回归】【完整场景】用 binary 与 overlay 测量实际光标，root 保存证据；focus_window 加测私有 niri 失活，返回全部结果"""
    root.mkdir(parents=True, exist_ok=False)
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    (root / "binary.json").write_text(json.dumps({"sha256": fingerprint, "bytes": binary.stat().st_size}, indent=2) + "\n")
    environment = session.environment()
    with session.input_context(), pointer_context(), tempfile.TemporaryDirectory(prefix="mark-shot-caret-") as temporary:
        private = Path(temporary)
        config, state = private / "config/mark-shot", private / "state/mark-shot"
        config.mkdir(parents=True)
        state.mkdir(parents=True)
        (config / "config.toml").write_text(f'[general]\noverlay = "{overlay}"\n[history]\nenabled = false\n[notifications]\nenabled = false\n')
        (state / "tools.toml").write_text('magnifier = false\nfont_size = 20.0\nfont_size_unit = "pt"\ntext_family = "Inter"\nbold = false\nitalic = false\n')
        environment.update(XDG_CONFIG_HOME=str(private / "config"), XDG_STATE_HOME=str(private / "state"),
                           XDG_DATA_HOME=str(private / "data"), WAYLAND_DEBUG="client")
        source = private / "source.png"
        Image.new("RGB", (1600, 900), (30, 34, 38)).save(source)
        with (root / "process.log").open("w") as log:
            process = subprocess.Popen([str(binary), "--fullscreen", "--fullscreen-default-tool", "text",
                                        "--default-color", "#FF4D4D", "--to", "s", "--output", str(private / "export"), str(source)], env=environment, stderr=log)
            try:
                time.sleep(0.3)
                initial = capture(process, root, "initial")
                session.wait_single_thread(process.pid)
                point = round(initial.width * 0.62), round(initial.height * 0.52)
                region = (point[0] - 6, point[1] - 6, point[0] + 280, point[1] + 90)
                click(point)
                session.paste_text("alpha  ")
                # 1. 【文字光标回归】【静止编辑】无正文变化时只有窄光标相位，协议不得持续提交整屏
                result = {"typing": blink(sample(process, root, "typing", region))}
                session.key("ctrl+a")
                result["selection"] = blink(sample(process, root, "selection", region))
                session.key("End")
                opened = capture(process, root, "font-before")
                button = font_button(opened)
                click(button)
                wait_font(process, root)
                result["field_stationary_samples"] = stationary(sample(process, root, "field", region, 9))
                click(button)
                result["field_return"] = blink(sample(process, root, "field-return", region))
                # 2. 【文字光标回归】【菜单暂停】原生抓取期间正文静止；关闭后重新闪烁，不提交事务
                click((point[0] + 8, point[1] + 12), "right")
                result["menu_stationary_samples"] = stationary(sample(process, root, "menu", region, 9))
                session.key("Escape")
                result["menu_return"] = blink(sample(process, root, "menu-return", region))
                if focus_window:
                    if session.backend() is not session.niri or overlay != "window":
                        raise AssertionError("external focus checks require the private niri window backend")
                    from wayland_history_actions import window
                    editor_window = window("mark-shot", process.pid)
                    auxiliary = None
                    focus_protocol_start = len((root / "process.log").read_text())
                    try:
                        with (root / "auxiliary.log").open("w") as auxiliary_log:
                            auxiliary = subprocess.Popen([str(binary), "--history"], env=environment, stderr=auxiliary_log, stdout=subprocess.DEVNULL)
                        other = window("mark-shot-history", auxiliary.pid)
                        session.niri.control("focus", other["id"])
                        time.sleep(0.15)
                        focus_damage_before = (root / "process.log").read_text().count(".damage_buffer(")
                        result["external_focus_stationary_samples"] = stationary(sample(process, root, "focus-out", region, 9))
                        focus_protocol = (root / "process.log").read_text()
                        if focus_protocol.count(".damage_buffer(") != focus_damage_before or not re.search(r"wl_keyboard@\d+\.leave", focus_protocol[focus_protocol_start:]):
                            raise AssertionError("external focus did not stop caret redraw after an actual keyboard leave")
                        result["external_focus_has_no_redraw"] = True
                        session.niri.control("focus", editor_window["id"])
                        time.sleep(0.15)
                        result["external_focus_return"] = blink(sample(process, root, "focus-return", region))
                    finally:
                        if auxiliary and auxiliary.poll() is None:
                            auxiliary.terminate()
                            auxiliary.wait(timeout=3)
                session.key("ctrl+a")
                session.key("ctrl+c")
                session.wait_clipboard(environment, "alpha  ")
                session.key("ctrl+Return")
                time.sleep(0.15)
                committed = capture(process, root, "first-commit").crop(region)
                click((point[0] + 10, point[1] + 12))
                result["existing_document"] = blink(sample(process, root, "existing", region))
                session.key("ctrl+Return")
                # 3. 【文字光标回归】【事务结束】提交后彻底停止定时，CPU 与帧数稳定，输出不含光标
                time.sleep(0.2)
                before = session.resources(process.pid)
                start_protocol = (root / "process.log").read_text().count(".damage_buffer(")
                committed_frames = sample(process, root, "committed", region, 9)
                stationary(committed_frames)
                if ImageChops.difference(committed, committed_frames[0][1]).getbbox():
                    raise AssertionError("unchanged existing editing altered committed annotation pixels")
                after = session.resources(process.pid)
                end_protocol = (root / "process.log").read_text().count(".damage_buffer(")
                if before["cpu_ticks"] != after["cpu_ticks"] or after["threads"] != 1 or start_protocol != end_protocol:
                    raise AssertionError(f"committed text did not stop polling and redraw: {before}, {after}, {start_protocol}, {end_protocol}")
                session.key("ctrl+s")
                process.wait(timeout=6)
                exports = list((private / "export").glob("*.png"))
                if process.returncode or len(exports) != 1:
                    raise AssertionError(f"caret workflow did not export: {process.returncode}, {exports}")
                with Image.open(exports[0]) as exported:
                    if exported.size != (1600, 900):
                        raise AssertionError(f"caret workflow changed the original source dimensions: {exported.size}")
                shutil.copyfile(exports[0], root / "export.png")
                protocol = (root / "process.log").read_text()
                damages = [list(map(int, match)) for match in re.findall(r"wl_surface@\d+\.damage_buffer\((-?\d+), (-?\d+), (\d+), (\d+)\)", protocol)]
                narrow = [rect for rect in damages if rect[2] < 32 and 10 < rect[3] < 90]
                if len(narrow) < 8:
                    raise AssertionError(f"blink did not submit narrow damage buffers: {len(narrow)}")
                if focus_window and "wl_keyboard" in protocol and ".leave" not in protocol:
                    raise AssertionError("external focus did not deliver keyboard leave")
                if (private / "data/mark-shot/models").exists():
                    raise AssertionError("caret editing initialized OCR models")
                result.update(overlay=overlay, narrow_damage_frames=len(narrow), idle_before=before, idle_after=after,
                              no_commit_redraw=True, clipboard="alpha  ", export_size=[1600, 900], no_ocr=True)
                return {"sha256": fingerprint, "scenario": result}
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=3)


def main():
    """【文字光标回归】【入口】解析构建、证据与焦点选项，保存实际结果；失败保留日志及启动指纹，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--overlay", choices=("layer", "window"), default="layer")
    parser.add_argument("--focus-window", action="store_true")
    options = parser.parse_args()
    result = run(options.binary.resolve(), options.artifacts.resolve(), options.overlay, options.focus_window)
    (options.artifacts / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
