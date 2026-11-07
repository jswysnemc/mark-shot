#!/usr/bin/env python3
"""【优先界面回归】【原生输入】私有Sway验证主题、工具滚轮气泡与置顶图中文菜单"""

import argparse
from contextlib import contextmanager
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import subprocess
import time
import tomllib

from PIL import Image, ImageChops
from wayland_theme_transition import toggle as wait_theme

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("priority_ruler_support", PROJECT / "scripts/wayland-ruler-chrome.py")
SUPPORT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SUPPORT)
INPUT, FLOW, session = SUPPORT.INPUT, SUPPORT.FLOW, SUPPORT.session


def snapshot(root, name):
    """【优先界面回归】【画面采集】按root和name保存私有合成器实际画面，返回RGB图像"""
    path = root / f"{name}.png"
    session.screenshot(path)
    return Image.open(path).convert("RGB")


def wait_frame(root, name, predicate, message):
    """【优先界面回归】【呈现等待】按predicate检查本次画面直到满足，root和name保留失败证据，返回图像"""
    deadline = time.monotonic() + 3
    while True:
        image = snapshot(root, name)
        if predicate(image):
            return image
        assert time.monotonic() < deadline, message
        time.sleep(0.04)


def toggle(config, expected):
    """【优先界面回归】【主题切换】发送一次原生快捷键并核对config保存expected，无返回值"""
    wait_theme(config, expected, session.key, session.screenshot)
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        values = tomllib.loads(config.read_text())
        if values.get("ui", {}).get("theme") == expected:
            assert values["custom"]["sentinel"] == "retained"
            return
        time.sleep(0.03)
    raise AssertionError(f"theme did not persist {expected}")


def changed(first, second, region):
    """【优先界面回归】【区域像素】按first和second比较region内差异，返回非空范围或空值"""
    return ImageChops.difference(first.crop(region), second.crop(region)).getbbox()


def editor(binary, environment, root, config, source):
    """【优先界面回归】【工具提示】通过真实滚轮检查三种工具、移动与到期及主题，返回证据"""
    with INPUT.launch(binary, environment, config, source, root, "editor") as (process, log):
        FLOW.wait_input(process, log, (320, 240))
        time.sleep(0.3)
        light = snapshot(root, "editor-light")
        toolbar = (0, 0, light.width, 100)
        toggle(config, "dark")
        dark = wait_frame(root, "editor-dark", lambda image: changed(light, image, toolbar) is not None, "toolbar did not switch to dark")
        toggle(config, "light")
        wait_frame(root, "editor-restored", lambda image: changed(light, image, toolbar) is None, "toolbar did not restore its light cache")
        cases = []
        for name, key in [("pen", "p"), ("number", "n"), ("text", "t")]:
            session.key(key)
            INPUT.move((320, 240))
            time.sleep(0.25)
            before = snapshot(root, f"{name}-before")
            region = (300, 220, 540, 320)
            SUPPORT.HIT.wheel(1, 1)
            shown = wait_frame(root, f"{name}-shown", lambda image: changed(before, image, region) is not None, f"{name} has no pointer value bubble")
            INPUT.move((400, 280))
            moved = wait_frame(root, f"{name}-moved", lambda image: changed(shown, image, (300, 220, 570, 360)) is not None, f"{name} value bubble did not follow pointer")
            time.sleep(1.1)
            expired = snapshot(root, f"{name}-expired")
            assert changed(moved, expired, (370, 255, 570, 355)) is not None, f"{name} bubble did not expire without movement"
            cases.append({"tool": name, "shown": True, "followed": True, "expired": True})
        session.key("p")
        INPUT.move((320, 240))
        session.key("ctrl+c")
        assert process.wait(timeout=5) == 0
        data = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment, timeout=3)
        actual = Image.open(io.BytesIO(data)).convert("RGBA")
        expected = Image.open(source).convert("RGBA")
        assert actual.size == expected.size and actual.tobytes() == expected.tobytes(), "wheel or theme preview entered the exported image"
    return {"theme_roundtrip": True, "wheel_cases": cases, "unchanged_source": True, "dark_frame_changed": changed(light, dark, toolbar) is not None}


def ruler(binary, environment, root, config, source):
    """【优先界面回归】【尺子悬停】实际显示坐标与颜色提示，验证浅色底板、深色恢复与原图保持，返回证据"""
    with INPUT.launch(binary, environment, config, source, root, "ruler") as (process, log):
        FLOW.wait_input(process, log, (320, 240))
        session.key("u")
        INPUT.move((320, 240))
        region = (334, 254, 610, 282)
        light = wait_frame(root, "ruler-hover-light", lambda image: sum(min(pixel) > 230 for pixel in image.crop(region).get_flattened_data()) > 1500, "ruler hover still has a dark background in light theme")
        toggle(config, "dark")
        dark = wait_frame(root, "ruler-hover-dark", lambda image: sum(max(pixel) < 100 for pixel in image.crop(region).get_flattened_data()) > 1500, "ruler hover did not use dark theme")
        toggle(config, "light")
        wait_frame(root, "ruler-hover-restored", lambda image: ImageChops.difference(light, image).getbbox() is None, "ruler hover theme did not restore exact full pixels")
        session.key("ctrl+c")
        assert process.wait(timeout=5) == 0
        data = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment, timeout=3)
        actual = Image.open(io.BytesIO(data)).convert("RGBA")
        expected = Image.open(source).convert("RGBA")
        assert actual.size == expected.size and actual.tobytes() == expected.tobytes(), "ruler hover entered the exported original image"
        return {"hover_light_pixels": sum(min(pixel) > 230 for pixel in light.crop(region).get_flattened_data()),
                "hover_dark_pixels": sum(max(pixel) < 100 for pixel in dark.crop(region).get_flattened_data()),
                "full_roundtrip": True, "unchanged_source": True}


@contextmanager
def launch_pin(binary, environment, root, config, source):
    """【优先界面回归】【置顶图进程】按参数启动独立贴图，退出时只回收本次进程，返回句柄与日志"""
    path = root / "pin.log"
    with path.open("w") as log:
        process = subprocess.Popen([str(binary), "--pin", "--position", "80,120", "--config", str(config), str(source)], env=dict(environment, WAYLAND_DEBUG="client"), stdout=subprocess.DEVNULL, stderr=log)
        try:
            session.wait_single_thread(process.pid)
            time.sleep(0.4)
            yield process, path
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)


def pin(binary, environment, root, config, source):
    """【优先界面回归】【中文原生菜单】打开真实菜单、切换双主题并采样空闲，返回原生表面和资源证据"""
    with launch_pin(binary, environment, root, config, source) as (process, log):
        INPUT.click((130, 160), "right")
        deadline = time.monotonic() + 3
        while True:
            matches = re.findall(r"xdg_popup@(\d+)\.configure, \((-?\d+), (-?\d+), (\d+), (\d+)\)", log.read_text())
            if matches:
                break
            assert time.monotonic() < deadline, "pin native menu did not configure"
            time.sleep(0.04)
        role, x, y, width, height = map(int, matches[-1])
        bounds = (80 + x, 120 + y, 80 + x + width, 120 + y + height)
        time.sleep(0.2)
        light = snapshot(root, "pin-chinese-light")
        light.crop(bounds).save(root / "pin-menu-light-crop.png")
        toggle(config, "dark")
        dark = wait_frame(root, "pin-chinese-dark", lambda image: changed(light, image, bounds) is not None, "pin menu did not redraw in dark theme")
        dark.crop(bounds).save(root / "pin-menu-dark-crop.png")
        toggle(config, "light")
        wait_frame(root, "pin-chinese-restored", lambda image: changed(light, image, bounds) is None, "live pin menu did not restore its original light pixels")
        session.key("Escape")
        assert process.poll() is None, "popup escape closed the pin"
        idle = FLOW.NAV.SIZE.idle(process, log)
        session.key("Escape")
        assert process.wait(timeout=5) == 0
    return {"popup_role": role, "bounds": bounds, "native_theme_roundtrip": True, "idle": idle}


def run(binary, root):
    """【优先界面回归】【完整入口】按binary和新root执行已校验的私有单输出流程，返回不可变指纹与结果"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"), MARK_SHOT_LANG="chinese", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/priority-ui")
    config = root / "config.toml"
    config.write_text('[general]\noverlay="window"\nto=""\n[notifications]\nenabled=false\n[history]\nenabled=false\n[ui]\nlanguage="chinese"\n[custom]\nsentinel="retained"\n')
    source = root / "source.png"
    Image.new("RGBA", (500, 320), (205, 205, 205, 255)).save(source)
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    previous = dict(os.environ)
    os.environ.update(environment)
    try:
        with session.input_context(), FLOW.NAV.SIZE.pointer_context():
            session.sway.keyboard_ready()
            report = {"binary_sha256": fingerprint, "editor": editor(binary, environment, root, config, source), "ruler": ruler(binary, environment, root, config, source), "pin": pin(binary, environment, root, config, source)}
    finally:
        os.environ.clear()
        os.environ.update(previous)
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == fingerprint
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    """【优先界面回归】【命令入口】解析独立binary与新root并执行私有流程，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
