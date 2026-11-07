#!/usr/bin/env python3
"""【OCR 置顶测试】【真实会话】私有 Wayland 验证角色切换、独立配置、菜单、输入法和运行中的翻译任务"""

import argparse
import hashlib
import json
from pathlib import Path
import re
import select
import subprocess
import time

from PIL import ImageChops
import wayland_text_format_session as session
from wayland_input_method_transport import InputMethod, build, protocol_evidence
from wayland_ocr_stacking_checks import layer_window, roles, wait_config, wait_role
from wayland_ocr_translation_checks import configure, idle, point, reclaimed, requests
from wayland_ocr_translation_input_checks import Checks, result_frame
from wayland_text_pointer import CURRENT, click, pointer_context
from wayland_virtual_pointer import transform_for_pointer
from wayland_window_input_session import geometry, newer_state, screenshot


def pin(window):
    """【OCR 置顶测试】【原版命中】按 window 返回标题置顶按钮中心，布局来自原版完整窗口，返回逻辑坐标"""
    return window["x"] + window["width"] - 53, window["y"] + 19


def drag(start, delta):
    """【OCR 置顶测试】【真实拖动】用同一私有设备从 start 按下并连续移动 delta，最后释放；不更新内部窗口几何，无返回值"""
    pointer, logical, _, _ = CURRENT.get()
    coordinates = transform_for_pointer(start[0] - logical["x"], start[1] - logical["y"], logical["width"], logical["height"], logical.get("pointer_transform", logical["transform"]))
    pointer.move_abs(*coordinates)
    time.sleep(0.08)
    pointer.press()
    try:
        time.sleep(0.04)
        for fraction in [1 / 3, 2 / 3, 1]:
            position = (start[0] + delta[0] * fraction, start[1] + delta[1] * fraction)
            coordinates = transform_for_pointer(position[0] - logical["x"], position[1] - logical["y"], logical["width"], logical["height"], logical.get("pointer_transform", logical["transform"]))
            pointer.move_abs(*coordinates)
            time.sleep(0.08)
    finally:
        pointer.release()
    time.sleep(0.1)


def launch(binary, environment, output):
    """【OCR 置顶测试】【窗口启动】用 binary、environment 创建实际结果窗口，output 保存协议，返回进程；失败只终止本次子进程"""
    process = subprocess.Popen([str(binary), "--ocr-result", "--nonotify"], env=dict(environment, WAYLAND_DEBUG="client"), stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=output)
    process.stdin.write(result_frame())
    process.stdin.close()
    try:
        assert select.select([process.stdout], [], [], 8)[0] and process.stdout.readline() == b"\n"
        return process
    except BaseException:
        process.terminate()
        process.wait(timeout=3)
        raise


def startup_only(binary, root, xml):
    """【OCR 置顶测试】【首次置顶】按 binary、root、xml 验证保存开关直接创建浮层，不短暂创建普通窗口，不继承贴图配置，无返回值"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    config = configure(root)
    config.write_text(config.read_text() + "\n[pinned]\nalways_on_top = false\n[ocr_result_window]\nalways_on_top = true\n")
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    client = build(root / "client", xml)
    log = root / "process.log"
    with session.input_context(), pointer_context(), log.open("w") as output:
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        method = InputMethod(client, environment, root)
        process = None
        try:
            process = launch(binary, environment, output)
            window = layer_window(log)
            assert ".get_toplevel(" not in log.read_text()
            checks = Checks(method, window, root, log, environment)
            checks.focus("source", "base")
            checks.batch("baseX", commit="X")
            checks.full_copy("initial-layer-source", "baseX")
            screenshot(root, "initial-layer")
            session.key("Escape")
            assert process.wait(timeout=3) == 0
            assert not roles(log.read_text())
            proof = {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "startup_direct_layer": True, "no_temporary_xdg_window": True, "independent_pinned_image_option": True, "native_input": True, "clipboard": checks.clipboard, "no_ocr_or_external_service": True}
            (root / "result.json").write_text(json.dumps(proof, ensure_ascii=False, indent=2) + "\n")
        finally:
            if process and process.poll() is None:
                session.key("Escape")
                try:
                    process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    process.terminate()
                    process.wait(timeout=3)
            method.close()


def run(binary, root, xml):
    """【OCR 置顶测试】【完整场景】用 binary、root、xml 驱动真实窗口和输入法；保存正文、资源与协议，不启动模型或外部服务"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    config = configure(root)
    text = re.sub(r"(timeout_ms|timeoutMs)\s*=\s*\d+", r"\1 = 60000", config.read_text())
    config.write_text(text + "\n[pinned]\nalways_on_top = true\n")
    (root / "mode").write_text("success")
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
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
            wait_role(log, False)
            reference = json.loads(Path("tests/fixtures/qt-ocr-stacking-menu.json").read_text())["operations"]
            checks = Checks(method, window, root, log, environment, expected=reference)
            checks.focus("source", "base")
            checks.batch("baseX", commit="X")
            checks.full_copy("before-role-change", "baseX")
            # 1. 【OCR 置顶测试】【独立角色】真实左键释放保存独立开关，旧 xdg 角色必须销毁，编辑正文保持不变
            click(pin(window))
            wait_config(config, True)
            wait_role(log, True)
            method.state(active=False)
            method.batch(commit="stale-role")
            window = layer_window(log)
            checks.window = window
            checks.focus("source", "baseX")
            checks.full_copy("first-layer-source", "baseX")
            screenshot(root, "first-layer")
            original = window.copy()
            drag((window["x"] + window["width"] / 2, window["y"] + 19), (28, 18))
            window = layer_window(log)
            assert abs(window["x"] - original["x"] - 28) <= 1 and abs(window["y"] - original["y"] - 18) <= 1
            assert (window["width"], window["height"]) == (original["width"], original["height"])
            original = window.copy()
            drag((window["x"] + (window["width"] - 18) // 2 + 10, window["y"] + window["height"] - 13), (20, 14))
            window = layer_window(log)
            assert abs(window["width"] - original["width"] - 20) <= 1 and abs(window["height"] - original["height"] - 14) <= 1
            assert (window["x"], window["y"]) == (original["x"], original["y"])
            checks.window = window
            checks.focus("source", "baseX")
            checks.full_copy("after-layer-geometry", "baseX")
            screenshot(root, "moved-and-resized-layer")
            current_role = roles(log.read_text())[0]
            # 2. 【OCR 置顶测试】【右键隔离】右键置顶按钮只改变焦点，不保存或重新创建角色
            click(pin(window), "right")
            time.sleep(0.1)
            assert wait_config(config, True) and roles(log.read_text())[0] == current_role
            # 3. 【OCR 置顶测试】【角色往返】取消置顶后保留同一按钮焦点，Space 完成后重新置顶，不借助窗口管理器改写尺寸
            click(pin(window))
            wait_config(config, False)
            wait_role(log, False)
            method.state(active=False)
            method.batch(commit="stale-role")
            session.key("Space")
            wait_config(config, True)
            window = layer_window(log)
            checks.window = window
            checks.focus("source", "baseX")
            checks.full_copy("roundtrip-source", "baseX")
            # 4. 【OCR 置顶测试】【浮层菜单】实际 layer-shell 主表面挂接 xdg 弹层，候选不进入正文，过期提交仍拒绝
            menu_evidence = checks.menu("source", "baseX", "source-menu")
            checks.full_copy("after-layer-menu", "baseX")
            previous = method.states[-1]["serial"]
            session.key("ctrl+z")
            newer_state(method, previous, "base")
            previous = method.states[-1]["serial"]
            session.key("ctrl+shift+z")
            newer_state(method, previous, "baseX")
            checks.full_copy("role-history-preserved", "baseX")
            # 5. 【OCR 置顶测试】【任务保持】请求只含确认文字，置顶往返不能取消、重启或丢失当前翻译句柄
            (root / "mode").write_text("wait")
            session.key("ctrl+Return")
            record = requests(root, 1)[0]
            assert record["source"]["tokens"][0]["text"] == "baseX"
            click(pin(window))
            wait_config(config, False)
            wait_role(log, False)
            assert Path(f"/proc/{record['pid']}").exists()
            session.key("Space")
            wait_config(config, True)
            window = layer_window(log)
            checks.window = window
            assert len(list((root / "requests").glob("*.json"))) == 1
            assert Path(f"/proc/{record['pid']}").exists()
            click(point(window, "action"))
            reclaimed(record)
            checks.focus("source", "baseX")
            checks.full_copy("after-task-cancel", "baseX")
            # 6. 【OCR 置顶测试】【保存失败】私有非法配置触发实际保存失败，按钮和角色均回退，原文件不能改写
            click(pin(window), "right")
            time.sleep(0.1)
            baseline = screenshot(root, "before-save-failure")
            healthy = config.read_text()
            invalid = re.sub(r"(\[ocr_result_window\]\s*always_on_top\s*=\s*)true", r"\1'invalid'", healthy)
            assert invalid != healthy
            config.write_text(invalid)
            before_roles = roles(log.read_text())
            click(pin(window))
            time.sleep(0.15)
            assert config.read_text() == invalid
            assert roles(log.read_text()) == before_roles
            failed = screenshot(root, "save-failed")
            footer = (window["x"], window["y"] + window["height"] - 26, window["x"] + window["width"], window["y"] + window["height"])
            assert ImageChops.difference(baseline.crop(footer), failed.crop(footer)).getbbox() is not None
            time.sleep(2.6)
            expired = screenshot(root, "failure-expired")
            assert ImageChops.difference(baseline.crop(footer), expired.crop(footer)).getbbox() is None
            config.write_text(healthy)
            checks.full_copy("final-source", "baseX")
            time.sleep(2.1)
            resources = idle(process.pid, log)
            assert not (root / "data/mark-shot/models").exists()
            evidence = protocol_evidence(log.read_text())
            assert evidence["native_protocol"] and evidence["preedit_delivered"]
            # 7. 【OCR 置顶测试】【独立退出】角色切换不能重复发送 ready，关闭后客户端、角色和所属命令全部回收
            assert not select.select([process.stdout], [], [], 0)[0]
            session.key("Escape")
            assert process.wait(timeout=4) == 0
            proof = {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "compositor": "sway" if session.backend() is session.sway else "niri", "independent_config": True, "actual_role_roundtrip": True, "layer_drag_and_resize": True, "source_preserved": True, "undo_redo_preserved": True, "stale_role_and_menu_commits_rejected": True, "layer_popup_attached": True, "menu": menu_evidence, "running_task_preserved": True, "failure_rolls_back_and_expires": True, "ready_not_repeated": True, "resources": resources, "clipboard": checks.clipboard, "final_roles": roles(log.read_text()), "no_ocr_or_external_service": True, **evidence}
            (root / "result.json").write_text(json.dumps(proof, ensure_ascii=False, indent=2) + "\n")
        finally:
            if process and process.poll() is None:
                for _ in range(2):
                    session.key("Escape")
                    try:
                        process.wait(timeout=2)
                        break
                    except subprocess.TimeoutExpired:
                        pass
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=3)
            method.close()


def main():
    """【OCR 置顶测试】【入口】解析本地构建、私有证据目录和协议路径，失败返回非零状态，无参数"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--startup-only", action="store_true")
    parser.add_argument("--xml", type=Path, default=Path("target/test-tools/input-method/input-method-unstable-v2.xml"))
    options = parser.parse_args()
    (startup_only if options.startup_only else run)(options.binary.resolve(), options.root.resolve(), options.xml.resolve())


if __name__ == "__main__":
    main()
