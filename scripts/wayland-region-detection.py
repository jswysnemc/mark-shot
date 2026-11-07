#!/usr/bin/env python3
"""【区域识别原生回归】【独立会话】验证悬停、点击、滚轮、拖选、导出和工作线程回收"""

import argparse
import hashlib
import io
import json
import os
import re
import subprocess
import sys
import time
from contextlib import contextmanager
from pathlib import Path

import wayland_region_detection_checks as checks
import wayland_text_format_session as session
from PIL import Image
from wayland_scroll_raw_capture import capture as raw_capture
from wayland_text_pointer import CURRENT, click, pointer_context
from wayland_virtual_pointer import transform_for_pointer


def digest(path):
    """【区域识别原生回归】【构建指纹】按 path 返回文件 SHA-256"""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def move(point):
    """【区域识别原生回归】【指针移动】向已校验私有设备发送 point，无返回值"""
    pointer, logical, _, x11 = CURRENT.get()
    if x11:
        environment, mode = x11
        coordinates = [round((point[axis] - logical[["x", "y"][axis]]) * mode[["width", "height"][axis]] / logical[["width", "height"][axis]]) for axis in range(2)]
        adjacent = [max(0, coordinates[0] - 2), max(0, coordinates[1] - 2)]
        for location in (adjacent, coordinates):
            subprocess.run(["xdotool", "mousemove", "--sync", *map(str, location)], env=environment, check=True, timeout=3)
            time.sleep(0.03)
    else:
        pointer.move_abs(*transform_for_pointer(point[0], point[1], logical["width"], logical["height"], logical.get("pointer_transform", logical["transform"])))
    time.sleep(0.06)


def press(pressed):
    """【区域识别原生回归】【按钮状态】按 pressed 发送左键按下或释放，无返回值"""
    pointer, _, _, x11 = CURRENT.get()
    if x11:
        subprocess.run(["xdotool", "mousedown" if pressed else "mouseup", "1"], env=x11[0], check=True, timeout=3)
    elif pressed:
        pointer.press()
    else:
        pointer.release()
    time.sleep(0.04)


@contextmanager
def wheel():
    """【区域识别原生回归】【父区域滚轮】发送一刻度并返回验证范围，结束后释放私有 X11 按钮，无参数"""
    pointer, _, _, x11 = CURRENT.get()
    if x11:
        # 1. 【区域识别原生回归】【私有输入】winit 对按下与释放各发送一刻度，验证完成后才释放按钮
        subprocess.run(["xdotool", "mousedown", "4"], env=x11[0], check=True, timeout=3)
    else:
        pointer.scroll(dy=-10)
    try:
        time.sleep(0.08)
        yield
    finally:
        if x11:
            subprocess.run(["xdotool", "mouseup", "4"], env=x11[0], check=True, timeout=3)


def snapshot(root, name, physical=False):
    """【区域识别原生回归】【实际画面】按 root、name 保存私有输出，physical 指定原始像素，返回 RGBA 图像"""
    path = root / f"{name}.png"
    if physical:
        # 1. 【区域识别原生回归】【冻结基线】直接读取合成器缓冲，grim 会按逻辑尺寸重新采样分数比例输出
        raw_capture().save(path)
    else:
        session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGBA")


def color_bounds(image, values):
    """【区域识别原生回归】【来源边界】按 image 中 values 灰色像素返回实际矩形，无命中视为失败"""
    mask = Image.new("L", image.size)
    mask.putdata([255 if pixel[0] == pixel[1] == pixel[2] and pixel[0] in values else 0 for pixel in image.getdata()])
    bounds = mask.getbbox()
    assert bounds, f"source colors {values} are missing"
    return bounds


def hover_bounds(image):
    """【区域识别原生回归】【候选像素】按 image 返回原版薄荷色边框的可见范围，无候选返回空值"""
    mask = Image.new("L", image.size)
    mask.putdata([255 if 40 < pixel[0] < 150 and pixel[1] > 170 and 130 < pixel[2] < 245 else 0 for pixel in image.getdata()])
    return mask.getbbox()


def wait_hover(process, root, name, expected):
    """【区域识别原生回归】【呈现确认】按 process、root、name 等待 expected 的候选边框，返回实际截图边界"""
    deadline = time.monotonic() + 4
    while True:
        assert process.poll() is None, "editor exited before hover verification"
        image = snapshot(root, name)
        actual = hover_bounds(image)
        if expected is None and actual is None:
            return actual
        if actual is not None and expected is not None and all(abs(left - right) <= 2 for left, right in zip(actual, expected)):
            return actual
        assert time.monotonic() < deadline, f"{name}: hover bounds {actual} != {expected}"
        time.sleep(0.04)


def logical_rect(rect, physical, logical):
    """【区域识别原生回归】【显示坐标】按 rect、physical 和 logical 把实际物理边界映射为输出逻辑边界"""
    return tuple(rect[index] * logical[["width", "height"][index % 2]] / physical[index % 2] for index in range(4))


def center(rect):
    """【区域识别原生回归】【区域中心】按 rect 返回逻辑中心位置"""
    return (rect[0] + rect[2]) / 2, (rect[1] + rect[3]) / 2


def idle(process, log):
    """【区域识别原生回归】【空闲采样】按 process 和 log 验证单线程、稳定 CPU 与提交数，返回实际样本"""
    session.wait_single_thread(process.pid)
    time.sleep(0.15)
    samples = []
    for _ in range(5):
        sample = session.resources(process.pid)
        sample["commits"] = len(re.findall(r"wl_surface@\d+\.commit\(", log.read_text()))
        samples.append(sample)
        time.sleep(0.08)
    assert all(sample["threads"] == 1 for sample in samples), samples
    assert len({sample["cpu_ticks"] for sample in samples}) == 1, samples
    assert len({sample["commits"] for sample in samples}) == 1, samples
    return samples


@contextmanager
def launch(binary, environment, root, name, image, windows):
    """【区域识别原生回归】【截图会话】按参数启动独立区域交付进程，返回进程、日志与输出路径，退出回收本次进程"""
    directory = root / name
    directory.mkdir()
    config = directory / "config.toml"
    config.write_text(f'[general]\noverlay="layer"\nto="o"\n[history]\nenabled=false\n[notifications]\nenabled=false\n[region_detection]\nenabled={str(image).lower()}\n[window_detection]\nenabled={str(windows).lower()}\n')
    private = dict(environment, WAYLAND_DEBUG="client", XDG_CONFIG_HOME=str(directory / "config"), XDG_DATA_HOME=str(directory / "data"), XDG_STATE_HOME=str(directory / "state"))
    log, output = directory / "client.log", directory / "output.png"
    with log.open("w") as errors, output.open("wb") as stdout:
        process = subprocess.Popen([str(binary), "--select-region", "--to", "o", "--config", str(config), "--nonotify"], env=private, stdout=stdout, stderr=errors)
        try:
            deadline = time.monotonic() + 5
            while True:
                assert process.poll() is None, log.read_text()
                if ".damage_buffer(0, 0," in log.read_text():
                    break
                assert time.monotonic() < deadline, "editor did not present its initial buffer"
                time.sleep(0.03)
            session.wait_single_thread(process.pid)
            session.key("shift+m")
            time.sleep(0.15)
            yield process, log, output, directory
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)


def finish(process, output, expected):
    """【区域识别原生回归】【完整交付】按 process、output 与 expected 验证退出及全部 PNG 像素，返回输出大小"""
    assert process.wait(timeout=5) == 0
    with Image.open(io.BytesIO(output.read_bytes())) as image:
        image = image.convert("RGBA")
        assert image.size == expected.size, (image.size, expected.size)
        assert image.tobytes() == expected.tobytes(), "region export differs from the original frozen pixels"
        return list(image.size)


def run(binary, target, root):
    """【区域识别原生回归】【完整矩阵】在已校验私有会话使用 binary、target 并保存 root 证据，返回完整结果"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment["DBUS_SESSION_BUS_ADDRESS"] = "unix:path=/nonexistent/region-detection-test"
    runtime = json.loads((Path(environment["XDG_RUNTIME_DIR"]) / "runtime.json").read_text())
    logical = next(iter(runtime["outputs"].values()))["logical"]
    compositor = runtime.get("compositor", "niri")
    if compositor == "niri":
        environment["NIRI_CONFIG"] = str(Path(environment["XDG_RUNTIME_DIR"]) / "niri-child/config.kdl")
    fingerprint, target_fingerprint = digest(binary), digest(target)
    previous = dict(os.environ)
    os.environ.update(environment)
    trace = root / "source.json"
    reports = []
    try:
        with session.input_context(), pointer_context(backend="virtual" if compositor == "sway" else "x11"):
            if compositor == "sway":
                session.sway.keyboard_ready()
            with (root / "source.log").open("w") as log:
                source = subprocess.Popen([str(target), str(trace)], env=environment, stdout=subprocess.DEVNULL, stderr=log)
                try:
                    deadline = time.monotonic() + 4
                    while not trace.exists():
                        assert source.poll() is None, "source target did not map"
                        assert time.monotonic() < deadline, "source trace did not arrive"
                        time.sleep(0.03)
                    time.sleep(0.25)
                    baseline = snapshot(root, "source-physical", physical=True)
                    child, parent, window = [color_bounds(baseline, colors) for colors in ({75}, {45, 75}, {20, 45, 75})]
                    child_logical = logical_rect(child, baseline.size, logical)
                    parent_logical = logical_rect(parent, baseline.size, logical)
                    window_logical = logical_rect(window, baseline.size, logical)
                    point = center(child_logical)
                    empty = None
                    for name, image, windows in [("both", True, True), ("image", True, False), ("window", False, True), ("disabled", False, False)]:
                        with launch(binary, environment, root, name, image, windows) as (process, client, output, directory):
                            move(point)
                            expected_rect = child_logical if image else window_logical if windows else None
                            shown = wait_hover(process, directory, "hover", expected_rect)
                            resources = idle(process, client)
                            if not image and not windows:
                                empty = snapshot(directory, "no-candidate")
                            click(point)
                            if image or windows:
                                size = finish(process, output, baseline.crop(child if image else window))
                            else:
                                assert process.poll() is None, "disabled detection unexpectedly adopted a region"
                                click(point, "right")
                                assert process.wait(timeout=5) == 0 and output.stat().st_size == 0
                                size = None
                            reports.append({"name":name, "image":image, "windows":windows, "hover":shown, "png_size":size, "pixels_equal":True, "idle":resources})
                    with launch(binary, environment, root, "parent-wheel", True, True) as (process, client, output, directory):
                        move(point)
                        wait_hover(process, directory, "child", child_logical)
                        offset = len(client.read_text())
                        with wheel():
                            shown = wait_hover(process, directory, "parent", parent_logical)
                            axes = checks.wheel_events(client, offset)
                            resources = idle(process, client)
                            click(point)
                            size = finish(process, output, baseline.crop(parent))
                        reports.append({"name":"parent-wheel", "hover":shown, "png_size":size, "pixels_equal":True, "axis_events":axes, "idle":resources})
                    with launch(binary, environment, root, "short-drag", True, True) as (process, client, output, directory):
                        move(point)
                        wait_hover(process, directory, "child", child_logical)
                        press(True)
                        try:
                            move((point[0] + 3, point[1]))
                            wait_hover(process, directory, "short", child_logical)
                        finally:
                            press(False)
                        size = finish(process, output, baseline.crop(child))
                        reports.append({"name":"short-drag", "png_size":size, "pixels_equal":True})
                    with launch(binary, environment, root, "dismiss-drag", True, True) as (process, client, output, directory):
                        move(point)
                        wait_hover(process, directory, "child", child_logical)
                        press(True)
                        try:
                            move((point[0] + 8, point[1]))
                            wait_hover(process, directory, "dismissed", None)
                        finally:
                            press(False)
                        restored = snapshot(directory, "restored")
                        assert empty is not None
                        area = tuple(round(value) + (-4 if index < 2 else 4) for index, value in enumerate(child_logical))
                        assert restored.crop(area).tobytes() == empty.crop(area).tobytes(), "dismissed candidate left stale fill or border pixels"
                        resources = idle(process, client)
                        click(point, "right")
                        assert process.wait(timeout=5) == 0 and output.stat().st_size == 0
                        reports.append({"name":"dismiss-drag", "candidate_area_restored":True, "checked_rect":area, "idle":resources})
                    reports.extend(checks.run(sys.modules[__name__], binary, environment, root, baseline, logical, point, child_logical, empty))
                finally:
                    if source.poll() is None:
                        source.terminate()
                        source.wait(timeout=3)
    finally:
        os.environ.clear()
        os.environ.update(previous)
    assert digest(binary) == fingerprint and digest(target) == target_fingerprint
    report = {"binary_sha256":fingerprint, "target_sha256":target_fingerprint, "compositor":compositor,
              "logical":logical, "physical":list(baseline.size), "source_child":child, "source_parent":parent,
              "source_window":window, "cases":reports}
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    """【区域识别原生回归】【命令入口】解析固定 binary、target 与新 root，返回验证进程退出状态"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--target", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    report = run(options.binary.resolve(), options.target.resolve(), options.root.resolve())
    print(json.dumps({"compositor": report["compositor"], "logical": report["logical"],
                      "binary_sha256": report["binary_sha256"], "cases": [case["name"] for case in report["cases"]]}, indent=2))


if __name__ == "__main__":
    main()
