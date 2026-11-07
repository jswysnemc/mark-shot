#!/usr/bin/env python3
"""【主题回归】【真实窗口】在已校验私有 niri 验证明暗切换、缓存恢复、重开与原图导出"""

import argparse
from contextlib import contextmanager
import hashlib
import io
import json
import os
from pathlib import Path
import select
import subprocess
import time
import tomllib

from PIL import Image, ImageChops
import wayland_test_session as session
from wayland_window_input_session import frame as ocr_frame
from wayland_ocr_popup_tree import active_tree
from wayland_theme_transition import toggle as wait_theme


def screenshot(root, name):
    """【主题回归】【实际截图】将私有画面保存到 root/name，返回 RGB 像素；不包含指针图像"""
    path = root / f"{name}.png"
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def wait_window(process, app_id):
    """【主题回归】【窗口定位】等待 process 对应唯一 app_id，返回真实窗口；进程退出或超时报错"""
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise AssertionError(f"process exited before mapping: {process.returncode}")
        windows = [window for window in session.windows()
                   if window["app_id"] == app_id and window["pid"] == process.pid]
        if len(windows) == 1:
            session.control("focus", windows[0]["id"])
            session.wait_single_thread(process.pid)
            time.sleep(0.4)
            return windows[0]
        if len(windows) > 1:
            raise AssertionError(f"ambiguous private window: {app_id}")
        time.sleep(0.03)
    raise AssertionError(f"private window not mapped: {app_id}")


@contextmanager
def launch(binary, environment, root, name, arguments, payload=None):
    """【主题回归】【进程范围】用 arguments 启动 binary，按需发送 payload；返回进程，退出时只回收本次句柄"""
    with (root / f"{name}.log").open("w") as log:
        process = subprocess.Popen([str(binary), *map(str, arguments)],
            env=dict(environment, WAYLAND_DEBUG="client"),
            stdin=subprocess.PIPE if payload is not None else subprocess.DEVNULL,
            stdout=subprocess.PIPE, stderr=log)
        try:
            if payload is not None:
                process.stdin.write(payload)
                process.stdin.close()
                if not select.select([process.stdout], [], [], 8)[0] or process.stdout.readline() != b"\n":
                    raise AssertionError("OCR result did not confirm its first frame")
            yield process
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=3)
            process.stdout.close()


def toggle(config, expected):
    """【主题回归】【真实快捷键】发送统一切换键，确认 config 已原子保存 expected；随后等待实际绘制，无返回值"""
    wait_theme(config, expected, session.key, session.screenshot)
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        values = tomllib.loads(config.read_text())
        if values.get("ui", {}).get("theme") == expected:
            assert values["custom"]["sentinel"] == "retained"
            assert values["history"]["limit"] == 23
            time.sleep(0.3)
            return
        time.sleep(0.03)
    raise AssertionError(f"theme shortcut did not persist {expected}")



def pixels(image):
    """【主题回归】【像素遍历】按 image 返回扁平像素序列，兼容当前及旧版 Pillow，无重复编码"""
    method = getattr(image, "get_flattened_data", None)
    return method() if method is not None else image.getdata()

def changed(first, second, minimum, darker):
    """【主题回归】【明暗呈现】比较 first、second 中实际亮度变化，darker 选择方向；返回覆盖像素数，必须超过 minimum"""
    if first.size != second.size:
        raise AssertionError("theme changed logical output dimensions")
    count = sum(1 for before, after in zip(pixels(first), pixels(second))
                if (sum(before) - sum(after) if darker else sum(after) - sum(before)) > 300)
    if count < minimum:
        raise AssertionError(f"visible theme changed only {count} pixels, expected at least {minimum}")
    return count


def equal(first, second, label):
    """【主题回归】【完整恢复】逐像素比较 first、second；label 标识缓存恢复或导出，差异立即报错，无返回值"""
    if first.size != second.size or ImageChops.difference(first, second).getbbox() is not None:
        raise AssertionError(f"{label} changed image pixels: {first.size} / {second.size}")


def color_count(image, color, minimum):
    """【主题回归】【原版实色】在 image 中核对 color 的实际 RGB 覆盖，允许一单位舍入；返回数量，少于 minimum 报错"""
    count = sum(1 for pixel in pixels(image)
                if max(abs(pixel[channel] - color[channel]) for channel in range(3)) <= 1)
    if count < minimum:
        raise AssertionError(f"Qt palette {color} occupies only {count} pixels")
    return count


def idle(process, root, name):
    """【主题回归】【静止资源】在 process 完成后台任务后采样一秒 CPU 和提交次数，root/name 指定日志；返回资源记录"""
    session.wait_single_thread(process.pid)
    time.sleep(0.5)
    before = session.resources(process.pid)
    log = root / f"{name}.log"
    damage = log.read_text().count(".damage_buffer(")
    time.sleep(1)
    after = session.resources(process.pid)
    assert after["threads"] == 1 and before["cpu_ticks"] == after["cpu_ticks"], (before, after)
    assert damage == log.read_text().count(".damage_buffer("), "idle theme keeps submitting frames"
    return {"before": before, "after": after, "damage": damage}


def clipboard(environment):
    """【主题回归】【原图读取】从 environment 读取 PNG 剪贴板并返回 RGBA，保留物理分辨率和透明通道"""
    data = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment, timeout=3)
    with Image.open(io.BytesIO(data)) as image:
        return image.convert("RGBA")


def properties(root, config, baseline):
    """【主题回归】【文字与取色】按 baseline 定位实际属性面板，验证浅色、深色和活动取色弹层缓存，返回报告"""
    session.key("t")
    time.sleep(0.3)
    light = screenshot(root, "text-properties-light")
    # 1. 【主题回归】【真实定位】工具栏位于顶部，新增属性面板必须在顶部以下产生完整矩形，不能用推算位置代替实际呈现
    changed_bounds = ImageChops.difference(baseline.crop((0, 54, baseline.width, baseline.height)),
                                           light.crop((0, 54, light.width, light.height))).getbbox()
    assert changed_bounds is not None, "Text did not show its properties"
    left, top, right, bottom = changed_bounds
    top, bottom = top + 54, bottom + 54
    assert 400 <= right - left <= 470 and 40 <= bottom - top <= 50, changed_bounds
    bounds = (left, top, right, bottom)
    toggle(config, "dark")
    dark = screenshot(root, "text-properties-dark")
    count = changed(light.crop(bounds), dark.crop(bounds), 7000, True)
    toggle(config, "light")
    equal(light, screenshot(root, "text-properties-restored"), "text properties cache")
    # 2. 【主题回归】【同一取色器】颜色按钮位于原版三十二像素控件内；右键不能打开，左键释放打开后才验证主题
    session.control("click", "--logical", left + 338, top + 22, "--button", "right")
    session.key("t")
    session.control("click", "--logical", left + 338, top + 22)
    time.sleep(0.3)
    picker_light = screenshot(root, "color-picker-light")
    toggle(config, "dark")
    picker_dark = screenshot(root, "color-picker-dark")
    picker_count = changed(picker_light, picker_dark, count + 3000, True)
    toggle(config, "light")
    equal(picker_light, screenshot(root, "color-picker-restored"), "active color picker cache")
    # 3. 【主题回归】【无标注交付】切换移动工具清理面板，不在原图上新增文字或取色标注
    session.key("s")
    session.control("move", "--logical", 30, 30)
    time.sleep(0.3)
    equal(baseline, screenshot(root, "editor-after-properties"), "properties did not mutate source or toolbar")
    return {"bounds": list(bounds), "text_changed_pixels": count, "picker_changed_pixels": picker_count}


def editor(binary, environment, root, config, source):
    """【主题回归】【编辑器切换】用 source 启动 binary，核对默认浅色、双向恢复、config 持久化和原图，返回报告"""
    arguments = ["--fullscreen", "--to", "c", "--config", config, source]
    with launch(binary, environment, root, "editor", arguments) as process:
        wait_window(process, "mark-shot")
        session.control("move", "--logical", 30, 30)
        time.sleep(0.4)
        light = screenshot(root, "editor-light")
        toggle(config, "dark")
        dark = screenshot(root, "editor-dark")
        count = changed(light, dark, 3000, True)
        toggle(config, "light")
        equal(light, screenshot(root, "editor-light-restored"), "toolbar cache roundtrip")
        property_report = properties(root, config, light)
        resources = idle(process, root, "editor")
        toggle(config, "dark")
        session.key("ctrl+c")
        assert process.wait(timeout=5) == 0
        with Image.open(source) as expected:
            equal(clipboard(environment), expected.convert("RGBA"), "theme-only image export")
    # 1. 【主题回归】【重开继承】新进程读取刚保存的深色值，必须实际呈现同一深色工具栏
    with launch(binary, environment, root, "editor-reopen", arguments) as process:
        wait_window(process, "mark-shot")
        session.control("move", "--logical", 30, 30)
        time.sleep(0.4)
        equal(dark, screenshot(root, "editor-reopen-dark"), "persisted dark toolbar")
        toggle(config, "light")
        session.key("Escape")
        assert process.wait(timeout=5) == 0
    return {"changed_pixels": count, "properties": property_report,
            "idle": resources, "source_pixels": [900, 560]}


def capture_selection(binary, environment, root, config):
    """【主题回归】【初始选框】冻结私有桌面后实际拖选，四角必须保留原图而非圆角遮罩，返回角点与资源证据"""
    screenshot(root, "desktop-before-capture")
    with launch(binary, environment, root, "capture", ["--to", "", "--config", config]) as process:
        wait_window(process, "mark-shot")
        session.control("drag", "--from", 180, 140, "--to", 820, 500, "--steps", 12)
        time.sleep(0.4)
        actual = screenshot(root, "capture-square-selection")
        points = [(181, 141), (818, 141), (181, 498), (818, 498)]
        resources = idle(process, root, "capture")
        # 1. 【主题回归】【同次冻结图】另一次桌面截图可能包含合成器过渡，必须用同次捕获导出的原始区域作基准
        session.key("ctrl+c")
        assert process.wait(timeout=5) == 0
        frozen = clipboard(environment)
        frozen.save(root / "capture-frozen-region.png")
        expected_points = [(2, 2), (frozen.width - 3, 2),
                           (2, frozen.height - 3), (frozen.width - 3, frozen.height - 3)]
        colors = []
        for point, expected_point in zip(points, expected_points):
            expected = frozen.getpixel(expected_point)[:3]
            assert actual.getpixel(point) == expected, (
                "capture rounded or obscured its frozen corner", point, actual.getpixel(point), expected)
            colors.append(expected)
    return {"corners": points, "frozen_colors": colors,
            "source_pixels": list(frozen.size), "idle": resources}


def first_use(binary, environment, root, config, source):
    """【主题回归】【首次模型引导】打开未安装模型的 OCR，核对强调文字可见性和双主题；不点击下载，返回文字像素证据"""
    with launch(binary, environment, root, "first-use", ["--fullscreen", "--to", "", "--config", config, source]) as process:
        wait_window(process, "mark-shot")
        session.control("move", "--logical", 30, 30)
        session.key("o")
        time.sleep(0.4)
        light = screenshot(root, "ocr-guide-light").crop((0, 54, 1067, 667))
        light_ink = sum(1 for r, g, b in pixels(light)
                        if r < 150 and 70 < g < 210 and g - r > 45 and b - r > 40)
        assert light_ink >= 40, f"light model guide accent text is too faint: {light_ink} pixels"
        toggle(config, "dark")
        dark = screenshot(root, "ocr-guide-dark").crop((0, 54, 1067, 667))
        dark_ink = sum(1 for r, g, b in pixels(dark)
                       if g > 130 and g - r > 45 and b - r > 45)
        assert dark_ink >= 40, f"dark model guide accent text is too faint: {dark_ink} pixels"
        toggle(config, "light")
        models = root / "data/mark-shot/models"
        assert not models.exists() or not any(models.iterdir()), "guide downloaded a model without consent"
        session.key("Escape")
        if process.poll() is None:
            session.key("Escape")
        assert process.wait(timeout=5) == 0
    return {"light_accent_pixels": light_ink, "dark_accent_pixels": dark_ink}


def ocr(binary, environment, root, config):
    """【主题回归】【结果与菜单】直接传输固定 OCR 帧，验证窗口和活动菜单切换，不初始化模型，返回报告"""
    with launch(binary, environment, root, "ocr", ["--ocr-result", "--config", config], ocr_frame()) as process:
        window = session.size_window(wait_window(process, "mark-shot-ocr-result"), 760, 460)
        left, top = window["floating_pos"]
        bounds = (round(left), round(top), round(left + 760), round(top + 460))
        title = (bounds[0], bounds[1], bounds[2], bounds[1] + 38)
        light = screenshot(root, "ocr-light")
        count_light = color_count(light.crop(bounds), (248, 250, 252), 10000)
        toggle(config, "dark")
        dark = screenshot(root, "ocr-dark")
        count_dark = color_count(dark.crop(bounds), (15, 23, 42), 10000)
        toggle(config, "light")
        equal(light.crop(title), screenshot(root, "ocr-light-restored").crop(title), "OCR title cache")
        # 2. 【主题回归】【活动菜单】同一原生弹层保持抓取，切换必须重建菜单像素而非只更新父窗口
        session.click(window, 675, 19)
        time.sleep(0.2)
        session.key("Down")
        session.key("Right")
        time.sleep(0.2)
        nodes = active_tree((root / "ocr.log").read_text())
        assert len(nodes) == 2 and all(node["rect"] is not None for node in nodes), "More did not open parent and child popups"
        menu_light = screenshot(root, "ocr-menu-light")
        menu_bounds = []
        x, y = left, top
        for node in nodes:
            dx, dy, width, height = node["rect"]
            x, y = x + dx, y + dy
            menu_bounds.append((round(x), round(y), round(x + width), round(y + height)))
        toggle(config, "dark")
        menu_dark = screenshot(root, "ocr-menu-dark")
        assert [node["surface"] for node in active_tree((root / "ocr.log").read_text())] == [node["surface"] for node in nodes], "theme replaced the live menu grab"
        menu_count = [changed(menu_light.crop(bounds), menu_dark.crop(bounds), 4000, True) for bounds in menu_bounds]
        toggle(config, "light")
        restored = screenshot(root, "ocr-menu-restored")
        for bounds in menu_bounds:
            equal(menu_light.crop(bounds), restored.crop(bounds), "OCR live menu cache")
        equal(menu_light.crop(title), restored.crop(title), "OCR menu parent")
        session.key("Escape")
        session.key("Escape")
        assert process.poll() is None, "theme key or popup Escape closed result window"
        resources = idle(process, root, "ocr")
        session.key("Escape")
        assert process.wait(timeout=5) == 0
    return {"light_window_pixels": count_light, "dark_window_pixels": count_dark,
            "menu_changed_pixels": menu_count, "idle": resources}


def history(binary, environment, root, config):
    """【主题回归】【历史窗口】验证独立历史窗口使用共享主题且切换后恢复，config 其他字段保持原样，返回报告"""
    with launch(binary, environment, root, "history", ["--history", "--config", config]) as process:
        wait_window(process, "mark-shot-history")
        light = screenshot(root, "history-light")
        count_light = color_count(light, (248, 250, 252), 10000)
        toggle(config, "dark")
        dark = screenshot(root, "history-dark")
        count_dark = color_count(dark, (15, 23, 42), 10000)
        toggle(config, "light")
        equal(light, screenshot(root, "history-restored"), "history theme roundtrip")
        resources = idle(process, root, "history")
        session.key("Escape")
        assert process.wait(timeout=5) == 0
    return {"light_pixels": count_light, "dark_pixels": count_dark, "idle": resources}


def pin(binary, environment, root, config, source):
    """【主题回归】【贴图菜单】验证 source 贴图的活动菜单明暗切换和完整恢复，保留原图，返回报告"""
    with launch(binary, environment, root, "pin", ["--pin", "--position", "80,160", "--config", config, source]) as process:
        session.wait_single_thread(process.pid)
        time.sleep(0.5)
        session.control("click", "--logical", 130, 200, "--button", "right")
        time.sleep(0.2)
        light = screenshot(root, "pin-menu-light")
        toggle(config, "dark")
        dark = screenshot(root, "pin-menu-dark")
        count = changed(light, dark, 10000, True)
        toggle(config, "light")
        equal(light, screenshot(root, "pin-menu-restored"), "pin native popup cache")
        session.key("Escape")
        session.key("ctrl+c")
        with Image.open(source) as expected:
            equal(clipboard(environment), expected.convert("RGBA"), "pin image after theme switching")
        resources = idle(process, root, "pin")
        session.key("Escape")
        assert process.wait(timeout=5) == 0
    return {"menu_changed_pixels": count, "idle": resources}


def run(binary, root):
    """【主题回归】【完整执行】在私有会话验证 binary，root 保存所有证据；返回报告，不读取用户配置"""
    environment = session.environment()
    root.mkdir(parents=True, exist_ok=False)
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    config = root / "config/mark-shot/config.toml"
    config.parent.mkdir(parents=True)
    config.write_text('[general]\noverlay = "window"\nto = ""\n[notifications]\nenabled = false\n'
                      '[history]\nenabled = false\nlimit = 23\n[custom]\nsentinel = "retained"\n')
    source = root / "source.png"
    Image.new("RGBA", (900, 560), (205, 205, 205, 255)).save(source)
    previous = os.environ.copy()
    os.environ.update(environment)
    try:
        report = {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                  "editor": editor(binary, environment, root, config, source),
                  "capture_selection": capture_selection(binary, environment, root, config),
                  "first_use": first_use(binary, environment, root, config, source),
                  "ocr": ocr(binary, environment, root, config),
                  "history": history(binary, environment, root, config),
                  "pin": pin(binary, environment, root, config, source)}
    finally:
        os.environ.clear()
        os.environ.update(previous)
    models = root / "data/mark-shot/models"
    assert not models.exists() or not any(models.iterdir()), "theme or model guide downloaded OCR files"
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    """【主题回归】【入口】解析二进制与证据目录，执行真实回归并输出结果，无参数、无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
