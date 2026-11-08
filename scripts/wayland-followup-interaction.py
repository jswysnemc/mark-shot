#!/usr/bin/env python3
"""【交互回归】【原生业务】私有 Sway 检查光标协议、实时颜色、格式复制和设置后的真实保存"""

import argparse
import hashlib
import importlib.util
import json
import re
import subprocess
import time
import tomllib
from pathlib import Path

import wayland_text_format_session as session
from PIL import Image, ImageChops
from wayland_text_pointer import click, pointer_context

SPEC = importlib.util.spec_from_file_location("support", Path(__file__).with_name("wayland-memory-baseline.py"))
SUPPORT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SUPPORT)


def color_bounds(image, color):
    """【交互回归】【图像定位】按 image 中 color 的精确像素返回色块边界，不依赖窗口初始缩放"""
    channels = image.convert("RGB").split()
    mask = channels[0].point(lambda value: 255 if value == color[0] else 0)
    for channel, expected in zip(channels[1:], color[1:]):
        mask = ImageChops.multiply(mask, channel.point(lambda value, expected=expected: 255 if value == expected else 0))
    bounds = mask.getbbox()
    assert bounds, f"missing source color {color}"
    return bounds


def run(binary, root):
    """【交互回归】【场景执行】按 binary 和 root 执行隔离编辑会话，返回严格断言后的结果"""
    root.mkdir(parents=True, exist_ok=False)
    original = Image.new("RGB", (600, 340), (32, 67, 92))
    original.paste((120, 180, 40), (300, 0, 600, 340))
    source = root / "source.png"
    original.save(source)
    config = root / "config.toml"
    config.write_text('[ui]\nlanguage="chinese"\n[history]\nenabled=false\n[notifications]\nenabled=false\n[custom]\nkeep=17\n')
    environment = dict(session.environment(), XDG_CONFIG_HOME=str(root / "config-home"), XDG_STATE_HOME=str(root / "state"), WAYLAND_DEBUG="client", MARK_SHOT_LANG="chinese")
    log_path = root / "client.log"
    with session.input_context(), pointer_context() as pointer, log_path.open("w") as log:
        session.sway.keyboard_ready()
        process = subprocess.Popen([str(binary), "--config", str(config), str(source)], env=environment, stdout=subprocess.DEVNULL, stderr=log)
        try:
            def window(app_id):
                """【交互回归】【窗口查询】按 app_id 返回当前测试进程窗口，只使用私有桌面"""
                assert process.poll() is None, "editor exited before completing the interaction"
                return next((node for node in SUPPORT.SCROLL.tree(environment) if node.get("pid") == process.pid and node.get("app_id") == app_id), None)

            def move(point):
                """【交互回归】【指针移动】按 point 向私有 1280×800 输出发送移动并等待事件交付"""
                pointer.move_abs(*point, 1280, 800)
                time.sleep(0.12)

            def cursor(point):
                """【交互回归】【光标协议】移动到 point 后返回最后一次真实光标形状请求"""
                move(point)
                shapes = re.findall(r"cursor_shape_device_v1@\d+\.set_shape\(\d+, (\d+)\)", log_path.read_text())
                assert shapes, "no native cursor shape requests"
                return int(shapes[-1])

            def clipboard():
                """【交互回归】【剪贴板读取】无参数，读取应用实际写入的文本，不使用界面标签推断复制结果"""
                return subprocess.check_output(["wl-paste", "--no-newline", "--type", "text/plain"], env=environment, timeout=4).decode()

            shown = SUPPORT.SCROLL.wait(lambda: window("mark-shot-editor"), "editor did not open")
            SUPPORT.SCROLL.command(environment, f'[con_id={shown["id"]}] focus, fullscreen enable')
            time.sleep(0.5)
            initial = SUPPORT.raw_capture()
            initial.save(root / "initial.png")
            blue = color_bounds(initial, (32, 67, 92))
            green = color_bounds(initial, (120, 180, 40))
            sample_blue = (blue[0] + 50, blue[1] + 60)
            sample_green = (green[0] + 60, green[1] + 60)
            left, right, bottom = blue[0] - 1, green[2] + 1, green[3] + 1
            session.key("p")
            resize = cursor(((left + right) // 2, bottom))
            assert resize not in (1, 4, 8), f"handle did not request resize: {resize}"
            assert cursor(((left + right) // 2, bottom + 18)) == 1, "outside selection did not restore visible arrow"
            assert cursor((left + 35, bottom)) in (1, 8), "ordinary edge incorrectly requested resize"
            session.key("i")
            move(sample_blue)
            session.key("ctrl+c")
            SUPPORT.SCROLL.wait(lambda: clipboard() == "#20435C", "live blue sample was not copied")
            move(sample_green)
            session.key("ctrl+c")
            SUPPORT.SCROLL.wait(lambda: clipboard() == "#78B428", "live green sample did not update")
            move(sample_blue)
            click(sample_blue)
            time.sleep(0.2)
            panel_x = sample_blue[0] + 20 if sample_blue[0] + 446 < 1280 else max(8, sample_blue[0] - 446)
            panel_y = min(sample_blue[1] + 20, 394)
            SUPPORT.raw_capture().save(root / "color-inspector.png")
            formats = ["#20435C", "#20435c", "#20435CFF", "rgb(32, 67, 92)", "rgba(32, 67, 92, 1)", "hsl(205, 48%, 24%)", "hsv(205, 65%, 36%)", "Qt.rgba(0.125, 0.263, 0.361, 1)"]
            for index, expected in enumerate(formats):
                click((panel_x + 391, panel_y + 124 + index * 33.25 + 14))
                SUPPORT.SCROLL.wait(lambda expected=expected: clipboard() == expected, f"wrong copied format {expected}")
                assert process.poll() is None, "copying a color discarded the editor"
            session.key("Escape")
            session.key("v")
            session.key("ctrl+comma")
            settings = SUPPORT.SCROLL.wait(lambda: window("mark-shot-settings"), "settings did not open")
            SUPPORT.SCROLL.command(environment, f'[con_id={settings["id"]}] floating enable, resize set width 900 px height 640 px, move position center, focus')
            time.sleep(0.3)

            def settings_click(horizontal, vertical):
                """【设置业务回归】【局部操作】按 horizontal、vertical 点击当前设置窗口控件"""
                bounds = window("mark-shot-settings")["rect"]
                click((bounds["x"] + horizontal, bounds["y"] + vertical))
                time.sleep(0.1)

            def set_text(row, text):
                """【设置业务回归】【字段输入】将 text 输入 row 对应真实设置字段并提交"""
                settings_click(740, 165 + row * 68)
                session.key("ctrl+a")
                session.paste_text(text)
                session.key("Return")
                time.sleep(0.1)

            settings_click(75, 153)
            destination = root / "saved-from-settings"
            set_text(1, str(destination))
            set_text(2, "verified-capture")
            SUPPORT.SCROLL.command(environment, f'[con_id={settings["id"]}] resize set width 580 px height 420 px')
            time.sleep(0.2)
            bounds = window("mark-shot-settings")["rect"]
            move((bounds["x"] + 566, bounds["y"] + 153))
            pointer.press()
            move((bounds["x"] + 566, bounds["y"] + 348))
            pointer.release()
            time.sleep(0.15)
            SUPPORT.raw_capture().save(root / "settings-scrolled.png")
            settings_click(455, 301)
            session.key("ctrl+a")
            session.paste_text("verified-month")
            session.key("Return")
            data = tomllib.loads(config.read_text())
            assert data["save"]["month_format"] == "verified-month", "scrollbar did not expose the last setting"
            SUPPORT.SCROLL.command(environment, f'[con_id={settings["id"]}] resize set width 900 px height 640 px')
            time.sleep(0.2)
            settings_click(75, 198)
            settings_click(740, 233)
            session.key("ctrl+alt+s")
            time.sleep(0.2)
            assert "save" not in tomllib.loads(config.read_text()).get("keys", {}), "conflicting shortcut overwrote the binding"
            SUPPORT.raw_capture().save(root / "settings-conflict.png")
            session.key("ctrl+alt+shift+s")
            time.sleep(0.2)
            assert tomllib.loads(config.read_text())["keys"]["save"] == "Ctrl+Alt+Shift+S"
            SUPPORT.raw_capture().save(root / "settings-shortcut.png")
            settings_click(860, 39)
            SUPPORT.SCROLL.wait(lambda: window("mark-shot-editor"), "editor did not return from settings")
            time.sleep(0.3)
            session.key("ctrl+s")
            time.sleep(0.15)
            assert process.poll() is None, "old save shortcut remained active"
            session.key("ctrl+alt+shift+s")
            assert process.wait(timeout=10) == 0, "new shortcut did not finish saving"
            saved = destination / "verified-month" / "verified-capture.png"
            assert saved.is_file(), "current editor ignored the new save location or filename"
            result = Image.open(saved)
            assert result.size == original.size, "settings roundtrip changed image dimensions"
            assert ImageChops.difference(result.convert("RGB"), original).getbbox(), "settings discarded the color annotation"
            assert tomllib.loads(config.read_text())["custom"]["keep"] == 17
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)
    report = {"binary": str(binary), "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "native_cursor": True, "live_sampling": True, "copied_formats": formats, "scrollbar_drag": True, "shortcut_conflict_rejected": True, "runtime_shortcut": True, "runtime_save": str(saved), "passed": True}
    (root / "result.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(report, ensure_ascii=False, indent=2))


def main():
    """【交互回归】【参数入口】解析 binary、root 并运行私有桌面验证，无参数"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    run(options.binary.resolve(), options.root.resolve())


if __name__ == "__main__":
    main()
