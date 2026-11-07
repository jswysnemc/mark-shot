#!/usr/bin/env python3
"""【OCR 翻译回归】【真实会话】验证私有 Wayland 的分栏、独立编辑、取消、语言列表与关闭资源，不启动模型或外部服务"""

import argparse
import hashlib
import json
from pathlib import Path
import select
import subprocess
import time
import tomllib

from PIL import Image, ImageChops
import wayland_text_format_session as session
from wayland_ocr_popup_tree import popup_tree
from wayland_ocr_translation_checks import configure, idle, notice_rows, point, popup, reclaimed, requests
from wayland_ocr_window_layout import source_point, tab_point
from wayland_text_pointer import CURRENT, click, pointer_context
from wayland_window_input_session import frame, geometry


def snapshot(root, name):
    """【OCR 翻译回归】【真实像素】在 root 保存 name 画面并返回 RGB 像素，不从内部状态替代呈现证据"""
    path = root / (name + ".png")
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def edit(window, field, text):
    """【OCR 翻译回归】【实际编辑】在 window 的 field 通过真实选择和粘贴写入 text，无返回值"""
    click(point(window, field))
    session.key("ctrl+a")
    session.paste_text(text)


def copy_translation(environment, window, text):
    """【OCR 翻译回归】【独立复制】点击 window 译文全文按钮并等待 environment 剪贴板匹配 text，无返回值"""
    click(point(window, "copy"))
    session.wait_clipboard(environment, text)


def start(binary, root, environment, output):
    """【OCR 翻译回归】【实际窗口】用私有 environment 启动 binary 并注入固定结果，root 保存证据，output 接收日志，返回进程及矩形"""
    process = subprocess.Popen([str(binary), "--ocr-result", "--nonotify"], env=dict(environment, WAYLAND_DEBUG="client"), stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=output)
    try:
        process.stdin.write(frame())
        process.stdin.close()
        assert select.select([process.stdout], [], [], 5)[0] and process.stdout.readline() == b"\n"
        return process, geometry(process.pid, environment)
    except BaseException:
        if process.poll() is None:
            process.terminate()
        process.wait(timeout=3)
        raise


def scenario(binary, root, close_after_cancel=False, close_only=False):
    """【OCR 翻译回归】【完整交互】在私有会话运行 binary，root 保存证据；close_only 和 close_after_cancel 指定退出边界，返回回归结果"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    config = configure(root)
    log = root / "process.log"
    pointer_backend = "virtual"
    with session.input_context(), pointer_context(backend=pointer_backend), log.open("w") as output:
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        process, window = start(binary, root, environment, output)
        try:
            snapshot(root, "initial")
            click(point(window, "toggle"))
            record = requests(root, 1)[0]
            assert record["source"]["tokens"][0]["text"] == "base" and record["target"] == "English"
            snapshot(root, "busy")
            if close_only:
                if close_after_cancel:
                    click(point(window, "action"))
                session.key("Escape")
                assert process.wait(timeout=3) == 0
                assert not Path(f"/proc/{record['pid']}").exists(), "app exited before reaping its translation command"
                assert not Path(record["input"]).exists(), "app exited before deleting its private input"
                reclaimed(record)
                return {"closed_after_cancel": close_after_cancel, "direct_child_and_input_absent_on_app_exit": True, "descendant_stopped": True}
            # 1. 【OCR 翻译回归】【忙碌隔离】原文继续编辑，译文粘贴拒绝，请求仍持有原来的确认快照
            edit(window, "source", "edited source")
            click(source_point(window))
            session.wait_clipboard(environment, "edited source")
            edit(window, "translation", "forbidden")
            snapshot(root, "busy-source-edited")
            assert requests(root, 1)[0]["source"]["tokens"][0]["text"] == "base"
            click(point(window, "action"))
            reclaimed(record)
            snapshot(root, "canceled")
            canceled_idle = idle(process.pid, log)
            # 2. 【OCR 翻译回归】【实际完成】快捷键重新请求修改后的原文，完成后只编辑译文并核对独立撤销
            (root / "mode").write_text("success")
            click(point(window, "source"))
            session.key("ctrl+Return")
            record = requests(root, 2)[1]
            assert record["source"]["tokens"][0]["text"] == "edited source"
            reclaimed(record)
            time.sleep(0.12)
            copy_translation(environment, window, "translated edited source")
            edit(window, "translation", "manual translation")
            copy_translation(environment, window, "manual translation")
            click(point(window, "translation"))
            session.key("ctrl+z")
            copy_translation(environment, window, "translated edited source")
            edit(window, "translation", "manual translation")
            click(source_point(window))
            session.wait_clipboard(environment, "edited source")
            # 3. 【OCR 翻译回归】【焦点保留】切页恢复译文焦点，快捷复制仍归译文；重开已有译文不创建第三次请求
            click(point(window, "translation"))
            click(tab_point(window, "Image"))
            click(tab_point(window, "Text"))
            session.key("ctrl+shift+c")
            session.wait_clipboard(environment, "manual translation")
            click(point(window, "toggle"))
            click(point(window, "toggle"))
            assert len(requests(root)) == 2
            copy_translation(environment, window, "manual translation")
            # 4. 【OCR 翻译回归】【过期与目标保存】原文编辑改变底部提示，自由语言确认只保存配置而不发送新请求
            fresh = snapshot(root, "fresh")
            edit(window, "source", "new source")
            stale = snapshot(root, "stale-source")
            notice = (window["x"] + 470, window["y"] + window["height"] - 70, window["x"] + window["width"] - 10, window["y"] + window["height"] - 26)
            assert ImageChops.difference(fresh.crop(notice), stale.crop(notice)).getbbox() is not None
            assert notice_rows(fresh, stale, window) == 2, "stale notice did not present both wrapped rows"
            edit(window, "language", "Esperanto")
            session.key("Return")
            assert tomllib.loads(config.read_text())["translation"]["targetLanguage"] == "Esperanto"
            assert len(requests(root)) == 2
            # 5. 【OCR 翻译回归】【原生语言列表】八行弹层通过 End 选择第九项，抓取关闭后目标保存但原文与译文均不改变
            click(point(window, "arrow"))
            node = popup(log, 1)[0]
            assert node["rect"][2:] == [window["width"] - 219, 250]
            snapshot(root, "language-popup")
            session.key("End")
            snapshot(root, "language-ninth")
            session.key("Return")
            popup(log, 0)
            assert tomllib.loads(config.read_text())["translation"]["targetLanguage"] == "Russian"
            click(point(window, "arrow"))
            node = popup(log, 1)[0]
            session.key("Home")
            left, top = window["x"] + node["rect"][0], window["y"] + node["rect"][1]
            click((left + 20, top + 20), "right")
            assert len(popup(log, 1)) == 1
            assert tomllib.loads(config.read_text())["translation"]["targetLanguage"] == "Russian"
            CURRENT.get()[0].scroll(dy=15.0)
            time.sleep(0.08)
            click((left + 20, top + 235))
            popup(log, 0)
            assert tomllib.loads(config.read_text())["translation"]["targetLanguage"] == "Russian"
            copy_translation(environment, window, "manual translation")
            click(source_point(window))
            session.wait_clipboard(environment, "new source")
            # 6. 【OCR 翻译回归】【译文右键】菜单具有独立原生抓取，关闭菜单不改变全文；完成与取消后均没有空闲提交
            click(point(window, "translation"), "right")
            popup(log, 1)
            snapshot(root, "translation-context-menu")
            session.key("Escape")
            popup(log, 0)
            # 7. 【OCR 翻译回归】【失败与取消保留】已有译文在命令失败及再次取消后保持原样，忙碌选区复制仍可用
            (root / "mode").write_text("error")
            click(point(window, "action"))
            record = requests(root, 3)[2]
            assert record["target"] == "Russian"
            reclaimed(record)
            time.sleep(0.12)
            copy_translation(environment, window, "manual translation")
            snapshot(root, "failed-result-kept")
            (root / "mode").write_text("hold")
            click(point(window, "action"))
            record = requests(root, 4)[3]
            edit(window, "translation", "forbidden replacement")
            session.key("ctrl+c")
            session.wait_clipboard(environment, "manual translation")
            click(point(window, "action"))
            reclaimed(record)
            copy_translation(environment, window, "manual translation")
            time.sleep(2.1)
            final_idle = idle(process.pid, log)
            assert len(requests(root)) == 4
            assert not (root / "data/mark-shot/models").exists()
            session.key("Escape")
            assert process.wait(timeout=3) == 0
            trees = popup_tree(log.read_text(), closed=True)
            return {"pointer_backend": pointer_backend, "request_count": 4, "busy_source_snapshot": True, "independent_translation_edit_and_undo": True,
                    "restored_translation_focus": True, "reopen_without_request": True, "stale_notice_presented": True,
                    "wrapped_notice_rows": 2,
                    "free_language_saved": True, "ninth_language_saved": True, "native_popup_count": len(trees),
                    "language_scroll_and_right_button": True,
                    "failure_and_cancel_preserved_translation": True, "busy_selection_copy": True,
                    "first_action_after_context_close": True,
                    "canceled_idle": canceled_idle, "final_idle": final_idle, "no_ocr_or_external_service": True}
        finally:
            if process.poll() is None:
                # 8. 【OCR 翻译回归】【失败清理】先关闭菜单和窗口以回收所属任务，只有未响应进程才使用终止信号
                for _ in range(2):
                    session.key("Escape")
                    try:
                        process.wait(timeout=1)
                        break
                    except subprocess.TimeoutExpired:
                        pass
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=3)


def run(binary, root):
    """【OCR 翻译回归】【三种边界】按 binary 和 root 执行交互、运行中关闭及取消后关闭，保存同构建指纹，无返回值"""
    root.mkdir(parents=True, exist_ok=False)
    proof = {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
             "compositor": "sway" if session.backend() is session.sway else "niri"}
    proof["interaction"] = scenario(binary, root / "interaction")
    proof["running_close"] = scenario(binary, root / "running-close", close_only=True)
    proof["cancel_then_close"] = scenario(binary, root / "cancel-close", close_only=True, close_after_cancel=True)
    (root / "result.json").write_text(json.dumps(proof, indent=2) + "\n")


def main():
    """【OCR 翻译回归】【入口】解析本地 binary、root 后执行，失败返回非零状态，无参数"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    args = parser.parse_args()
    run(args.binary.resolve(), args.root.resolve())


if __name__ == "__main__":
    main()
