#!/usr/bin/env python3
"""【工具栏回归】【原生窗口】在私有 Wayland 会话验证配置交互并保留截图"""

import argparse
import importlib.util
import json
from pathlib import Path
import subprocess
import time
import tomllib

import wayland_text_format_session as session
from wayland_text_pointer import pointer_context

SPEC = importlib.util.spec_from_file_location("support", Path(__file__).with_name("wayland-memory-baseline.py"))
SUPPORT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SUPPORT)


def run(binary, root):
    """【工具栏回归】【场景执行】用 binary 在 root 创建独立配置，验证保存结果并输出报告，无返回值"""
    root.mkdir(parents=True, exist_ok=False)
    config = root / "config.toml"
    initial = ["rectangle", "circle", "separator", "copy", "save", "settings", "cancel"]
    config.write_text('[ui]\nlanguage="chinese"\ntheme="light"\n[custom]\nkeep=17\n'
                      '[toolbar_layout]\nfullscreen=' + json.dumps(initial) + '\n')
    environment = dict(session.environment(), MARK_SHOT_LANG="chinese", WAYLAND_DEBUG="client")
    with session.input_context(), pointer_context(), (root / "client.log").open("w") as log:
        session.sway.keyboard_ready()
        process = subprocess.Popen([str(binary), "--config", str(config), "--settings"], env=environment,
                                   stdout=subprocess.DEVNULL, stderr=log)
        try:
            def window():
                """【工具栏回归】【窗口定位】无参数，返回本次设置进程窗口，退出时报告失败"""
                assert process.poll() is None, "settings window exited unexpectedly"
                return next((node for node in SUPPORT.SCROLL.tree(environment)
                             if node.get("pid") == process.pid and node.get("app_id") == "mark-shot-settings"), None)

            shown = SUPPORT.SCROLL.wait(window, "settings window did not open")
            identifier = shown["id"]
            SUPPORT.SCROLL.command(environment, f'[con_id={identifier}] floating enable, resize set width 900 px height 640 px, move position center, focus')
            time.sleep(0.3)

            def click(horizontal, vertical):
                """【工具栏回归】【局部点击】将 horizontal、vertical 转为窗口全局坐标并点击，无返回值"""
                bounds = window()["rect"]
                SUPPORT.SCROLL.INPUT.click((bounds["x"] + horizontal, bounds["y"] + vertical))
                time.sleep(0.12)

            def values():
                """【工具栏回归】【配置读取】无参数，返回本次独立文件的持久化配置"""
                return tomllib.loads(config.read_text())

            def layout():
                """【工具栏回归】【布局读取】无参数，返回当前图片工具栏的已保存按钮顺序"""
                return values()["toolbar_layout"]["fullscreen"]

            click(70, 330)
            SUPPORT.raw_capture().save(root / "toolbar-light.png")
            click(270, 430)
            SUPPORT.SCROLL.wait(lambda: layout() == initial + ["selection"], "candidate button was not added")
            session.key("Delete")
            SUPPORT.SCROLL.wait(lambda: layout() == initial, "selected button was not removed")
            click(225, 241)
            session.key("alt+Right")
            expected = [initial[1], initial[0], *initial[2:]]
            SUPPORT.SCROLL.wait(lambda: layout() == expected, "button reorder was not saved")
            click(784, 320)
            SUPPORT.SCROLL.wait(lambda: values()["toolbar_layout"].get("fullscreen_vertical") is True,
                                "orientation was not saved")
            click(327, 241)
            click(310, 354)
            session.key("ctrl+alt+c")
            SUPPORT.SCROLL.wait(lambda: values().get("keys", {}).get("copy") == "Ctrl+Alt+C", "shortcut was not recorded")
            click(532, 354)
            SUPPORT.SCROLL.wait(lambda: values()["keys"]["copy"] == "", "shortcut was not cleared")
            click(756, 354)
            SUPPORT.SCROLL.wait(lambda: values()["keys"]["copy"] == ["Ctrl+C"], "default shortcut was not restored")
            click(812, 197)
            SUPPORT.SCROLL.wait(lambda: len(layout()) > len(initial), "default buttons were not restored")
            SUPPORT.raw_capture().save(root / "toolbar-default.png")
            click(70, 110)
            click(740, 233)
            click(740, 304)
            SUPPORT.SCROLL.wait(lambda: values()["ui"]["theme"] == "dark", "dark theme was not saved")
            click(70, 330)
            SUPPORT.raw_capture().save(root / "toolbar-dark.png")
            SUPPORT.SCROLL.command(environment, f'[con_id={identifier}] resize set width 580 px height 420 px')
            time.sleep(0.3)
            SUPPORT.raw_capture().save(root / "toolbar-narrow.png")
            session.key("End")
            time.sleep(0.15)
            SUPPORT.raw_capture().save(root / "toolbar-narrow-bottom.png")
            assert values()["custom"]["keep"] == 17
            click(540, 39)
            assert process.wait(timeout=5) == 0
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)
    report = dict(passed=True, add_remove=True, reorder=True, orientation=True, shortcut=True,
                  reset=True, themes=True, narrow_window=True, unknown_fields_preserved=True)
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


def main():
    """【工具栏回归】【运行入口】无参数，读取二进制和结果目录选项并执行检查，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    run(options.binary.resolve(), options.root.resolve())


if __name__ == "__main__":
    main()
