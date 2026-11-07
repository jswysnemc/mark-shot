#!/usr/bin/env python3
"""【OCR 界面语言】【真实验证】仅在私有 Wayland 中验证中文界面、目标稳定值、菜单和失败期限，不初始化模型或外部服务"""

import argparse
import hashlib
import json
from pathlib import Path
import re
import select
import subprocess
import time
import tomllib

from PIL import ImageChops
import wayland_text_format_session as session
from wayland_input_method_transport import InputMethod, build, protocol_evidence
from wayland_ocr_translation_checks import configure, idle, point, popup, reclaimed, requests
from wayland_ocr_translation_input_checks import Checks
from wayland_ocr_window_layout import source_point, tab_point, table_point
from wayland_text_pointer import click, pointer_context
from wayland_window_input_session import ROWS, frame, geometry, newer_state, screenshot


def preferences(root, environment, source):
    """【OCR 界面语言】【私有配置】按 root、environment、source 设置本次语言来源，返回配置路径；不读取用户配置或凭据"""
    config = configure(root)
    text = config.read_text()
    environment.pop("MARK_SHOT_LANG", None)
    environment.update(LC_ALL="C.UTF-8", LANG="en_US.UTF-8", LC_MESSAGES="en_US.UTF-8")
    if source == "legacy":
        text = "language = 'chinese'\n" + text
    elif source == "environment":
        text += "\n[ui]\nlanguage = 'english'\n"
        environment["MARK_SHOT_LANG"] = "zh_TW"
    elif source == "system":
        text += "\n[ui]\nlanguage = 'system'\n"
        environment["LC_ALL"] = "zh_CN.UTF-8"
    else:
        text += "\n[ui]\nlanguage = 'chinese'\n"
    config.write_text(text)
    (root / "mode").write_text("success")
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    return config


def launch(binary, environment, output):
    """【OCR 界面语言】【真实启动】按 binary、environment 启动真实窗口，output 保存协议；返回已完成首帧的进程，失败回收本次子进程"""
    process = subprocess.Popen([str(binary), "--ocr-result", "--nonotify"], env=dict(environment, WAYLAND_DEBUG="client"), stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=output)
    try:
        process.stdin.write(frame())
        process.stdin.close()
        assert select.select([process.stdout], [], [], 8)[0] and process.stdout.readline() == b"\n"
        return process
    except BaseException:
        if process.poll() is None:
            process.terminate()
        process.wait(timeout=3)
        raise


def run(binary, root, xml, source):
    """【OCR 界面语言】【完整场景】按 binary、root、xml、source 验证本次私有中文窗口，保存真实输入、菜单和任务证据，无返回值"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    config = preferences(root, environment, source)
    client = build(root / "client", xml)
    log = root / "process.log"
    with session.input_context(), pointer_context(), log.open("w") as output:
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        method = InputMethod(client, environment, root)
        process = None
        try:
            process = launch(binary, environment, output)
            window = geometry(process.pid, environment)
            assert re.search(r'set_title\(Some\("OCR 结果"\)\)', log.read_text()), "native window did not use the Chinese title"
            checks = Checks(method, window, root, log, environment)
            # 1. 【OCR 界面语言】【业务隔离】界面语言不转换识别正文、输入法确认文字和复制数据
            checks.focus("source", "base")
            checks.batch("base", preedit="业务")
            screenshot(root, "chinese-preedit")
            checks.full_copy("source-excludes-preedit", "base")
            text = "base业务 Copy %1"
            checks.batch(text, commit="业务 Copy %1")
            checks.full_copy("source-business-value", text)
            screenshot(root, "chinese-titlebar")
            click(source_point(window))
            screenshot(root, "chinese-copied-feedback")
            # 2. 【OCR 界面语言】【目标稳定值】中文预设只影响标签，真实命令始终接收稳定语言名称
            click(point(window, "toggle"))
            first = requests(root, 1)[0]
            assert first["target"] == "English" and first["source"]["tokens"][0]["text"] == text
            reclaimed(first)
            checks.focus("translation", "translated " + text)
            checks.full_copy("translation-business-value", "translated " + text)
            checks.focus("language", "英语")
            click(point(window, "arrow"))
            popup(log, 1)
            screenshot(root, "chinese-language-options")
            session.key("Home")
            for _ in range(3):
                session.key("Down")
            session.key("Return")
            popup(log, 0)
            checks.focus("language", "日语")
            checks.select_all("日语")
            session.key("ctrl+c")
            session.wait_clipboard(environment, "日语")
            checks.clipboard["selected-language-label"] = session.read_clipboard(environment)
            assert tomllib.loads(config.read_text())["translation"]["targetLanguage"] == "Japanese"
            click(point(window, "action"))
            second = requests(root, 2)[1]
            assert second["target"] == "Japanese" and second["source"]["tokens"][0]["text"] == text
            reclaimed(second)
            checks.focus("source", text)
            screenshot(root, "chinese-translation")
            # 3. 【OCR 界面语言】【菜单呈现】快捷键子菜单、表格格式和原图菜单使用真实独立表面
            click((window["x"] + window["width"] - 85, window["y"] + 19))
            popup(log, 1)
            session.key("Down")
            session.key("Right")
            popup(log, 2)
            screenshot(root, "chinese-shortcuts")
            session.key("Escape")
            session.key("Escape")
            click(tab_point(window, "Table", language="zh"))
            click(table_point(window))
            popup(log, 1)
            screenshot(root, "chinese-table-formats")
            session.key("Home")
            session.key("Return")
            session.wait_clipboard(environment, "\n".join("\t".join(row) for row in ROWS))
            click(tab_point(window, "Image", language="zh"))
            click((window["x"] + window["width"] / 2, window["y"] + 160), "right")
            popup(log, 1)
            screenshot(root, "chinese-image-menu")
            session.key("Escape")
            click(tab_point(window, "Text", language="zh"))
            checks.focus("source", text)
            previous = method.states[-1]["serial"]
            session.key("ctrl+z")
            newer_state(method, previous, "base")
            previous = method.states[-1]["serial"]
            session.key("ctrl+shift+z")
            newer_state(method, previous, text)
            checks.full_copy("source-history-preserved", text)
            # 4. 【OCR 界面语言】【失败反馈】私有保存失败显示中文且按原期限消失，不切换角色或改写非法文件
            pin = (window["x"] + window["width"] - 53, window["y"] + 19)
            click(pin, "right")
            time.sleep(0.08)
            before = screenshot(root, "before-save-failure")
            healthy = config.read_text()
            invalid = healthy + "\n[ocr_result_window]\nalways_on_top = 'invalid'\n"
            config.write_text(invalid)
            click(pin)
            time.sleep(0.15)
            assert config.read_text() == invalid
            failed = screenshot(root, "chinese-save-failed")
            footer = (window["x"], window["y"] + window["height"] - 26, window["x"] + window["width"], window["y"] + window["height"])
            assert ImageChops.difference(before.crop(footer), failed.crop(footer)).getbbox() is not None
            time.sleep(2.6)
            expired = screenshot(root, "chinese-failure-expired")
            assert ImageChops.difference(before.crop(footer), expired.crop(footer)).getbbox() is None
            config.write_text(healthy)
            resources = idle(process.pid, log)
            assert not (root / "data/mark-shot/models").exists()
            evidence = protocol_evidence(log.read_text())
            assert evidence["native_protocol"] and evidence["preedit_delivered"]
            session.key("Escape")
            assert process.wait(timeout=4) == 0
            result = {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "compositor": "sway" if session.backend() is session.sway else "niri",
                      "language_source": source, "chinese_native_title": True, "source_and_translation_values_unchanged": True,
                      "localized_language_labels_with_stable_saved_and_request_values": True, "requests": [record["target"] for record in requests(root, 2)],
                      "source_history_preserved": True, "native_menus": True, "localized_failure_expired": True, "resources": resources,
                      "clipboard": checks.clipboard, "no_ocr_or_external_service": True, **evidence}
            (root / "result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
        finally:
            if process and process.poll() is None:
                process.terminate()
                process.wait(timeout=3)
            method.close()


def main():
    """【OCR 界面语言】【运行入口】解析本地二进制、私有目录、协议和语言来源，执行场景后失败返回非零状态，无参数"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--xml", type=Path, default=Path("target/test-tools/input-method/input-method-unstable-v2.xml"))
    parser.add_argument("--language-source", choices=["config", "legacy", "environment", "system"], default="config")
    args = parser.parse_args()
    run(args.binary.resolve(), args.root.resolve(), args.xml.resolve(), args.language_source)


if __name__ == "__main__":
    main()
