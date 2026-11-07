#!/usr/bin/env python3
"""【字体主题回归】【真实弹层】在私有Wayland验证可读性、滚动、活动编辑恢复及原始导出"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time
import tomllib

from PIL import Image, ImageChops
import wayland_text_format_session as session
from wayland_text_glyph_capture import capture
from wayland_text_pointer import pointer_context, click as pointer_click
from wayland_text_property_locator import font_button, font_popup
from wayland_theme_transition import toggle as wait_theme


def click(point):
    """【字体主题回归】【指针输入】向私有point发送左键按下与释放，无返回值"""
    pointer_click(tuple(map(round, point)), "left")
    time.sleep(0.08)


def toggle(config, mode):
    """【字体主题回归】【实际切换】发送统一快捷键，等待config保存mode和实际刷新，无返回值"""
    wait_theme(config, mode, session.key, session.screenshot)
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        if tomllib.loads(config.read_text()).get("ui", {}).get("theme") == mode:
            time.sleep(0.25)
            return
        time.sleep(0.02)
    raise AssertionError(f"theme did not save {mode}")


def external_save(config, mode):
    """【字体主题回归】【外部保存】另一进程原子替换config中的主题为mode，不发送界面输入，无返回值"""
    code = """import os,re,sys,tempfile
from pathlib import Path
p=Path(sys.argv[1]); text=p.read_text()
text,count=re.subn(r'(?m)^(theme\\s*=\\s*)\"(?:light|dark)\"$',lambda m:m[1]+'\"'+sys.argv[2]+'\"',text)
assert count==1
with tempfile.NamedTemporaryFile(dir=p.parent,delete=False) as f:
 f.write(text.encode()); f.flush(); os.fsync(f.fileno()); name=f.name
os.replace(name,p)
"""
    subprocess.run(["/usr/bin/python", "-c", code, str(config), mode], check=True, timeout=3)


def stable(process, root, name):
    """【字体主题回归】【相同帧】采样process直到两张完整截图逐像素相同，root/name保存证据，返回图像"""
    previous = capture(process, root, name + "-first")
    for index in range(8):
        current = capture(process, root, f"{name}-{index}")
        if ImageChops.difference(previous, current).getbbox() is None:
            return current
        previous = current
    raise AssertionError(f"font popup did not reach identical full frames: {name}")


def equal(first, second, name):
    """【字体主题回归】【完整恢复】逐像素比较first、second，name标识动作，差异立即失败，无返回值"""
    difference = ImageChops.difference(first, second).getbbox()
    if difference is not None:
        raise AssertionError(f"{name}: different pixels at {difference}")


def appearance(image, bounds, light):
    """【字体主题回归】【文字与轨道】按bounds检查image中普通字体字形与轨道亮度，light指定主题，返回实际计数"""
    left, top, right, bottom = bounds
    image = image.crop(bounds)
    readable = 0
    rows = 0
    for y in range(40, image.height - 46, 28):
        background = image.getpixel((image.width - 25, y + 14))
        if (sum(background) / 3 > 200) != light:
            continue
        rows += 1
        for pixel in image.crop((40, y + 4, image.width - 28, y + 24)).get_flattened_data():
            readable += (sum(pixel) / 3 < 130 if light else sum(pixel) / 3 > 150)
    if rows < 4 or readable < 500:
        raise AssertionError(f"font list has unreadable ordinary rows: {rows}, {readable}, light={light}")
    track = [image.getpixel((image.width - 13, y)) for y in range(56, image.height - 60)]
    light_track = sum(sum(pixel) / 3 > 200 for pixel in track)
    dark_track = sum(sum(pixel) / 3 < 100 for pixel in track)
    if light and light_track < 40 or not light and dark_track < 40:
        raise AssertionError(f"scrollbar did not inherit theme: {light_track}, {dark_track}")
    control = image.getpixel((11, image.height - 30))
    if (sum(control) / 3 > 200) != light:
        raise AssertionError(f"font size field retains the wrong theme: {control}")
    return {"bounds": [left, top, right, bottom], "readable_pixels": readable,
            "normal_rows": rows, "light_track_pixels": light_track, "dark_track_pixels": dark_track}


def run(binary, root):
    """【字体主题回归】【完整场景】以binary在新root验证活动文字、字体滚动与导出，返回同构建结果"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"),
                       XDG_STATE_HOME=str(root / "state"), WAYLAND_DEBUG="client")
    config = root / "config/mark-shot/config.toml"
    config.parent.mkdir(parents=True)
    config.write_text('[general]\nto="s"\noverlay="window"\n[notifications]\nenabled=false\n'
                      '[history]\nenabled=false\n[ui]\ntheme="light"\n')
    state = root / "state/mark-shot/tools.toml"
    state.parent.mkdir(parents=True)
    state.write_text('magnifier=false\nfont_size=20.0\nfont_size_unit="pt"\ntext_family="Inter"\nbold=false\nitalic=false\n')
    source = root / "source.png"
    Image.new("RGB", (1600, 900), (30, 34, 38)).save(source)
    previous_environment = dict(os.environ)
    os.environ.update(environment)
    try:
        with session.input_context(), pointer_context(), (root / "process.log").open("w") as log:
            process = subprocess.Popen([str(binary), "--fullscreen", "--fullscreen-default-tool", "text",
                "--default-color", "#FF4D4D", "--output", str(root / "export"), str(source)], env=environment, stderr=log)
            try:
                initial = capture(process, root, "initial")
                session.wait_single_thread(process.pid)
                if session.backend() is session.sway:
                    session.sway.keyboard_ready()
                point = round(initial.width * 0.70), round(initial.height * 0.64)
                click(point)
                time.sleep(0.3)
                for character in "alpha":
                    session.key(character)
                typed = stable(process, root, "typed")
                button = font_button(typed)
                click(button)
                light = stable(process, root, "font-light")
                bounds = font_popup(light)
                light_report = appearance(light, bounds, True)
                toggle(config, "dark")
                dark = stable(process, root, "font-dark")
                dark_report = appearance(dark, bounds, False)
                if ImageChops.difference(light.crop(bounds), dark.crop(bounds)).getbbox() is None:
                    raise AssertionError("font popup cache did not change")
                toggle(config, "light")
                equal(light, stable(process, root, "font-restored"), "live font popup and active document")
                # 1. 【字体主题回归】【真实翻页】点击已观察轨道底部箭头，主题往返保持滚动后全部像素和字形
                click((bounds[2] - 13, bounds[3] - 47))
                scrolled = stable(process, root, "font-scrolled")
                if ImageChops.difference(light.crop(bounds), scrolled.crop(bounds)).getbbox() is None:
                    raise AssertionError("font scrollbar did not scroll the visible list")
                toggle(config, "dark")
                stable(process, root, "scrolled-dark")
                toggle(config, "light")
                equal(scrolled, stable(process, root, "scrolled-restored"), "scrolled font popup")
                # 2. 【字体主题回归】【查询保持】逐字符输入搜索内容，往返对照完整弹层与仍在编辑的正文
                click((bounds[0] + 60, bounds[1] + 21))
                for character in "sans":
                    session.key(character)
                searched = stable(process, root, "font-search")
                toggle(config, "dark")
                stable(process, root, "search-dark")
                toggle(config, "light")
                equal(searched, stable(process, root, "search-restored"), "font search and document")
                # 3. 【字体主题回归】【外部更新】另一进程保存后保持当前搜索和正文，没有输入事件协助刷新
                external_save(config, "dark")
                external_dark = stable(process, root, "external-search-dark")
                assert ImageChops.difference(searched, external_dark).getbbox() is not None
                external_save(config, "light")
                equal(searched, stable(process, root, "external-search-restored"), "external font theme and search")
                click(button)
                session.key("ctrl+a")
                session.key("ctrl+c")
                session.wait_clipboard(environment, "alpha")
                session.key("ctrl+Return")
                session.key("ctrl+s")
                assert process.wait(timeout=6) == 0
                output = list((root / "export").glob("*.png"))
                assert len(output) == 1
                with Image.open(output[0]) as saved:
                    assert saved.size == (1600, 900)
                    equal(Image.open(source).convert("RGB").crop((0, 0, 1600, 350)),
                          saved.convert("RGB").crop((0, 0, 1600, 350)), "original source pixels")
                assert tomllib.loads(state.read_text())["text_family"] == "Inter"
                assert not (root / "data/mark-shot/models").exists()
                return {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                        "light": light_report, "dark": dark_report, "full_popup_roundtrip": True,
                        "scroll_roundtrip": True, "search_roundtrip": True, "external_font_roundtrip": True,
                        "document_text": "alpha", "original_pixels_preserved": True, "export_size": [1600, 900]}
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=3)
    finally:
        os.environ.clear()
        os.environ.update(previous_environment)


def main():
    """【字体主题回归】【命令入口】解析binary和root，保存实际回归报告，无参数、无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    result = run(options.binary.resolve(), options.root.resolve())
    (options.root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
