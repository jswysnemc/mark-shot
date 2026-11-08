#!/usr/bin/env python3
"""【文字属性回归】【真实输入】验证字体选择、字号即时应用、默认隔离和原图输出，只连接私有 Wayland 会话"""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import tomllib

from PIL import Image, ImageChops
import wayland_test_session as session
from wayland_text_property_locator import font_button, font_popup

SOURCE = (30, 34, 38)


def capture(process, root, name):
    """【文字属性回归】【缓冲记录】保存 process 的 name 截图到 root，返回 RGB 图，进程提前退出抛出异常"""
    time.sleep(0.2)
    if process.poll() is not None:
        raise AssertionError(f"the editor exited unexpectedly: {process.returncode}")
    path = root / f"{name}.png"
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def click(point, button="left"):
    """【文字属性回归】【按钮点击】向私有会话 point 发送 button 完整点击，无返回值"""
    subprocess.run(["wtype", "-m", "ctrl", "-m", "alt", "-m", "shift", "-m", "logo"], env=session.environment(), check=True)
    session.control("click", "--logical", *map(round, point), "--button", button, "--hold", "0.1")
    time.sleep(0.04)


def copied_size(environment, expected):
    """【文字属性回归】【字号复制】从当前真实字号字段复制并验证 expected，environment 指向私有会话，无返回值"""
    session.key("ctrl+a")
    session.key("ctrl+c")
    session.wait_clipboard(environment, expected + " pt")


def wait_idle(pid):
    """【文字属性回归】【空闲等待】等待 pid 的线程数和 CPU 时钟稳定；返回资源快照，允许按需创建的渲染线程池"""
    previous = session.resources(pid)
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        time.sleep(0.3)
        current = session.resources(pid)
        if current["threads"] == previous["threads"] and current["cpu_ticks"] == previous["cpu_ticks"]:
            return current
        previous = current
    raise AssertionError(f"the editor did not become idle: {previous}")


def scenario(binary, root):
    """【文字属性回归】【业务场景】用 binary 验证字体弹层和已有文字属性，root 保存证据，返回资源与输出数据"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    with tempfile.TemporaryDirectory(prefix="mark-shot-font-test-") as directory:
        private = Path(directory)
        config, state = private / "config/mark-shot", private / "state/mark-shot"
        config.mkdir(parents=True)
        state.mkdir(parents=True)
        (config / "config.toml").write_text('[general]\nto = "s"\n[notifications]\nenabled = false\n')
        (state / "tools.toml").write_text('magnifier = false\nfont_size = 20.0\nfont_size_unit = "pt"\ntext_family = "Sans Serif"\nbold = true\nitalic = false\noutline = false\n')
        environment.update(XDG_CONFIG_HOME=str(private / "config"), XDG_DATA_HOME=str(private / "data"), XDG_STATE_HOME=str(private / "state"))
        source = private / "source.png"
        Image.new("RGBA", (1600, 900), (*SOURCE, 255)).save(source)
        with (root / "process.log").open("w") as log:
            process = subprocess.Popen([str(binary), "--fullscreen", "--fullscreen-default-tool", "text", "--default-color", "#FF4D4D", "--to", "s", "--output", str(private / "export"), str(source)], env=environment, stderr=log)
            try:
                wait_idle(process.pid)
                deadline = time.monotonic() + 5
                while time.monotonic() < deadline:
                    window = next((item for item in session.windows() if item["pid"] == process.pid), None)
                    if window:
                        session.control("action", "fullscreen-window", "--id", window["id"])
                        break
                    time.sleep(0.05)
                else:
                    raise AssertionError("the image editor window did not appear")
                time.sleep(1)
                initial = capture(process, root, "initial")
                wait_idle(process.pid)
                button = font_button(initial)
                click(button, "right")
                protected = capture(process, root, "right-button")
                try:
                    font_popup(protected)
                except AssertionError:
                    pass
                else:
                    raise AssertionError("right click opened the font panel")
                click(button)
                opened = capture(process, root, "font-open")
                left, top, _right, _bottom = font_popup(opened)
                font_idle_before = wait_idle(process.pid)
                time.sleep(1)
                font_idle_after = session.resources(process.pid)
                if font_idle_before["cpu_ticks"] != font_idle_after["cpu_ticks"] or font_idle_after["threads"] != font_idle_before["threads"]:
                    raise AssertionError(f"the open font panel was not event driven: {font_idle_before}, {font_idle_after}")
                session.paste_text("Liberation Sans")
                capture(process, root, "font-search")
                session.key("Tab")
                session.paste_text("42")
                copied_size(environment, "42")
                session.key("Return")
                size = capture(process, root, "size-confirmed")
                # 1. 【文字属性回归】【字段焦点】字号确认不关闭弹层，也不触发截图导出
                if max(abs(first - second) for first, second in zip(font_popup(size), (left, top, left + 360, top + 360))) > 2:
                    raise AssertionError("size Enter closed or moved the font panel")
                click((left + 311, top + 339))
                capture(process, root, "bold-off")
                session.key("Tab")
                session.paste_text("Liberation Sans")
                filtered = capture(process, root, "family-filtered")
                left, top, _right, _bottom = font_popup(filtered)
                click((left + 55, top + 54))
                chosen = capture(process, root, "family-chosen")
                font_popup(chosen)
                copied_size(environment, "42")
                session.key("Escape")
                # 2. 【文字属性回归】【默认与对象】新文字先使用四十二点，已有文字改为二十点时同步默认字号
                point = (round(initial.width * 0.58), round(initial.height * 0.6))
                click(point)
                session.paste_text("Native text")
                text = capture(process, root, "text-created")
                click(font_button(text))
                opened = capture(process, root, "object-font-open")
                left, top, _right, _bottom = font_popup(opened)
                session.key("Tab")
                copied_size(environment, "42")
                session.paste_text("20")
                copied_size(environment, "20")
                capture(process, root, "object-size-twenty")
                session.key("Escape")
                capture(process, root, "object-font-closed")
                click((point[0] - 100, point[1] + 140))
                capture(process, root, "committed")
                before = wait_idle(process.pid)
                time.sleep(1)
                after = session.resources(process.pid)
                session.key("Return")
                process.wait(timeout=6)
                if process.returncode:
                    raise AssertionError(f"text export failed: {process.returncode}")
                files = list((private / "export").glob("*.png"))
                if len(files) != 1:
                    raise AssertionError(f"unexpected export count: {files}")
                shutil.copyfile(files[0], root / "export.png")
                shutil.copyfile(state / "tools.toml", root / "tools.toml")
                saved = tomllib.loads((state / "tools.toml").read_text())
                if saved["font_size"] != 20 or saved["text_family"] != "Liberation Sans" or saved["bold"]:
                    raise AssertionError(f"size did not synchronize defaults or changed unrelated styles: {saved}")
                with Image.open(files[0]) as image:
                    output = image.convert("RGB")
                bounds = ImageChops.difference(output, Image.new("RGB", output.size, SOURCE)).getbbox()
                if output.size != (1600, 900) or not bounds or not 15 <= bounds[3] - bounds[1] <= 32:
                    raise AssertionError(f"the text did not use the object font size or original image: {output.size}, {bounds}")
                if before["cpu_ticks"] != after["cpu_ticks"] or after["threads"] != before["threads"]:
                    raise AssertionError(f"idle editing was not event driven: {before}, {after}")
                if (private / "data/mark-shot/models").exists():
                    raise AssertionError("font selection initialized OCR models")
                return {"right_button_protected": True, "size_enter_preserves_panel": True, "family_keeps_focus": True, "default_points": 20, "object_points": 20, "family": saved["text_family"], "bold": saved["bold"], "export_size": list(output.size), "pixel_bounds": bounds, "font_idle_before": font_idle_before, "font_idle_after": font_idle_after, "idle_before": before, "idle_after": after}
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=3)


def main():
    """【文字属性回归】【命令入口】读取 binary 和 artifacts，记录同一构建的输入、输出与资源证据，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, required=True)
    options = parser.parse_args()
    binary = options.binary.resolve()
    result = {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "scenario": scenario(binary, options.artifacts.resolve())}
    (options.artifacts / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
