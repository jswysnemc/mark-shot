#!/usr/bin/env python3
"""【原文菜单测试】【私有桌面】验证真实原生菜单、鼠标职责、输入隔离和动作顺序，不调用 OCR 模型"""

import argparse
import hashlib
import json
from pathlib import Path
import re
import select
import struct
import subprocess
import time

import wayland_text_format_session as session
from wayland_ocr_source_menu_checks import popup_configurations
from wayland_text_pointer import click, pointer_context
from wayland_window_input_session import geometry

ROOT = Path(__file__).resolve().parents[1]
SOURCE = "alpha beta\ngamma"
FIXTURE = json.loads((ROOT / "tests/fixtures/qt-ocr-source-menu.json").read_text())


def copy(environment, text):
    """【原文菜单测试】【剪贴板提供】在 environment 的私有座席提供 text，返回无；不连接用户剪贴板"""
    subprocess.run(["wl-copy", "--", text], env=environment, check=True, timeout=3)


def document():
    """【原文菜单测试】【固定文档】返回真实结果传输帧，正文与 Qt 对照一致，不运行识别模型，无参数"""
    header = json.dumps({"text": SOURCE, "lines": []}).encode()
    png = (ROOT / "tests/fixtures/ocr-table.png").read_bytes()
    return b"MSOCR001" + struct.pack("<II", len(header), len(png)) + header + png


def select_alpha():
    """【原文菜单测试】【选区准备】通过真实快捷键选择首词，不替换正文，无参数、无返回值"""
    session.key("ctrl+Home")
    for _ in range(5):
        session.key("shift+Right")


def configurations(log):
    """【原文菜单测试】【协议位置】从 log 的原生配置返回菜单矩形列表，不把父窗口画面误认成菜单"""
    return popup_configurations(log.read_text())


def opened(window, log):
    """【原文菜单测试】【右键打开】在 window 原文中右键，等待 log 新菜单配置，返回真实矩形；没有菜单时失败"""
    previous = len(configurations(log))
    click((window["x"] + 120, window["y"] + 120), "right")
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        values = configurations(log)
        if len(values) > previous:
            rect = values[-1]
            assert list(rect[2:]) == FIXTURE["layout"]["size"], "native source menu dimensions differ from actual Qt"
            time.sleep(0.08)
            return rect
        time.sleep(0.03)
    raise AssertionError("right-click did not create an independent native OCR source menu")


def item(window, popup, label):
    """【原文菜单测试】【真实行位置】按 window、popup 及冻结 Qt 样本返回 label 行中心，不使用猜测尺寸"""
    row = next(row for row in FIXTURE["layout"]["rows"] if row["label"] == label)
    x, y, width, height = row["rect"]
    return window["x"] + popup[0] + x + width / 2, window["y"] + popup[1] + y + height / 2


def source(environment, expected):
    """【原文菜单测试】【正文断言】关闭菜单后用真实全文快捷键复制 expected，environment 指向私有座席，无返回值"""
    copy(environment, "source sentinel")
    session.key("ctrl+shift+c")
    session.wait_clipboard(environment, expected)


def closed(log, previous):
    """【原文菜单测试】【销毁等待】等待 log 在 previous 之后产生原生 popup 销毁请求，返回新增协议文字"""
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        text = log.read_text()[previous:]
        if re.search(r"xdg_popup@\d+\.destroy\(\)", text):
            time.sleep(0.05)
            return text
        time.sleep(0.03)
    raise AssertionError("source menu did not destroy its native role")


def choose(window, log, label):
    """【原文菜单测试】【行执行】在 window 菜单点击 label，等待 log 角色销毁，返回协议增量"""
    popup = opened(window, log)
    previous = len(log.read_text())
    click(item(window, popup, label))
    return closed(log, previous)


def run(binary, root):
    """【原文菜单测试】【完整回归】在已校验私有会话运行 binary，root 保存同指纹画面和协议证据；失败立即退出，无返回值"""
    environment = session.environment()
    root.mkdir(parents=True, exist_ok=False)
    private = root / "private"
    environment.update(XDG_CONFIG_HOME=str(private / "config"), XDG_DATA_HOME=str(private / "data"), XDG_STATE_HOME=str(private / "state"))
    log = root / "process.log"
    with session.input_context(), pointer_context(), log.open("w") as output:
        process = subprocess.Popen([str(binary), "--ocr-result", "--nonotify"], env=dict(environment, WAYLAND_DEBUG="client"), stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=output)
        try:
            process.stdin.write(document())
            process.stdin.close()
            assert select.select([process.stdout], [], [], 5)[0] and process.stdout.readline() == b"\n", "result did not map"
            window = geometry(process.pid, environment)
            select_alpha()
            copy(environment, "pasted text")
            popup = opened(window, log)
            session.screenshot(root / "source-menu.png")
            # 1. 【原文菜单测试】【鼠标职责】右键菜单项不复制，控制快捷键不剪切，Escape 只关闭菜单
            click(item(window, popup, "Copy"), "right")
            assert session.read_clipboard(environment) == "pasted text", "right-click copied source text"
            session.key("ctrl+x")
            assert session.read_clipboard(environment) == "pasted text", "editor shortcut crossed the menu grab"
            previous = len(log.read_text())
            session.key("Escape")
            closed(log, previous)
            assert process.poll() is None, "menu Escape closed the result window"
            source(environment, SOURCE)
            choose(window, log, "Copy")
            session.wait_clipboard(environment, "alpha")
            # 2. 【原文菜单测试】【动作顺序】原生角色先销毁，再创建剪贴板源，复制选择保持原样且关闭后仍可读取
            select_alpha()
            protocol = choose(window, log, "Cut")
            assert protocol.index(".destroy()") < protocol.index("【文字识别】【选区复制】开始复制"), "clipboard operation started before menu destruction"
            session.wait_clipboard(environment, "alpha")
            source(environment, " beta\ngamma")
            choose(window, log, "Undo")
            source(environment, SOURCE)
            choose(window, log, "Redo")
            source(environment, " beta\ngamma")
            choose(window, log, "Undo")
            source(environment, SOURCE)
            select_alpha()
            copy(environment, "pasted text")
            choose(window, log, "Paste")
            source(environment, "pasted text beta\ngamma")
            choose(window, log, "Undo")
            select_alpha()
            choose(window, log, "Delete")
            source(environment, " beta\ngamma")
            choose(window, log, "Undo")
            source(environment, SOURCE)
            choose(window, log, "Select All")
            choose(window, log, "Copy")
            session.wait_clipboard(environment, SOURCE)
            # 3. 【原文菜单测试】【导航和禁用】无选择时 Cut 禁用，Home 跳到可用项，首字母定位后 Enter 执行动作
            session.key("ctrl+Home")
            copy(environment, "navigation sentinel")
            popup = opened(window, log)
            click(item(window, popup, "Cut"))
            assert session.read_clipboard(environment) == "navigation sentinel", "disabled Cut executed"
            session.key("s")
            session.screenshot(root / "select-all-highlight.png")
            previous = len(log.read_text())
            session.key("Return")
            closed(log, previous)
            choose(window, log, "Copy")
            session.wait_clipboard(environment, SOURCE)
            # 4. 【原文菜单测试】【外部取消】点击父窗口正文只关闭抓取，不能改变原选区或触发窗口控件
            select_alpha()
            opened(window, log)
            previous = len(log.read_text())
            click((window["x"] + 40, window["y"] + 220))
            closed(log, previous)
            choose(window, log, "Copy")
            session.wait_clipboard(environment, "alpha")
            # 5. 【原文菜单测试】【空闲资源】菜单显示时没有周期帧或 CPU 空转，关闭窗口不遗留额外进程或模型
            opened(window, log)
            session.wait_single_thread(process.pid)
            before = session.resources(process.pid)
            requests = log.read_text().count("wl_surface")
            time.sleep(1.1)
            after = session.resources(process.pid)
            assert before["cpu_ticks"] == after["cpu_ticks"] and before["threads"] == after["threads"] == 1, "idle menu consumed CPU or retained workers"
            assert log.read_text().count("wl_surface") == requests, "idle menu continued committing frames"
            session.key("Escape")
            source(environment, SOURCE)
            session.key("Escape")
            assert process.wait(timeout=3) == 0
            session.wait_clipboard(environment, SOURCE)
            assert not (private / "data/mark-shot/models").exists(), "source menu initialized OCR models"
            proof = {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "bytes": binary.stat().st_size,
                "compositor": "sway" if session.backend() is session.sway else "niri", "native_popup": True,
                "qt_menu_size": FIXTURE["layout"]["size"], "preserved_right_click_selection": True,
                "right_button_and_shortcut_isolation": True, "clipboard_after_popup_destruction": True,
                "copy_cut_paste_delete_undo_redo_select_all": True, "disabled_and_mnemonic_navigation": True,
                "outside_click_not_forwarded": True, "idle_before": before, "idle_after": after,
                "no_idle_frames": True, "no_ocr": True}
            (root / "result.json").write_text(json.dumps(proof, indent=2) + "\n")
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)


def main():
    """【原文菜单测试】【入口】解析二进制和证据目录后执行私有回归，无参数；失败返回非零状态"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    args = parser.parse_args()
    run(args.binary.resolve(), args.root.resolve())


if __name__ == "__main__":
    main()
