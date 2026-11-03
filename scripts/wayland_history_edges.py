"""【截图历史测试】【边界流程】首次禁用、忙碌快捷键、长图与完整目录导航"""

from pathlib import Path
import json
import subprocess
import time

import numpy as np
from PIL import Image

from wayland_filter_fixture import source_image
import wayland_history_actions as actions
import wayland_history_app as app
import wayland_history_checks as checks
import wayland_test_session as session


def stop(process):
    """【截图历史测试】【异常清理】只终止本测试的 process；正常结束时不执行操作，无返回值"""
    if process.poll() is None:
        process.terminate()
        process.wait(timeout=3)


def first_disabled(binary, artifacts):
    """【截图历史测试】【首次禁用】使用 binary 验证捕获和空历史窗口不创建目录；证据写入 artifacts，返回断言结果"""
    artifacts.mkdir()
    environment, config, history, _default = app.configuration(artifacts)
    config.write_text(config.read_text().replace("enabled = true", "enabled = false"))
    app.headless(binary, environment, config, artifacts / "capture.png")
    if history.exists():
        raise AssertionError("a first capture created a disabled history directory")
    process = app.launch(binary, environment, config, ["--history"], artifacts / "history-process.log")
    try:
        session.size_window(actions.window(pid=process.pid), 360, 360)
        resources = checks.idle(process.pid)
        session.screenshot(artifacts / "disabled-empty-window.png")
        if history.exists():
            raise AssertionError("the empty disabled window created a history directory")
        session.key("Escape")
        app.wait_exit(process)
        return {"capture_created_no_directory": True, "window_created_no_directory": True, "idle": resources}
    finally:
        stop(process)


def shortcut_guard(binary, artifacts):
    """【截图历史测试】【快捷键保护】验证 binary 拖动期间忽略 Ctrl+H，结束后完整交接；artifacts 保存真实输入证据，返回结果"""
    artifacts.mkdir()
    environment, config, history, _default = app.configuration(artifacts)
    source = artifacts / "source.png"
    source_image().save(source)
    process = app.launch(binary, environment, config, ["--fullscreen", "--fullscreen-default-tool", "pen", "--to", "", source], artifacts / "editor-process.log")
    history_window = None
    try:
        app.wait_editor(process, artifacts / "before-gesture.png")
        before = checks.rgba(artifacts / "before-gesture.png")
        # 1. 【截图历史测试】【真实忙碌】保留输入连接和鼠标按下状态，不能用已经结束的拖动冒充忙碌手势
        with subprocess.Popen(["niri-use", "--child", "click", "--logical", "330", "250", "--hold", "3"], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE) as gesture:
            time.sleep(0.2)
            session.control("move", "--logical", 440, 300)
            time.sleep(0.1)
            session.screenshot(artifacts / "active-gesture.png")
            active = checks.rgba(artifacts / "active-gesture.png")
            if gesture.poll() is not None or np.count_nonzero(np.any(before != active, axis=2)) < 20:
                raise AssertionError("the shortcut probe did not have an observable active gesture")
            session.key("ctrl+h")
            time.sleep(0.3)
            windows = session.windows()
            session.screenshot(artifacts / "ignored-shortcut.png")
            (artifacts / "shortcut-state.json").write_text(json.dumps({"editor_exit": process.poll(), "gesture_exit": gesture.poll(), "windows": windows}, indent=2) + "\n")
            if process.poll() is not None or any(item["app_id"] == "mark-shot-history" for item in windows):
                raise AssertionError("Ctrl+H interrupted an active screenshot gesture")
            gesture.wait(timeout=5)
            if gesture.returncode:
                raise RuntimeError(gesture.stderr.read().decode())
        # 2. 【截图历史测试】【完成交接】首帧就绪后原截图进程正常取消，独立历史窗口继续运行且不保存取消截图
        session.key("ctrl+h")
        app.wait_exit(process)
        history_window = actions.window()
        if history_window["pid"] == process.pid:
            raise AssertionError("history did not run as an independent helper")
        resources = checks.idle(history_window["pid"])
        session.screenshot(artifacts / "handoff-history.png")
        if checks.owned(history):
            raise AssertionError("opening history recorded the canceled screenshot")
        actions.focus(history_window)
        session.key("Escape")
        app.wait_removed(history_window["id"])
        history_window = None
        return {"active_drag_shortcut_ignored": True, "ready_handoff": True, "canceled_capture_not_recorded": True, "idle": resources}
    finally:
        stop(process)
        if history_window is not None:
            session.control("close", history_window["id"])


def picker_path(environment, expected):
    """【截图历史测试】【导航断言】等待 environment 中当前路径字段等于 expected；返回规范路径，失败报告实际字段"""
    session.key("ctrl+l")
    deadline = time.monotonic() + 5
    actual = ""
    while time.monotonic() < deadline:
        actual = actions.field_text(environment)
        if actual == str(expected.resolve()):
            return actual
        time.sleep(0.04)
    raise AssertionError(f"picker directory: expected {expected}, found {actual!r}")


def picker_button(item, name):
    """【截图历史测试】【导航按钮】按 item 实际对话框几何点击 name；无返回值，不使用原生文件对话框"""
    x, y, _width, _height = actions.picker_geometry(item)
    session.click(item, x + {"home": 36, "up": 84, "hidden": 143, "new_folder": 226}[name], y + 88)


def picker_first_row(item):
    """【截图历史测试】【目录行】单击 item 对话框中的首行目录并直接进入；无返回值，不追加保存按键"""
    x, y, _width, _height = actions.picker_geometry(item)
    session.click(item, x + 100, y + 124)


def picker_keyboard_row(item):
    """【截图历史测试】【键盘目录】在 item 列表空行取得焦点，再用 Up、Enter 选择首行目录；无返回值"""
    x, y, _width, _height = actions.picker_geometry(item)
    session.click(item, x + 100, y + 152)
    session.key("Up")
    session.key("Return")


def directory_navigation(item, environment, expected, artifacts):
    """【截图历史测试】【目录流程】验证 item 中 Home、Up、隐藏目录和 Unicode 新建；保存图像须等于 expected，返回结果"""
    destination = artifacts / "navigation"
    destination.mkdir()
    (destination / "visible").mkdir()
    (destination / ".hidden").mkdir()
    actions.footer(item, "save")
    time.sleep(0.2)
    actions.navigate(item, environment, destination)
    picker_button(item, "home")
    picker_path(environment, Path(environment["HOME"]))
    actions.navigate(item, environment, destination / "visible")
    picker_button(item, "up")
    picker_path(environment, destination)
    picker_first_row(item)
    picker_path(environment, destination / "visible")
    session.key("alt+Up")
    picker_path(environment, destination)
    picker_keyboard_row(item)
    picker_path(environment, destination / "visible")
    session.key("alt+Up")
    picker_path(environment, destination)
    picker_button(item, "hidden")
    picker_path(environment, destination)
    picker_first_row(item)
    picker_path(environment, destination / ".hidden")
    picker_button(item, "up")
    picker_path(environment, destination)
    actions.filename(item, environment, "navigation-copy.png")

    # 1. 【截图历史测试】【新建取消】Escape 只取消新建步骤，原保存对话框和文件名保持不变
    picker_button(item, "new_folder")
    actions.filename(item, environment, "cancelled-folder")
    session.key("Escape")
    if (destination / "cancelled-folder").exists() or actions.field_text(environment) != "navigation-copy.png":
        raise AssertionError("canceling a new folder changed the save name or created a directory")
    # 2. 【截图历史测试】【新建完成】新建只刷新当前列表，保留原文件名，再显式进入新目录
    picker_button(item, "new_folder")
    actions.filename(item, environment, "新建目录")
    actions.save(item)
    created = destination / "新建目录"
    deadline = time.monotonic() + 5
    while not created.is_dir() and time.monotonic() < deadline:
        time.sleep(0.04)
    if not created.is_dir():
        raise AssertionError("the Unicode directory was not created")
    session.wait_single_thread(item["pid"])
    picker_path(environment, destination)
    actions.navigate(item, environment, created)
    picker_path(environment, created)
    x, y, width, height = actions.picker_geometry(item)
    session.click(item, x + width / 2, y + height - 66)
    if actions.field_text(environment) != "navigation-copy.png":
        raise AssertionError("creating a directory discarded the original save name")
    session.screenshot(artifacts / "created-folder-picker.png")
    actions.save(item)
    saved = actions.wait_file(created / "navigation-copy.png")
    checks.equal_image(saved, expected)
    return {"home": True, "up_button": True, "alt_up": True, "keyboard_directory_navigation": True, "hidden_directory_toggle": True, "new_folder_cancel": True, "unicode_new_folder": True, "save_name_preserved": True}


def tall_source(path):
    """【截图历史测试】【长图原件】在 path 创建 144×20000 的透明条纹 PNG；返回原图尺寸，不使用旧高度上限"""
    rows = np.arange(20_000)
    colors = np.array([[0, 0, 0, 0], [128, 64, 32, 128], [200, 120, 40, 255]], dtype=np.uint8)
    pixels = np.broadcast_to(colors[(rows // 113) % len(colors), None, :], (20_000, 144, 4)).copy()
    Image.fromarray(pixels).save(path)
    return [144, 20_000]


def long_reedit(binary, artifacts):
    """【截图历史测试】【长图重编辑】使用 binary 完成长图记录、独立重编辑和目录保存；artifacts 保存全部像素证据，返回结果"""
    artifacts.mkdir()
    environment, config, history, _default = app.configuration(artifacts)
    source = artifacts / "long-source.png"
    size = tall_source(source)
    original = app.capture_image(binary, environment, config, source, artifacts / "original-export", artifacts, "long-original")
    checks.equal_image(original, source)
    record = checks.wait_count(history, 1)[0]
    process = app.launch(binary, environment, config, ["--history"], artifacts / "history-process.log")
    editor = None
    try:
        item = session.size_window(actions.window(pid=process.pid), 760, 460)
        actions.footer(item, "edit")
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            editors = [window for window in session.windows() if window["app_id"] != "mark-shot-history"]
            if len(editors) == 1:
                editor = editors[0]
                break
            time.sleep(0.04)
        if editor is None:
            raise AssertionError("the long history image did not open an independent editor")
        session.wait_single_thread(editor["pid"])
        actions.focus(editor)
        session.screenshot(artifacts / "long-reedit.png")
        session.key("ctrl+c")
        app.wait_removed(editor["id"])
        editor = None
        copied = artifacts / "long-reedit-copy.png"
        checks.clipboard(environment, original, copied)
        checks.wait_count(history, 2)
        if list(Image.open(copied).size) != size:
            raise AssertionError("the long history image was truncated or resized")
        actions.focus(item)
        navigation = directory_navigation(item, environment, record, artifacts)
        checks.wait_count(history, 2)
        resources = checks.idle(process.pid)
        session.key("Escape")
        app.wait_exit(process)
        return {"source_size": size, "complete_rgba_preserved": True, "reedit_added_one_record": True, "navigation_save_added_no_record": True, "directory_navigation": navigation, "idle": resources}
    finally:
        stop(process)
        if editor is not None:
            session.control("close", editor["id"])
