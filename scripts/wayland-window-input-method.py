#!/usr/bin/env python3
"""【窗口输入法测试】【真实投递】验证原文、单元格、原图及按钮焦点的原生输入法，不启动 OCR"""

import argparse
import json
from pathlib import Path
import select
import subprocess
import tempfile
import time

from PIL import ImageChops
from wayland_input_method_transport import InputMethod, build, protocol_evidence
import wayland_text_format_session as session
from wayland_text_pointer import click, pointer_context
from wayland_window_input_session import ROWS, cursor_rect, frame, geometry, newer_state, press, presented_candidate, screenshot


def batch(method, text, **arguments):
    """【窗口输入法测试】【确认等待】向 method 发送 arguments，等待 text 对应的新状态，返回协议状态"""
    previous = method.states[-1]["serial"]
    method.batch(**arguments)
    return newer_state(method, previous, text)


def copy_source(environment, expected):
    """【窗口输入法测试】【原文复制】验证全局原文快捷键复制 expected，environment 为本次私有会话，无返回值"""
    session.key("ctrl+shift+c")
    session.wait_clipboard(environment, expected)


def source(method, window, environment, root, log):
    """【窗口输入法测试】【原文回归】使用 method、window、environment 检查候选和历史，root、log 保存真实证据，返回确认原文"""
    method.state("base")
    session.key("ctrl+End")
    method.wait(lambda line: line.startswith("state\t") and method.states[-1]["cursor"] == 4)
    before = screenshot(root, "source-baseline")
    # 1. 【窗口输入法测试】【候选呈现】等待实际字形，不用协议激活或光标移动代替画面验证
    old = cursor_rect(log)
    state = batch(method, "base", preedit="ni")
    assert state["hint"] & 512, "multiline editor omitted the multiline hint"
    _, pixels = presented_candidate(root, "source-preedit", before, window, old, log)
    copy_source(environment, "base")
    # 2. 【窗口输入法测试】【确认与替换】逐次确认独立撤销，UTF-8 删除必须完整命中码点
    batch(method, "baseX", commit="X", preedit="hao")
    batch(method, "baseXY", commit="Y")
    for chord, text in (("ctrl+z", "baseX"), ("ctrl+z", "base"), ("ctrl+shift+z", "baseX"), ("ctrl+y", "baseXY")):
        previous = method.states[-1]["serial"]
        session.key(chord)
        newer_state(method, previous, text)
    batch(method, "baseXY𠮷", commit="𠮷")
    batch(method, "baseXY你", commit="你", delete=(4, 0))
    time.sleep(0.08)
    confirmed = screenshot(root, "replacement-confirmed")
    old = cursor_rect(log)
    batch(method, "baseXY你", preedit="ni")
    previous, _ = presented_candidate(root, "invalid-before", confirmed, window, old, log)
    method.batch(commit="wrong", delete=(1, 0))
    time.sleep(0.15)
    rejected = screenshot(root, "invalid-after")
    box = (round(window["x"] + 18), round(window["y"] + 88), round(window["x"] + window["width"] - 18), round(window["y"] + window["height"] - 38))
    assert ImageChops.difference(previous.crop(box), rejected.crop(box)).getbbox() is None, "invalid batch changed confirmed or candidate pixels"
    copy_source(environment, "baseXY你")
    previous_serial = method.states[-1]["serial"]
    # 3. 【窗口输入法测试】【页面隔离】隐藏编辑器先停用，过期确认不能进入恢复后的原文
    press(window, 98, 56)
    newer_state(method, previous_serial, None, active=False)
    method.batch(commit="inactive")
    time.sleep(0.08)
    press(window, 36, 56)
    method.state("baseXY你")
    copy_source(environment, "baseXY你")
    (root / "source-result.json").write_text(json.dumps({"native_protocol": True, "preedit_glyph_pixels": pixels, "multiline_hint": True, "independent_commit_undo": True, "supplementary_byte_replacement": True, "invalid_batch_preserved_pixels": True, "hidden_view_ignored_commit": True, "confirmed": "baseXY你"}, ensure_ascii=False, indent=2) + "\n")
    return "baseXY你"


def source_menu(method, window, environment, root, log, original):
    """【窗口输入法测试】【菜单候选】用 method 验证 window 原文菜单停用与恢复，environment、root、log 保存实际字形，original 为确认正文；无返回值"""
    # 1. 【窗口输入法测试】【原文菜单】候选显示后打开真实右键菜单，停用协议并拒绝过期确认，关闭后恢复原文身份
    session.key("ctrl+End")
    time.sleep(0.08)
    baseline = screenshot(root, "source-menu-baseline")
    old = cursor_rect(log)
    batch(method, original, preedit="menu-candidate")
    presented_candidate(root, "source-menu-preedit", baseline, window, old, log)
    previous_serial = method.states[-1]["serial"]
    click((window["x"] + 220, window["y"] + 160), "right")
    newer_state(method, previous_serial, None, active=False)
    time.sleep(0.12)
    canceled = screenshot(root, "source-menu-canceled-preedit")
    region = (round(window["x"] + 18), round(window["y"] + 88), round(window["x"] + 190), round(window["y"] + 114))
    assert ImageChops.difference(baseline.crop(region), canceled.crop(region)).getbbox() is None, "source menu retained candidate glyphs"
    method.batch(commit="stale-menu")
    previous_serial = method.states[-1]["serial"]
    session.key("Escape")
    newer_state(method, previous_serial, original)
    copy_source(environment, original)
    path = root / "source-result.json"
    result = json.loads(path.read_text())
    result.update(source_menu_preedit_canceled=True, source_menu_stale_commit_ignored=True)
    path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")


def copy_table(environment, window, expected):
    """【窗口输入法测试】【表格导出】在 window 通过真实菜单复制 expected，environment 为私有会话，无返回值"""
    press(window, window["width"] - 44, 56)
    press(window, window["width"] - 98, 92)
    session.wait_clipboard(environment, expected)


def table(method, window, environment, root, original):
    """【窗口输入法测试】【单元格回归】使用 method 验证 window 的单行历史、提交、取消和菜单，environment、root 保存证据，original 为原文"""
    previous = method.states[-1]["serial"]
    press(window, 160, 56)
    newer_state(method, previous, None, active=False)
    session.key("F2")
    # 1. 【窗口输入法测试】【单行编辑】实际开启单元格编辑后才取得输入资格，连续确认合并撤销
    state = method.state("Item")
    assert not state["hint"] & 512 and state["purpose"] == 0, "cell editor used multiline or numeric content type"
    batch(method, "", preedit="ni")
    screenshot(root, "cell-preedit")
    batch(method, "你", commit="你")
    batch(method, "你𠮷", commit="𠮷")
    previous = method.states[-1]["serial"]
    session.key("ctrl+z")
    newer_state(method, previous, "Item")
    previous = method.states[-1]["serial"]
    session.key("ctrl+shift+z")
    newer_state(method, previous, "你𠮷")
    batch(method, "你好", commit="好", delete=(4, 0))
    copy_source(environment, original)
    # 2. 【窗口输入法测试】【单元格提交】Enter 写回草稿，数据导出经真实菜单而非原文快捷键
    previous = method.states[-1]["serial"]
    session.key("Return")
    newer_state(method, previous, None, active=False)
    rows = [row.copy() for row in ROWS]
    rows[0][0] = "你好"
    expected = "\n".join("\t".join(row) for row in rows)
    copy_table(environment, window, expected)
    press(window, 82, 122)
    # 3. 【窗口输入法测试】【单元格取消】重新编辑后 Escape 丢弃，不回退已经确认的表格
    session.key("F2")
    method.state("你好")
    batch(method, "discarded", commit="discarded")
    previous = method.states[-1]["serial"]
    session.key("Escape")
    newer_state(method, previous, None, active=False)
    copy_table(environment, window, expected)
    (root / "table.tsv").write_text(expected)
    (root / "table-result.json").write_text(json.dumps({"native_protocol": True, "singleline_hint": True, "selected_preedit_removed_only_draft": True, "merged_commit_undo": True, "supplementary_byte_replacement": True, "return_committed_cell": True, "escape_discarded_draft": True, "global_copy_kept_original_source": True, "menu_tsv": expected}, ensure_ascii=False, indent=2) + "\n")


def run(binary, root, xml):
    """【窗口输入法测试】【完整场景】用 binary、xml 在当前私有合成器验证输入法，root 保存同指纹证据，无返回值"""
    import hashlib
    environment = session.environment()
    root.mkdir(parents=True, exist_ok=False)
    client = build(root / "client", xml)
    with session.input_context(), pointer_context():
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        method = InputMethod(client, environment, root)
        with tempfile.TemporaryDirectory(prefix="mark-shot-window-ime-") as temporary:
            private = Path(temporary)
            environment.update(XDG_CONFIG_HOME=str(private / "config"), XDG_DATA_HOME=str(private / "data"), XDG_STATE_HOME=str(private / "state"), WAYLAND_DEBUG="client")
            config = private / "config/mark-shot"
            config.mkdir(parents=True)
            (config / "config.toml").write_text('[notifications]\nenabled = false\n[history]\nenabled = false\n')
            log_path = root / "process.log"
            with log_path.open("w") as log:
                process = subprocess.Popen([str(binary), "--ocr-result"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=log, env=environment)
                try:
                    process.stdin.write(frame())
                    process.stdin.close()
                    if not select.select([process.stdout], [], [], 5)[0] or process.stdout.readline() != b"\n":
                        raise AssertionError("OCR window did not submit its initial frame")
                    window = geometry(process.pid, environment)
                    original = source(method, window, environment, root, log_path)
                    table(method, window, environment, root, original)
                    press(window, 36, 56)
                    method.state(original)
                    source_menu(method, window, environment, root, log_path, original)
                    session.wait_single_thread(process.pid)
                    before = session.resources(process.pid)
                    commits = log_path.read_text().count("wl_surface")
                    time.sleep(1.1)
                    after = session.resources(process.pid)
                    assert before["threads"] == after["threads"] == 1 and before["cpu_ticks"] == after["cpu_ticks"], "stationary result window consumed CPU or retained worker threads"
                    assert log_path.read_text().count("wl_surface") == commits, "stationary window continued submitting surface requests"
                    assert not (private / "data/mark-shot/models").exists(), "result editing initialized OCR models"
                    evidence = protocol_evidence(log_path.read_text())
                    assert evidence["native_protocol"] and evidence["preedit_delivered"]
                    session.key("Escape")
                    assert process.wait(timeout=3) == 0
                    (root / "result.json").write_text(json.dumps({"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "bytes": binary.stat().st_size, "compositor": "sway" if session.backend() is session.sway else "niri", "source": json.loads((root / "source-result.json").read_text()), "table": json.loads((root / "table-result.json").read_text()), "idle_before": before, "idle_after": after, "no_idle_surface_requests": True, "no_ocr": True, **evidence}, ensure_ascii=False, indent=2) + "\n")
                finally:
                    if process.poll() is None:
                        process.terminate()
                        process.wait(timeout=3)
                    method.close()


def main():
    """【窗口输入法测试】【入口】解析二进制、协议和证据目录后运行验证，无参数；失败以非零状态退出"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--xml", type=Path, default=Path("target/test-tools/input-method/input-method-unstable-v2.xml"))
    args = parser.parse_args()
    run(args.binary.resolve(), args.root.resolve(), args.xml.resolve())


if __name__ == "__main__":
    main()
