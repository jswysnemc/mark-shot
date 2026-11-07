#!/usr/bin/env python3
"""【复制反馈】【真实会话】验证切页保留、原文与表格独立期限及反馈结束后的静止资源，不使用 OCR 模型"""

import argparse
import hashlib
import json
from pathlib import Path
import select
import subprocess
import time

from PIL import Image
import wayland_text_format_session as session
from wayland_ocr_preview_checks import equal_pixels
from wayland_text_pointer import CURRENT, click, pointer_context
from wayland_virtual_pointer import transform_for_pointer
from wayland_window_input_session import ROWS, frame, geometry
from wayland_ocr_window_layout import source_point, source_right, tab_point, table_point


def caption(root, name, window, table=False, view="Text"):
    """【复制反馈】【真实文字】把 window 的按钮内文字保存到 root/name，table 选择独立表格按钮，返回排除边框与悬停的像素"""
    pointer, logical, _, _ = CURRENT.get()
    point = (window["x"] + window["width"] / 2, window["y"] + window["height"] / 2)
    pointer.move_abs(*transform_for_pointer(point[0] - logical["x"], point[1] - logical["y"], logical["width"], logical["height"], logical.get("pointer_transform", logical["transform"])))
    time.sleep(0.08)
    path = root / (name + ".png")
    session.screenshot(path)
    right = window["x"] + window["width"] - 8 if table else source_right(window, view)
    top = window["y"] + (39 if table else 6)
    with Image.open(path) as image:
        return image.convert("RGBA").crop((round(right - 52), round(top + 5), round(right - 8), round(top + 22)))


def copy_source(environment, window, view="Text"):
    """【复制反馈】【原文动作】在 window 标题栏完成左键复制并核对 environment 剪贴板，不直接设置反馈标志，无返回值"""
    click(source_point(window, view))
    session.wait_clipboard(environment, "base")


def run(binary, root):
    """【复制反馈】【完整回归】在已验证私有会话执行 binary，root 保存同二进制像素、协议及期限证据，失败回收本次进程"""
    environment = session.environment()
    root.mkdir(parents=True, exist_ok=False)
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    config = root / "config/mark-shot/config.toml"
    config.parent.mkdir(parents=True)
    config.write_text("[history]\nenabled = false\n[notifications]\nenabled = false\n")
    log = root / "process.log"
    with session.input_context(), pointer_context(), log.open("w") as output:
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        subprocess.run(["wl-copy", "--", "feedback sentinel"], env=environment, check=True, timeout=3)
        process = subprocess.Popen([str(binary), "--ocr-result", "--nonotify"], env=dict(environment, WAYLAND_DEBUG="client"), stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=output)
        try:
            process.stdin.write(frame())
            process.stdin.close()
            assert select.select([process.stdout], [], [], 5)[0] and process.stdout.readline() == b"\n"
            window = geometry(process.pid, environment)
            click(source_point(window), "right")
            baseline = caption(root, "source-before", window)
            # 1. 【复制反馈】【相同位置】非文字页按钮右移，分数缓冲的像素相位不同；各页只与相同几何的基准逐像素比较
            click(tab_point(window, "Image"))
            image_baseline = caption(root, "image-before", window, view="Image")
            click(tab_point(window, "Table"))
            table_source_baseline = caption(root, "table-source-before", window, view="Table")
            click(tab_point(window, "Text"))
            # 1. 【复制反馈】【切页保留】原文复制经过实际左键释放；切换原图后同一按钮仍显示相同反馈
            copy_source(environment, window)
            copied = caption(root, "source-copied", window)
            assert not equal_pixels(baseline, copied), "source copy did not present feedback"
            click(tab_point(window, "Image"))
            image = caption(root, "image-copied", window, view="Image")
            assert not equal_pixels(image_baseline, image), "view switch canceled source copy feedback before its deadline"
            time.sleep(2.05)
            expired = caption(root, "source-expired-on-image", window, view="Image")
            assert equal_pixels(image_baseline, expired), "source feedback did not expire without interaction"
            # 2. 【复制反馈】【独立表格】表格菜单完成后不覆盖原文反馈；原文重新复制也不重置表格期限
            click(tab_point(window, "Text"))
            copy_source(environment, window)
            click(tab_point(window, "Table"))
            table_source_copied = caption(root, "table-source-copied", window, view="Table")
            assert not equal_pixels(table_source_baseline, table_source_copied), "table view canceled source feedback"
            table_before = caption(root, "table-before", window, table=True)
            click(table_point(window))
            session.key("Home")
            session.key("Return")
            session.wait_clipboard(environment, "\n".join("\t".join(row) for row in ROWS))
            table_copied = caption(root, "table-copied", window, table=True)
            assert not equal_pixels(table_before, table_copied), "table copy did not present its own feedback"
            assert equal_pixels(table_source_copied, caption(root, "source-kept-after-table-copy", window, view="Table")), "table copy overwrote source feedback"
            time.sleep(0.65)
            copy_source(environment, window, view="Table")
            time.sleep(1.2)
            assert equal_pixels(table_before, caption(root, "table-expired-independently", window, table=True)), "source copy reset the independent table deadline"
            assert equal_pixels(table_source_copied, caption(root, "source-renewed", window, view="Table")), "renewed source feedback expired on the old table deadline"
            time.sleep(0.9)
            assert equal_pixels(table_source_baseline, caption(root, "all-feedback-expired", window, view="Table")), "renewed source feedback remained indefinitely"
            # 3. 【复制反馈】【编辑器隔离】普通选区复制不触发标题栏全文复制反馈
            click(tab_point(window, "Text"))
            click((window["x"] + 130, window["y"] + 150))
            session.key("ctrl+a")
            subprocess.run(["wl-copy", "--", "editor selection sentinel"], env=environment, check=True, timeout=3)
            session.key("ctrl+c")
            session.wait_clipboard(environment, "base")
            assert equal_pixels(baseline, caption(root, "editor-copy-without-fulltext-feedback", window)), "editor selection copy triggered whole-source feedback"
            # 4. 【复制反馈】【单次期限】反馈清除后没有周期提交、空转或等待线程；复制不初始化识别模型
            session.wait_single_thread(process.pid)
            before = session.resources(process.pid)
            requests = log.read_text().count("wl_surface")
            time.sleep(1.1)
            after = session.resources(process.pid)
            assert before["threads"] == after["threads"] == 1 and before["cpu_ticks"] == after["cpu_ticks"]
            assert log.read_text().count("wl_surface") == requests
            assert not (root / "data/mark-shot/models").exists()
            session.key("Escape")
            assert process.wait(timeout=3) == 0
            proof = {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "compositor": "sway" if session.backend() is session.sway else "niri", "view_kept_source_feedback": True, "source_and_table_independent": True, "renewed_source_deadline": True, "expired_without_interaction": True, "editor_copy_kept_fulltext_feedback_idle": True, "no_idle_frames_after_expiry": True, "idle_before": before, "idle_after": after, "no_ocr": True}
            (root / "result.json").write_text(json.dumps(proof, indent=2) + "\n")
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)


def main():
    """【复制反馈】【入口】解析本地 binary、root 后执行真实回归，无参数，失败返回非零状态"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    args = parser.parse_args()
    run(args.binary.resolve(), args.root.resolve())


if __name__ == "__main__":
    main()
