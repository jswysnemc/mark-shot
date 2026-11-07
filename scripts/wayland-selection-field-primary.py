#!/usr/bin/env python3
"""【字段主选择回归】【真实会话】共用 Qt 与 Rust 输入序列，验证合并移动、主选择复制、中键粘贴和会话释放"""

import argparse
from contextlib import contextmanager
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import time

from wayland_virtual_pointer import transform_for_pointer

PROJECT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("field_navigation_support", PROJECT / "scripts/wayland-selection-field-navigation.py")
NAV = importlib.util.module_from_spec(spec)
spec.loader.exec_module(NAV)
SIZE, INPUT, session = NAV.SIZE, NAV.SIZE.INPUT, NAV.session


def put(environment, text, primary=False):
    """【字段主选择回归】【外部设置】在 environment 写入 text，primary 选择主选择或普通剪贴板，返回空值"""
    subprocess.run(["wl-copy", "--type", "text/plain;charset=utf-8", *(["--primary"] if primary else [])], env=environment,
                   input=text.encode(), check=True, timeout=3)


def clipboard(environment, expected, primary=False):
    """【字段主选择回归】【外部读取】在 environment 等待 expected 完整文本，primary 控制类型，返回实际字符串"""
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        result = subprocess.run(["wl-paste", "--no-newline", *(["--primary"] if primary else [])],
                                env=environment, capture_output=True, timeout=2)
        if result.returncode == 0 and result.stdout.decode() == expected:
            return expected
        time.sleep(0.01)
    raise AssertionError(f"private {'primary' if primary else 'regular'} clipboard differed: {result.stdout!r} != {expected!r}")


def empty(environment):
    """【字段主选择回归】【退出释放】等待 environment 的主选择数据源消失，返回空值，不清除外部剪贴板代替检查"""
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        result = subprocess.run(["wl-paste", "--primary", "--no-newline"], env=environment,
                                capture_output=True, timeout=2)
        if result.returncode != 0:
            return
        time.sleep(0.01)
    raise AssertionError(f"primary source remained after application exit: {result.stdout!r}")


def replace(method, text):
    """【字段主选择回归】【正文替换】在 method 当前字段用标准全选粘贴 text，等待完整周边文本后返回状态"""
    session.key("ctrl+a")
    previous = method.states[-1]["serial"]
    session.paste_text(text)
    return NAV.changed(method, previous, text, len(text), len(text))


def rapid(button="left"):
    """【字段主选择回归】【原位点击】用当前已校验设备发送 button，不重新移动指针，始终释放按钮，无返回值"""
    NAV.button(button, True)
    try:
        time.sleep(0.015)
    finally:
        NAV.button(button, False)


def burst(points):
    """【字段主选择回归】【连续移动】在当前设备投递 points，无额外等待，返回空值，私有 X11 也保持原坐标转换"""
    pointer, logical, _, x11 = INPUT.CURRENT.get()
    for point in points:
        if pointer is None:
            environment, mode = x11
            pixel = (round((point[0] - logical["x"]) * mode["width"] / logical["width"]),
                     round((point[1] - logical["y"]) * mode["height"] / logical["height"]))
            subprocess.run(["xdotool", "mousemove", "--sync", str(pixel[0]), str(pixel[1])],
                           env=environment, check=True, timeout=3)
        else:
            pointer.move_abs(*transform_for_pointer(point[0], point[1], logical["width"], logical["height"],
                                                   logical.get("pointer_transform", logical["transform"])))


def processed(log, previous):
    """【字段主选择回归】【按钮交付】等待 log 晚于 previous 的实际左键按下，返回最新协议时间，不以等待时长代替交付"""
    pattern = r"\.button,?\s*\(\s*\d+,\s*(\d+),\s*272,\s*1\s*\)"
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        stamps = list(map(int, re.findall(pattern, log.read_text())))
        if len(stamps) > previous:
            return stamps[-1]
        time.sleep(0.01)
    raise AssertionError("the private pointer press was not delivered to the application")


def selected(method, root, qt, previous, text, cursor, anchor, primary):
    """【字段主选择回归】【真实选择】按 qt 读取控件或 method 原生状态，核对 text、cursor、anchor 与 primary，返回实际上下文"""
    clipboard(session.environment(), primary, True)
    if not qt:
        return NAV.changed(method, previous, text, cursor, anchor)
    deadline = time.monotonic() + 3
    path = root / "qt-field.json.states"
    while time.monotonic() < deadline:
        if path.exists():
            records = [json.loads(line) for line in path.read_text().splitlines()]
            if records:
                value = records[-1]["state"]
                start = value["selection_start"]
                end = start + len(value["selected"])
                actual_anchor = start if value["cursor"] == end else end
                if (value["text"], value["cursor"], actual_anchor) == (text, cursor, anchor):
                    return {"active": True, "text": text, "cursor": cursor, "anchor": actual_anchor,
                            "control_state": value, "input_method_state": NAV.context(method)}
        time.sleep(0.01)
    raise AssertionError(f"actual Qt selection did not match {(text, cursor, anchor)}: {records[-3:]}")


@contextmanager
def application(binary, environment, root, qt):
    """【字段主选择回归】【本次进程】用 binary 与 environment 启动 qt 或 Rust；root 保存日志，返回进程、日志和字段矩形"""
    if qt:
        metadata = root / "qt-field.json"
        path = root / "qt.log"
        with path.open("w") as log:
            process = subprocess.Popen([str(binary), str(metadata), "--interactive"],
                env=dict(environment, QT_QPA_PLATFORM="wayland", QT_IM_MODULE="wayland", QT_FONT_DPI="96",
                         QT_WAYLAND_DISABLE_WINDOWDECORATION="1", WAYLAND_DEBUG="client"),
                stdout=subprocess.DEVNULL, stderr=log, start_new_session=True)
            try:
                node = INPUT.EXPORT.wait_window(environment, "mark-shot-field-reference", process.pid)
                geometry = INPUT.PIN.geometry(environment, node, (600, 360))
                deadline = time.monotonic() + 3
                while not metadata.exists() and time.monotonic() < deadline:
                    time.sleep(0.01)
                values = json.loads(metadata.read_text())
                assert values["primary_supported"], values
                rect = values["field_rect"]
                yield process, path, (geometry["x"] + rect[0], geometry["y"] + rect[1], rect[2], rect[3])
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=3)
    else:
        config = root / "config.toml"
        config.write_text('[general]\noverlay="window"\nto=""\n[notifications]\nenabled=false\n[history]\nenabled=false\n')
        with INPUT.launch(binary, environment, config, None, root, "capture") as (process, log):
            INPUT.drag((0, 0), (300, 180))
            session.key("ctrl+r")
            rect = json.loads((PROJECT / "tests/fixtures/qt-selection-size/reference.json").read_text())["hud_rects"]["width"]
            yield process, log, (6 + rect[0], 6 + rect[1], rect[2], rect[3])


def motion_case(method, process, log, point):
    """【字段主选择回归】【合并移动】暂停本次 process 后送达移出和返回，再验证第二次点击，log 记录原始时间，返回状态及间隔"""
    NAV.key(method, "Home", "+1200", 0, 0)
    NAV.click(method, point, "+1200", 5, 5)
    before = NAV.context(method)
    pattern = r"\.button,?\s*\(\s*\d+,\s*(\d+),\s*272,\s*1\s*\)"
    stamps = list(map(int, re.findall(pattern, log.read_text())))
    first = stamps[-1]
    # 1. 【字段主选择回归】【排队输入】只暂停本次私有进程，合成器保持运行，确保应用一次接收到两次不同位置
    os.kill(process.pid, signal.SIGSTOP)
    try:
        burst([(point[0] - 15, point[1]), point])
    finally:
        os.kill(process.pid, signal.SIGCONT)
    rapid()
    last = processed(log, len(stamps))
    assert 0 <= last - first < 400, (first, last)
    after = NAV.unchanged(method, before)
    return {"before": before, "after": after, "native_interval_ms": last - first}


def workflow(binary, root, qt, motions_only):
    """【字段主选择回归】【完整流程】在私有会话验证 binary，root 保存协议、事件和剪贴板结果，qt 与 motions_only 控制范围"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"),
        XDG_STATE_HOME=str(root / "state"), TMPDIR=str(root / "temporary"), MARK_SHOT_LANG="english",
        DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/field-primary-test")
    Path(environment["TMPDIR"]).mkdir()
    report = {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "compositor": session.backend().__name__, "qt": qt}
    with session.input_context(), SIZE.pointer_context(backend="x11" if session.backend() is session.niri else "virtual"), SIZE.input_method(environment, root / "input-method") as method:
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        with application(binary, environment, root, qt) as (process, log, rect):
            method.wait(lambda line: line.startswith("state\t") and method.states[-1]["active"])
            if NAV.context(method)["text"] != "+1200":
                replace(method, "+1200")
            point = (rect[0] + rect[2] - 8, rect[1] + rect[3] / 2)
            put(environment, "regular-sentinel")
            put(environment, "primary-sentinel", True)
            report["field_rect"] = rect
            report["motion"] = motion_case(method, process, log, point)
            if not motions_only:
                # 2. 【字段主选择回归】【普通复制】选择只进入主选择，普通剪贴板保持独立
                time.sleep(0.45)
                NAV.key(method, "Home", "+1200", 0, 0)
                NAV.click(method, point, "+1200", 5, 5)
                clipboard(environment, "primary-sentinel", True)
                previous = method.states[-1]["serial"]
                rapid()
                report["word"] = selected(method, root, qt, previous, "+1200", 5, 1, "1200")
                clipboard(environment, "regular-sentinel")
                previous = method.states[-1]["serial"]
                # 3. 【字段主选择回归】【右键插入】原位快速插入右键再左键，计时器保持活动且主选择更新为全文
                rapid("right")
                rapid()
                report["right_then_triple"] = selected(method, root, qt, previous, "+1200", 5, 0, "+1200")
                clipboard(environment, "regular-sentinel")

                # 4. 【字段主选择回归】【中键粘贴】主选择先插入数字，再拒绝非法文字，普通剪贴板不参与
                replace(method, "120")
                put(environment, "regular-sentinel")
                put(environment, "9", True)
                NAV.key(method, "Home", "120", 0, 0)
                left = (rect[0] + 2, point[1])
                INPUT.move(left)
                previous = method.states[-1]["serial"]
                rapid("middle")
                report["middle_valid"] = NAV.changed(method, previous, "9120", 1, 1)
                clipboard(environment, "9", True)
                clipboard(environment, "regular-sentinel")
                put(environment, "not a number", True)
                NAV.key(method, "Home", "9120", 0, 0)
                before = NAV.context(method)
                rapid("middle")
                report["middle_invalid"] = NAV.unchanged(method, before)
                clipboard(environment, "not a number", True)

                # 5. 【字段主选择回归】【自身来源】拖选后在字段外释放，再粘贴本进程拥有的来源，验证不会等待自身发送回调
                replace(method, "12")
                INPUT.move(left)
                previous = method.states[-1]["serial"]
                NAV.button("left", True)
                try:
                    INPUT.move((point[0] + 60, point[1]))
                finally:
                    NAV.button("left", False)
                report["release_outside"] = selected(method, root, qt, previous, "12", 2, 0, "12")
                replace(method, "3")
                NAV.key(method, "Home", "3", 0, 0)
                INPUT.move(left)
                previous = method.states[-1]["serial"]
                rapid("middle")
                report["middle_own_source"] = NAV.changed(method, previous, "123", 2, 2)
                clipboard(environment, "12", True)
                put(environment, "regular-sentinel")
                clipboard(environment, "regular-sentinel")
                replace(method, "9120")
                time.sleep(0.45)
                NAV.key(method, "Home", "9120", 0, 0)

                # 6. 【字段主选择回归】【当前拥有者】用最后一次真实选择替换外部数据源，隐藏输入后再关闭进程验证生命周期
                NAV.click(method, point, "9120", 4, 4)
                previous = method.states[-1]["serial"]
                rapid()
                report["owned_at_exit"] = selected(method, root, qt, previous, "9120", 4, 0, "9120")
                session.key("Escape")
                time.sleep(0.6)
                first = session.resources(process.pid)
                submissions = log.read_text().count(".damage_buffer(") + log.read_text().count(".damage(")
                time.sleep(1)
                second = session.resources(process.pid)
                assert first["cpu_ticks"] == second["cpu_ticks"], (first, second)
                assert submissions == log.read_text().count(".damage_buffer(") + log.read_text().count(".damage(")
                if not qt:
                    assert second["threads"] == 1
                    assert not Path(f"/proc/{process.pid}/task/{process.pid}/children").read_text().strip()
                report["idle"] = {"before": first, "after": second, "buffer_submissions": submissions}
                if qt:
                    process.terminate()
                else:
                    session.key("Escape")
                assert process.wait(timeout=3) is not None
                empty(environment)
                report["primary_released_on_exit"] = True
            session.screenshot(root / "complete.png")
        report["states"] = method.states
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    """【字段主选择回归】【入口】解析本次 binary、root 与范围，失败保留日志，打印完成记录"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--qt", action="store_true")
    parser.add_argument("--motions-only", action="store_true")
    options = parser.parse_args()
    report = workflow(options.binary.resolve(), options.root.resolve(), options.qt, options.motions_only)
    print(json.dumps({"binary_sha256": report["binary_sha256"], "qt": options.qt,
                      "motion_interval_ms": report["motion"]["native_interval_ms"],
                      "result": str(options.root / "result.json")}, indent=2))


if __name__ == "__main__":
    main()
