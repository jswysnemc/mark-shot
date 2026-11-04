#!/usr/bin/env python3
"""【截图历史测试】【独立确认】私有桌面验证同进程父子窗口、取消与鼠标职责，不以模态提示代替输入隔离"""

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import time

from PIL import Image

import wayland_history_actions as actions
import wayland_history_checks as checks
from wayland_history_app import capture_image, configuration, launch, wait_exit, wait_removed
import wayland_test_session as session
from wayland_test_keyboard import focused_environment
from wayland_text_pointer import click, pointer_context


def owned_snapshot(history):
    """【截图历史测试】【文件身份】返回 history 当前所有合法记录的字节摘要，保留损坏记录和身份断言"""
    return {path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in checks.owned(history)}


def open_dialog(parent, root, name):
    """【截图历史测试】【子窗口定位】从 parent 清空按钮打开唯一子窗口，root 保存 name 画面，返回原版尺寸的同进程窗口"""
    actions.footer(parent, "clear")
    dialog = actions.window("mark-shot-history-clear", pid=parent["pid"])
    assert dialog["pid"] == parent["pid"], "clear confirmation spawned a separate process"
    assert dialog["size"] == [315, 96], "clear confirmation did not use the real Qt default size"
    assert dialog["floating"] and dialog["floating_pos"], "the modal child was not an independent floating window"
    assert len(session.windows()) == 2, "clear confirmation created an extra or missing window"
    actions.focus(dialog)
    checks.idle(parent["pid"])
    session.screenshot(root / (name + ".png"))
    return dialog


def button_point(dialog, label):
    """【截图历史测试】【真实按钮位置】按冻结 Qt 样本返回 dialog 内 label 的全局中心，不复用旧窗口内确认层坐标"""
    fixture = json.loads((Path(__file__).resolve().parents[1] / "tests/fixtures/qt-history-dialog.json").read_text())
    x, y, width, height = next(button["rect"] for button in fixture["layout"]["buttons"] if button["text"] == label)
    left, top = dialog["floating_pos"]
    return left + x + width / 2, top + y + height / 2


def keyboard(value, pressed):
    """【截图历史测试】【按下释放】在已验证私有 Xvfb 输入端向 value 单独发送 pressed 状态，不连接用户显示服务器，无返回值"""
    subprocess.run(["xdotool", "keydown" if pressed else "keyup", value], env=focused_environment(), check=True, timeout=3)
    time.sleep(0.1)


def run(binary, root):
    """【截图历史测试】【确认流程】用 binary 运行全部私有确认案例，root 保存同指纹协议、画面和文件证据，无返回值"""
    root.mkdir(parents=True, exist_ok=False)
    environment, config, history, default = configuration(root)
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    unchanged = default.read_bytes()
    source = root / "source.png"
    Image.new("RGBA", (384, 240), (45, 110, 190, 91)).save(source)
    original = capture_image(binary, environment, config, source, root / "export", root, "source")
    record = checks.wait_count(history, 1)[0]
    checks.equal_image(record, original)
    broken = history / "capture-20000101000000001-22222222222222222222222222222222.png"
    broken.write_bytes(b"broken PNG")
    unrelated = history / "keep.txt"
    unrelated.write_text("keep")
    linked = history / "capture-20000101000000000-11111111111111111111111111111111.png"
    linked.symlink_to(original)
    before = owned_snapshot(history)
    protocol_environment = dict(environment, WAYLAND_DEBUG="client")
    process = launch(binary, protocol_environment, config, ["--history"], root / "history-process.log")
    try:
        parent = session.size_window(actions.window(pid=process.pid), 800, 520)
        # 1. 【截图历史测试】【取消行为】默认 Enter、Escape 和窗口管理器关闭都只关闭子窗口，不读取或删除记录
        for name in ("default-enter", "ctrl-enter", "alt-enter", "escape", "wm-close"):
            dialog = open_dialog(parent, root, name)
            if name == "wm-close":
                session.control("close", dialog["id"])
            else:
                session.key({"default-enter": "Return", "ctrl-enter": "ctrl+Return", "alt-enter": "alt+Return", "escape": "Escape"}[name])
            wait_removed(dialog["id"])
            assert process.poll() is None and owned_snapshot(history) == before, name + " changed history or closed the parent"
        # 2. 【截图历史测试】【父窗口隔离】模态时复制、删除和再次清空不能穿透，右键只改变确认按钮焦点
        dialog = open_dialog(parent, root, "input-isolation")
        session.key("ctrl+Escape")
        assert len(session.windows()) == 2, "modified Escape incorrectly closed the Qt-style dialog"
        session.control("copy", "parent modal sentinel")
        actions.focus(parent)
        session.key("ctrl+c")
        assert session.read_clipboard(environment) == "parent modal sentinel", "parent copy leaked through a modal dialog"
        actions.footer(parent, "delete")
        assert owned_snapshot(history) == before and len(session.windows()) == 2, "parent Delete leaked through modality"
        actions.focus(dialog)
        with pointer_context() as pointer:
            point = button_point(dialog, "Clear History")
            click(point, "right")
            assert owned_snapshot(history) == before and len(session.windows()) == 2, "right-click cleared history"
            session.screenshot(root / "right-clear-focus.png")
            # 3. 【截图历史测试】【释放取消】在清空按钮按下后移出并释放，必须保持对话框和原记录
            from wayland_virtual_pointer import transform_for_pointer
            runtime = json.loads((Path(environment["XDG_RUNTIME_DIR"]) / "runtime.json").read_text())
            logical = next(iter(runtime["outputs"].values()))["logical"]
            points = [point, (dialog["floating_pos"][0] + 3, dialog["floating_pos"][1] + 3)]
            positions = [transform_for_pointer(x - logical["x"], y - logical["y"], logical["width"], logical["height"], logical["transform"]) for x, y in points]
            pointer.move_abs(*positions[0])
            pointer.press()
            time.sleep(0.1)
            assert owned_snapshot(history) == before, "left press cleared history before release"
            pointer.move_abs(*positions[1])
            pointer.release()
            time.sleep(0.1)
            assert owned_snapshot(history) == before and len(session.windows()) == 2, "release outside activated Clear History"
        actions.focus(dialog)
        session.key("Escape")
        wait_removed(dialog["id"])
        # 4. 【截图历史测试】【父窗口关闭】关闭父窗口必须回收子窗口，不执行隐藏确认或保留辅助进程
        dialog = open_dialog(parent, root, "parent-close")
        session.control("close", parent["id"])
        wait_exit(process)
        wait_removed(dialog["id"])
        assert owned_snapshot(history) == before and not session.windows(), "parent close accepted or orphaned its dialog"
        process = launch(binary, protocol_environment, config, ["--history"], root / "history-confirm-process.log")
        parent = session.size_window(actions.window(pid=process.pid), 800, 520)
        dialog = open_dialog(parent, root, "space-confirm")
        # 5. 【截图历史测试】【明确确认】Tab 选择清空，Space 按下保留记录，释放才提交一次清空，合法损坏记录也在清理范围
        session.key("Tab")
        keyboard("ctrl+space", True)
        try:
            assert owned_snapshot(history) == before and len(session.windows()) == 2, "Space press deleted records"
            session.screenshot(root / "space-pressed.png")
        finally:
            keyboard("ctrl+space", False)
        wait_removed(dialog["id"])
        checks.wait_count(history, 0)
        assert unrelated.read_text() == "keep" and linked.is_symlink() and original.exists() and not broken.exists(), "confirmed Clear crossed the file identity boundary"
        resources = checks.idle(process.pid)
        assert process.poll() is None and len(session.windows()) == 1, "confirming closed the parent window"
        session.screenshot(root / "cleared-parent.png")
        session.key("Escape")
        wait_exit(process)
        assert default.read_bytes() == unchanged and hashlib.sha256(binary.read_bytes()).hexdigest() == fingerprint
        protocol = (root / "history-process.log").read_text() + (root / "history-confirm-process.log").read_text()
        parent_protocol = bool(re.search(r"xdg_toplevel@\d+\.set_parent\(.*xdg_toplevel@", protocol))
        modal_protocol = "get_xdg_dialog" in protocol and ".set_modal(" in protocol
        assert parent_protocol, "the independent dialog did not declare its native parent relationship"
        if "get_xdg_dialog" in protocol:
            assert modal_protocol, "advertised dialog protocol did not set modality"
        result = {"sha256": fingerprint, "binary_bytes": binary.stat().st_size, "same_process_child": True, "native_parent_relationship": parent_protocol, "native_modal_protocol": modal_protocol, "default_enter_canceled": True, "modified_enter_used_default_cancel": True, "modified_escape_did_not_cancel": True, "modified_space_release_confirmed": True, "escape_canceled": True, "wm_close_canceled": True, "parent_copy_delete_isolated": True, "right_click_did_not_confirm": True, "mouse_press_and_outside_release_kept_records": True, "parent_close_reclaimed_child": True, "space_release_confirmed_once": True, "owned_corrupt_record_removed": True, "unrelated_files_and_links_preserved": True, "default_config_unchanged": True, "dialog_size": [315, 96], "parent_idle": resources, "exact_qt_pixels_proven": False}
        (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result))
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=3)


def main():
    """【截图历史测试】【入口】解析二进制和新证据目录，在已验证私有会话执行确认流程，无参数，失败返回非零状态"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, required=True)
    options = parser.parse_args()
    session.environment()
    run(options.binary.resolve(), options.artifacts.resolve())


if __name__ == "__main__":
    main()
