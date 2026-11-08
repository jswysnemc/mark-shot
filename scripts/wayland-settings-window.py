#!/usr/bin/env python3
"""【设置窗口回归】【原生交互】在私有桌面验证分类、开关、路径和明暗主题，配置只写入本次目录"""

import argparse
import hashlib
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
    """【设置窗口回归】【场景执行】按 binary 在 root 的独立配置启动窗口，返回保存和尺寸验证结果"""
    root.mkdir(parents=True, exist_ok=False)
    config = root / "config.toml"
    config.write_text('[ui]\nlanguage="chinese"\ntheme="light"\n[notifications]\nenabled=true\n[custom]\nkeep=17\n')
    environment = dict(session.environment(), XDG_CONFIG_HOME=str(root / "config-home"), XDG_STATE_HOME=str(root / "state"), MARK_SHOT_LANG="chinese", WAYLAND_DEBUG="client")
    with session.input_context(), pointer_context(), (root / "client.log").open("w") as log:
        session.sway.keyboard_ready()
        process = subprocess.Popen([str(binary), "--config", str(config), "--settings"], env=environment, stdout=subprocess.DEVNULL, stderr=log)
        try:
            def window():
                """【设置窗口回归】【窗口定位】无参数，返回本次设置进程唯一窗口，退出或启动失败立即报告"""
                assert process.poll() is None, "settings window exited unexpectedly"
                return next((node for node in SUPPORT.SCROLL.tree(environment) if node.get("pid") == process.pid and node.get("app_id") == "mark-shot-settings"), None)

            shown = SUPPORT.SCROLL.wait(window, "settings window did not open")
            identifier = shown["id"]
            SUPPORT.SCROLL.command(environment, f'[con_id={identifier}] floating enable, resize set width 900 px height 640 px, move position center, focus')
            time.sleep(0.4)

            def click(horizontal, vertical):
                """【设置窗口回归】【局部点击】按 horizontal、vertical 转换本次窗口坐标并完成左键，无返回值"""
                bounds = window()["rect"]
                SUPPORT.SCROLL.INPUT.click((bounds["x"] + horizontal, bounds["y"] + vertical))
                time.sleep(0.12)

            def values():
                """【设置窗口回归】【配置读取】无参数，读取本次独立配置的实际持久化值"""
                return tomllib.loads(config.read_text())

            SUPPORT.raw_capture().save(root / "appearance-light.png")
            click(760, 300)
            assert values()["notifications"]["enabled"] is False
            click(75, 153)
            click(735, 233)
            session.key("ctrl+a")
            session.paste_text(str(root / "screenshots"))
            session.key("Return")
            SUPPORT.SCROLL.wait(lambda: values().get("save", {}).get("directory") == str(root / "screenshots"), "save directory was not persisted")
            SUPPORT.raw_capture().save(root / "capture.png")
            click(75, 198)
            click(740, 165)
            session.key("ctrl+alt+c")
            SUPPORT.SCROLL.wait(lambda: values().get("keys", {}).get("copy") == "Ctrl+Alt+C", "shortcut recording failed")
            SUPPORT.raw_capture().save(root / "shortcuts.png")
            click(75, 108)
            click(740, 233)
            click(740, 304)
            SUPPORT.SCROLL.wait(lambda: values()["ui"]["theme"] == "dark", "theme was not persisted")
            SUPPORT.raw_capture().save(root / "appearance-dark.png")
            SUPPORT.SCROLL.command(environment, f'[con_id={identifier}] resize set width 600 px height 480 px')
            time.sleep(0.25)
            session.key("ctrl+f")
            session.paste_text("copy")
            time.sleep(0.15)
            SUPPORT.raw_capture().save(root / "search-narrow.png")
            assert values()["custom"]["keep"] == 17
            click(560, 39)
            assert process.wait(timeout=5) == 0
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)
    report = dict(binary=str(binary), sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), passed=True,
                  notification_toggle=True, directory_saved=True, shortcut_recorded=True, theme_saved=True, unknown_fields_preserved=True)
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


def main():
    """【设置窗口回归】【参数入口】解析二进制及私有目录并执行验证，无参数"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    run(options.binary.resolve(), options.root.resolve())


if __name__ == "__main__":
    main()
