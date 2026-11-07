#!/usr/bin/env python3
"""【OCR 翻译输入测试】【双端真实投递】同一私有协议驱动原版 Qt 和 Rust 窗口，检查独立历史、候选、忙碌资格和菜单焦点"""

import argparse
import hashlib
import json
from pathlib import Path
import select
import subprocess
import time
import tomllib

import wayland_text_format_session as session
from wayland_input_method_transport import InputMethod, build, protocol_evidence
from wayland_ocr_translation_checks import configure, idle, point, reclaimed, requests
from wayland_ocr_translation_input_checks import Checks, result_frame
from wayland_ocr_window_layout import tab_point
from wayland_text_pointer import click, pointer_context
from wayland_window_input_session import geometry, newer_state, screenshot


def launch(binary, root, environment, output, reference):
    """【OCR 翻译输入测试】【相同窗口】在 environment 启动 binary，root 和 output 保存日志，reference 选择只读 Qt，返回进程和实际矩形"""
    if reference:
        environment.update(MARK_SHOT_PRIVATE_QT_REFERENCE=environment["XDG_RUNTIME_DIR"], QT_QPA_PLATFORM="wayland")
        arguments = [str(binary), str(root), "--wayland"]
    else:
        arguments = [str(binary), "--ocr-result", "--nonotify"]
    process = subprocess.Popen(arguments, env=dict(environment, WAYLAND_DEBUG="client"), stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=output)
    try:
        if not reference:
            process.stdin.write(result_frame())
            process.stdin.close()
        expected = b"ready\n" if reference else b"\n"
        assert select.select([process.stdout], [], [], 8)[0] and process.stdout.readline() == expected
        return process, geometry(process.pid, environment)
    except BaseException:
        if process.poll() is None:
            process.terminate()
        process.wait(timeout=3)
        raise


def scenario(checks, root, reference):
    """【OCR 翻译输入测试】【平台序列】按 checks、root 在相同窗口执行协议，reference 控制观察或差分断言，返回真实操作结果"""
    method, window = checks.method, checks.window
    results = {}
    text = "translated base"
    source = "base!"
    # 1. 【OCR 翻译输入测试】【真实结果】两端均由同一本地命令创建初始译文，不在测试中替换业务编辑器
    click(point(window, "toggle"))
    reclaimed(requests(root, 1)[0])
    checks.focus("translation", text)
    assert method.states[-1]["hint"] & 512 and method.states[-1]["purpose"] == 0
    checks.checkpoint("initial")
    profile = checks.candidate("translation-preedit", text, "ni")
    checks.full_copy("translation-with-preedit", text)
    checks.batch(text + "X", commit="X", preedit="hao")
    checks.checkpoint("translation-commit-x")
    checks.batch(text + "XY", commit="Y")
    checks.checkpoint("translation-commit-y")
    # 2. 【OCR 翻译输入测试】【独立历史】连续多行确认分别撤销，辅助平面字符通过实际 UTF-8 删除替换
    for name, chord, value in [("undo-y", "ctrl+z", text + "X"), ("undo-x", "ctrl+z", text), ("redo-x", "ctrl+shift+z", text + "X"), ("redo-y", "ctrl+shift+z", text + "XY")]:
        previous = method.states[-1]["serial"]
        session.key(chord)
        newer_state(method, previous, value)
        checks.checkpoint("translation-" + name)
        if name == "undo-x":
            checks.ctrl_y("translation-ctrl-y-no-redo", text)
    checks.batch(text + "XY𠮷", commit="𠮷")
    checks.checkpoint("translation-supplementary")
    text += "XY你"
    checks.batch(text, commit="你", delete=(4, 0))
    checks.checkpoint("translation-replacement")
    checks.full_copy("translation-replacement", text)
    results["translation_preedit_pixels"] = profile["pixels"]
    # 3. 【OCR 翻译输入测试】【忙碌原文】原文候选保持显示，请求只包含确认快照，译文和目标停用输入法
    checks.focus("source", "base")
    source_profile = checks.candidate("source-preedit", "base", "source-candidate")
    (root / "mode").write_text("wait")
    session.key("ctrl+Return")
    record = requests(root, 2)[1]
    assert record["source"]["tokens"][0]["text"] == "base" and record["target"] == "English"
    method.state("base")
    results["busy-source-restored"] = checks.restored_candidate("busy-source-restored", source_profile)
    checks.checkpoint("busy-source-preedit")
    checks.batch("base!", commit="!")
    checks.checkpoint("busy-source-commit")
    previous = method.states[-1]["serial"]
    session.key("ctrl+z")
    newer_state(method, previous, "base")
    checks.checkpoint("source-undo")
    checks.ctrl_y("source-ctrl-y-no-redo", "base")
    previous = method.states[-1]["serial"]
    session.key("ctrl+shift+z")
    newer_state(method, previous, "base!")
    checks.checkpoint("source-redo")
    previous = method.states[-1]["serial"]
    click(point(window, "translation"))
    newer_state(method, previous, None, active=False)
    method.batch(commit="forbidden")
    time.sleep(0.08)
    checks.full_copy("busy-translation", text)
    checks.checkpoint("busy-translation-rejected")
    click(point(window, "action"))
    reclaimed(record)
    checks.focus("translation", text)
    checks.checkpoint("canceled")
    # 4. 【OCR 翻译输入测试】【隐藏与恢复】真实切页停用，过期确认不能进入恢复后的译文，候选外观对照实际 Qt
    view_profile = checks.candidate("before-image", text, "view-candidate")
    previous = method.states[-1]["serial"]
    click(tab_point(window, "Image", has_table=False))
    newer_state(method, previous, None, active=False)
    checks.checkpoint("on-image")
    method.batch(commit="hidden-stale")
    click(tab_point(window, "Text", has_table=False))
    checks.context(text)
    time.sleep(0.08)
    results["restored-translation"] = checks.restored_candidate("restored-translation", view_profile)
    checks.checkpoint("restored-translation")
    checks.batch(text, preedit="", allow_no_update=True)
    checks.full_copy("translation-after-image", text)
    # 5. 【OCR 翻译输入测试】【原生右键】译文菜单失焦、候选外观和过期批次逐项匹配原版，关闭后恢复同一确认正文
    results["translation-menu"] = checks.menu("translation", text, "translation-menu")
    text = results["translation-menu"]["after_text"]
    checks.full_copy("translation-after-menu", text)
    results["source-menu"] = checks.menu("source", source, "source-menu")
    source = results["source-menu"]["after_text"]
    checks.full_copy("source-after-menu", source)
    # 6. 【OCR 翻译输入测试】【自由语言】单行候选删除所选文字但不保存候选，Enter 确认目标，忙碌期间完全停用
    checks.focus("language", "English")
    assert not method.states[-1]["hint"] & 512 and method.states[-1]["purpose"] == 0
    checks.select_all("English")
    checks.batch("", preedit="free")
    checks.checkpoint("language-preedit-selection")
    checks.batch("Esperanto", commit="Esperanto")
    checks.checkpoint("language-commit")
    session.key("Return")
    checks.context("Esperanto")
    checks.checkpoint("language-return-saved")
    checks.candidate("language-preedit-before-start", "Esperanto", "target-candidate")
    previous = method.states[-1]["serial"]
    session.key("ctrl+Return")
    record = requests(root, 3)[2]
    assert record["target"] == "Esperanto" and record["source"]["tokens"][0]["text"] == source
    newer_state(method, previous, None, active=False)
    checks.checkpoint("busy-from-language")
    method.batch(commit="disabled-language")
    click(point(window, "action"))
    reclaimed(record)
    checks.focus("language", "Esperanto")
    checks.checkpoint("language-restored")
    results["language-menu"] = checks.menu("language", "Esperanto", "language-menu")
    checks.checkpoint("final")
    checks.focus("source", source)
    checks.full_copy("source-final", source)
    checks.focus("translation", text)
    checks.full_copy("translation-final", text)
    results.update(requests=3,source_snapshot_excludes_preedit=True,translation_multiline=True,language_singleline=True,readonly_and_disabled_ignored_stale_commit=True,independent_undo_redo=True,supplementary_byte_replacement=True,final_translation=text,final_source=source)
    return results


def run(binary, root, xml, reference=False, expected=None):
    """【OCR 翻译输入测试】【完整会话】用 binary、root、xml 执行真实平台验证，reference 指原版观察，expected 指原版结果路径，无返回值"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    config = configure(root)
    (root / "mode").write_text("success")
    if reference:
        command = tomllib.loads(config.read_text())["translation"]["command"]
        (root / "reference-options.json").write_text(json.dumps({"command": command}))
    expected = json.loads(expected.read_text())["operations"] if expected else None
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    client = build(root / "client", xml)
    log = root / "process.log"
    with session.input_context(), pointer_context(), log.open("w") as output:
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        method = InputMethod(client, environment, root)
        process = None
        try:
            process, window = launch(binary, root, environment, output, reference)
            method.state("base")
            checks = Checks(method, window, root, log, environment, reference, expected)
            operations = scenario(checks, root, reference)
            time.sleep(2.1)
            resources = None if reference else idle(process.pid, log)
            evidence = protocol_evidence(log.read_text())
            assert evidence["native_protocol"] and evidence["preedit_delivered"]
            screenshot(root, "completed")
            session.key("Escape")
            assert process.wait(timeout=4) == 0
            proof = {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "reference": reference, "compositor": "sway" if session.backend() is session.sway else "niri", "operations": operations, "states": checks.states, "clipboard": checks.clipboard, "reference_uncommitted_surrounding_updates": checks.protocol_delays, "resources": resources, "no_ocr_or_paid_service": True, **evidence}
            (root / "result.json").write_text(json.dumps(proof,ensure_ascii=False,indent=2) + "\n")
        finally:
            if process and process.poll() is None:
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
            method.close()


def main():
    """【OCR 翻译输入测试】【入口】解析二进制、协议、私有目录与原版结果，执行后失败返回非零状态，无参数"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--xml", type=Path, default=Path("target/test-tools/input-method/input-method-unstable-v2.xml"))
    parser.add_argument("--reference", action="store_true")
    parser.add_argument("--expected-reference", type=Path)
    args = parser.parse_args()
    if not args.reference and not args.expected_reference:
        parser.error("native comparison requires --expected-reference")
    run(args.binary.resolve(), args.root.resolve(), args.xml.resolve(), args.reference, args.expected_reference)


if __name__ == "__main__":
    main()
