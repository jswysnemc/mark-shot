#!/usr/bin/env python3
"""【组件主题回归】【真实画面】私有Wayland检查旧配置在首帧、属性、模型和菜单中的浅色继承"""

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import time

from PIL import Image, ImageChops
from wayland_theme_transition import toggle

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("component_priority", PROJECT / "scripts/wayland-priority-ui.py")
SUPPORT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SUPPORT)
INPUT, FLOW, session = SUPPORT.INPUT, SUPPORT.FLOW, SUPPORT.session
LEGACY = '''[theme]
background="#0F1117E6"
foreground="#E5E7EB"
field="#272D2F"
track="#3B4647"
caret="#F5E0DC"
[toolbar]
background="#1E2124FA"
icon="#E5EDEF"
[settings_panel]
background="#1E2124FA"
text="#E5EDEF"
[color_picker]
background="#1E2124FA"
text="#E5EDEF"
label="#A2B1B4"
[model_picker]
background="#1E2124FA"
[toasts]
background="#1E2124FA"
text="#E5EDEF"
'''


def snapshot(root, name):
    """【组件主题回归】【实际呈现】按root和name保存本次真实画面，返回RGB图像"""
    time.sleep(0.25)
    path = root / f"{name}.png"
    session.screenshot(path)
    return Image.open(path).convert("RGB")


def bright(image, bounds=None):
    """【组件主题回归】【浅色面积】按image和可选bounds统计近中性的浅色像素，固定灰色原图不计入，返回数量"""
    target = image.crop(bounds) if bounds else image
    return sum(min(pixel) > 225 and max(pixel) - min(pixel) < 15 for pixel in target.get_flattened_data())


def emphasis(image):
    """【组件主题回归】【文字可见性】按image统计不透明青色强调字形，半透明背景色混入文字不计入，返回数量"""
    return sum(pixel[0] < 70 and pixel[1] > 100 and pixel[2] > 70 and pixel[1] - pixel[0] > 60 for pixel in image.get_flattened_data())


def verify(binary, environment, root, explicit):
    """【组件主题回归】【旧配置矩阵】以binary检查explicit指定的主题模式、首帧与真实模型面板，返回恢复和导出证据"""
    root.mkdir()
    config = root / "config.toml"
    config.write_text('[general]\noverlay="window"\nto=""\n[notifications]\nenabled=false\n[history]\nenabled=false\n' + ('[ui]\ntheme="light"\n' if explicit else '') + LEGACY)
    source = root / "source.png"
    Image.new("RGBA", (500, 320), (205, 205, 205, 255)).save(source)
    with INPUT.launch(binary, environment, config, source, root, "editor") as (process, log):
        FLOW.wait_input(process, log, (320, 240))
        light = snapshot(root, "first-light")
        top = (0, 0, light.width, 100)
        assert bright(light, top) > 3500, "the first toolbar frame still uses old dark component colors"
        session.key("t")
        properties = snapshot(root, "text-properties-light")
        assert bright(properties, top) > 3500, "the text property panel is still dark"
        session.key("o")
        model = snapshot(root, "model-guide-light")
        changed = ImageChops.difference(properties, model).getbbox()
        assert changed and bright(model, changed) > 12000, "the OCR model download guide still uses its old dark background"
        assert emphasis(model) > emphasis(properties) + 50, "model guide emphasis text still uses transparent background colors"
        before = model.copy()
        toggle(config, "dark", session.key, session.screenshot)
        dark = snapshot(root, "model-guide-dark")
        assert bright(dark, changed) < bright(before, changed) / 2, "the active model guide did not use dark theme"
        toggle(config, "light", session.key, session.screenshot)
        restored = snapshot(root, "model-guide-light-restored")
        assert ImageChops.difference(before, restored).getbbox() is None, "theme restoration retained a stale component cache"
        session.key("p")
        session.key("ctrl+c")
        assert process.wait(timeout=5) == 0
    return {"explicit_light": explicit, "first_light_pixels": bright(light, top), "model_light_pixels": bright(model, changed), "model_dark_pixels": bright(dark, changed), "model_visible_emphasis_pixels": emphasis(model), "full_theme_roundtrip": True}


def run(binary, root):
    """【组件主题回归】【私有入口】按binary和新root执行旧模板两种启动方式及中文菜单，无返回值"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"), MARK_SHOT_LANG="english", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/component-theme")
    previous = dict(os.environ)
    os.environ.update(environment)
    try:
        with session.input_context(), FLOW.NAV.SIZE.pointer_context():
            session.sway.keyboard_ready()
            report = {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "cases": [verify(binary, environment, root / name, explicit) for name, explicit in [("explicit", True), ("default", False)]]}
    finally:
        os.environ.clear()
        os.environ.update(previous)
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    """【组件主题回归】【命令入口】解析独立binary与root后执行本次私有流程，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
