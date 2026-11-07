#!/usr/bin/env python3
"""【选区尺寸回归】【真实会话】验证字段、比例抓取、主题、物理 PNG 和空闲资源"""

import argparse
from contextlib import contextmanager
import hashlib
import importlib.util
import io
import json
import math
import os
import re
from pathlib import Path
import subprocess
import time

from PIL import Image, ImageChops
import wayland_text_format_session as session
from wayland_text_pointer import pointer_context
from wayland_ocr_popup_tree import active_tree, popup_tree
from wayland_input_method_transport import InputMethod, build, protocol_evidence

PROJECT = Path(__file__).resolve().parents[1]


def support():
    """【选区尺寸回归】【设备共用】返回既有已校验的指针、截图和启动支持模块，不连接用户桌面"""
    spec = importlib.util.spec_from_file_location("selection_input_support", PROJECT / "scripts/wayland-selection-translation.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


INPUT = support()
NATIVE_REFERENCE = json.loads((PROJECT / "tests/fixtures/qt-selection-size/native.json").read_text())["field"]


@contextmanager
def input_method(environment, root):
    """【选区尺寸回归】【真实输入法】在 environment 私有座席连接原生客户端，root 保存协议，退出关闭本次连接"""
    client = build(root, PROJECT / "target/test-tools/input-method/input-method-unstable-v2.xml")
    method = InputMethod(client, environment, root)
    try:
        yield method
    finally:
        method.close()


def updated(method, previous, text):
    """【选区尺寸回归】【新上下文】等待 method 收到序号晚于 previous 且正文为 text 的状态，返回新状态"""
    method.wait(lambda line: line.startswith("state\t") and method.states[-1]["serial"] > previous and method.states[-1]["active"] and method.states[-1]["text"] == text)
    return method.states[-1]


def cursor(log):
    """【选区尺寸回归】【真实光标】返回 log 最近实际提交的输入法逻辑光标矩形，缺失视为失败"""
    values = re.findall(r"set_cursor_rectangle\((-?\d+), (-?\d+), (\d+), (\d+)\)", log.read_text())
    if not values:
        raise AssertionError("selection field did not send its native caret rectangle")
    return tuple(map(int, values[-1]))


def native_field(method, environment, root, log):
    """【选区尺寸回归】【原生字段】用 method 核对候选字形、原子拒绝与普通文字用途，environment、root、log 保存证据"""
    session.key("ctrl+r")
    active = method.state("100")
    if (active["hint"], active["purpose"]) != (NATIVE_REFERENCE["hint"], NATIVE_REFERENCE["purpose"]):
        raise AssertionError(f"selection field content type differs from real Qt: {active}")
    session.key("Backspace")
    method.state("")
    time.sleep(0.08)
    before = snapshot(root, "ime-empty")
    old = cursor(log)
    previous = method.states[-1]["serial"]
    method.batch(preedit="hao")
    updated(method, previous, "")
    time.sleep(0.12)
    after = snapshot(root, "ime-candidate")
    new = cursor(log)
    if new[0] <= old[0]:
        raise AssertionError(f"selection preedit did not advance shaped caret: {old} -> {new}")
    area = (old[0] + 2, new[1], new[0] - 2, new[1] + max(1, round(new[3] * 0.65)))
    difference = ImageChops.difference(before.crop(area), after.crop(area))
    values = difference.tobytes()
    pixels = sum(any(values[index:index + 3]) for index in range(0, len(values), 3))
    if pixels <= 6:
        raise AssertionError(f"selection candidate has no visible glyph evidence: {pixels}")
    previous = method.states[-1]["serial"]
    method.batch(preedit="")
    updated(method, previous, "")
    previous = method.states[-1]["serial"]
    method.batch(commit="100")
    updated(method, previous, "100")
    session.key("ctrl+a")
    method.batch(commit="12x")
    time.sleep(0.12)
    subprocess.run(["wl-copy", "--", "invalid commit sentinel"], env=environment, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True, timeout=3)
    session.key("ctrl+c")
    session.wait_clipboard(environment, "100")
    session.key("Return")
    method.state(active=False)
    evidence = protocol_evidence(log.read_text())
    if not evidence["native_protocol"] or not evidence["preedit_delivered"]:
        raise AssertionError(f"selection field omitted actual native protocol delivery: {evidence}")
    return {"normal_content_type": True, "hint": active["hint"], "purpose": active["purpose"], "candidate_pixels": pixels, "invalid_commit_preserved_complete_text": True, **evidence}


def snapshot(root, name):
    """【选区尺寸回归】【实际截图】将 name 画面保存在 root，返回完整 RGB 像素"""
    path = root / f"{name}.png"
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def theme_snapshot(process, log, root, name, physical):
    """【选区尺寸回归】【主题提交】向process切换主题，按log等待physical尺寸的新完整缓冲及回调，在root保存name，返回RGB图像"""
    previous = len(log.read_text())
    session.key("ctrl+shift+l")
    deadline = time.monotonic() + 3.0
    # 1. 【选区尺寸回归】【完整缓冲】主题保存和绘制时间不固定，当前按键后的全画布提交才代表新界面
    pattern = re.compile(r"wl_surface@\d+\.damage_buffer\(0, 0, " + str(physical[0]) + r", " + str(physical[1]) + r"\)")
    while True:
        assert process.poll() is None, "editor exited while changing theme"
        current = log.read_text()[previous:]
        damaged = pattern.search(current)
        if damaged:
            frame = re.search(r"wl_surface@\d+\.frame\(wl_callback@(\d+)\)", current[damaged.end():])
            # 2. 【选区尺寸回归】【回调完成】等待本次缓冲的回调送达，避免仍比较前一个深色或浅色画面
            if frame and re.search(r"wl_callback@" + frame[1] + r"\.done,", current[damaged.end()+frame.end():]):
                return snapshot(root, name)
        assert time.monotonic() < deadline, "theme did not submit and complete its new full buffer"
        time.sleep(0.015)


def creation_geometry(log, output):
    """【选区尺寸回归】【实际起止】按 log 第一段真实拖动和 output 像素尺寸返回 Qt 归一化几何与显示宽高"""
    position = anchor = None
    for line in log.read_text().splitlines():
        move = re.search(r"wl_pointer@\d+\.motion, \(\d+, (-?[\d.]+), (-?[\d.]+)\)", line)
        enter = re.search(r"wl_pointer@\d+\.enter, \(\d+, wl_surface@\d+, (-?[\d.]+), (-?[\d.]+)\)", line)
        if move or enter:
            event = move or enter
            position = tuple(round(float(value) * 256) / 256 for value in event.groups())
        button = re.search(r"wl_pointer@\d+\.button, \(\d+, \d+, 272, ([01])\)", line)
        if button and button[1] == "1" and anchor is None:
            anchor = position
        elif button and button[1] == "0" and anchor is not None:
            break
    if anchor is None or position is None:
        raise AssertionError("actual selection pointer origin or release is missing")
    mode = output["modes"][output["current_mode"]]
    scale = mode["width"] / output["logical"]["width"]
    values = [math.floor(abs(position[axis] - anchor[axis]) * scale + 0.5) for axis in range(2)]
    return {"anchor": list(anchor), "pointer": list(position), "capture_size": [mode["width"], mode["height"]], "capture_scale": scale, "pixels": values}


def field(environment, expected, side="width"):
    """【选区尺寸回归】【字段读取】用真实焦点和剪贴板核对 side 的 expected，结束时取消输入，返回字符串"""
    session.key("ctrl+r")
    if side == "height":
        session.key("Tab")
    subprocess.run(["wl-copy", "--", "field sentinel"], env=environment, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True, timeout=3)
    session.key("ctrl+c")
    actual = session.wait_clipboard(environment, str(expected))
    session.key("Escape")
    return actual


def set_size(value, side="width"):
    """【选区尺寸回归】【字段提交】通过真实键盘把 value 写入 side，不读取内部选区状态，无返回值"""
    session.key("ctrl+r")
    if side == "height":
        session.key("Tab")
    session.paste_text(str(value))
    session.key("Return")
    time.sleep(0.12)


def popup(log, point):
    """【选区尺寸回归】【原生抓取】点击 point 并等待 log 中新的单层真实抓取，返回实际角色和位置"""
    INPUT.click(point)
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        try:
            nodes = active_tree(log.read_text())
        except AssertionError as error:
            if str(error) != "popup has no explicit native grab":
                raise
            time.sleep(0.03)
            continue
        if len(nodes) == 1 and nodes[0]["rect"] is not None:
            if nodes[0]["rect"][2:] != [128, 164]:
                raise AssertionError(f"unexpected ratio popup size: {nodes[0]}")
            return nodes[0]
        time.sleep(0.03)
    raise AssertionError("ratio popup did not acquire its native grab")


def row(node, index):
    """【选区尺寸回归】【实际行坐标】按 node 协议位置返回 index 行中心，不假定合成器未调整锚点"""
    x, y, _, _ = node["rect"]
    return x + 64, y + 4 + index * 26 + 13


def closed(log):
    """【选区尺寸回归】【抓取释放】确认 log 中所有实际角色都已关闭，无参数以外返回值"""
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        if not active_tree(log.read_text()):
            return
        time.sleep(0.03)
    raise AssertionError("ratio popup retained its native grab")


def png(environment, process, root, expected):
    """【选区尺寸回归】【实际导出】复制截图后等待 process 完成，核对 environment 物理 PNG 尺寸并保存到 root"""
    session.key("ctrl+c")
    process.wait(timeout=5)
    if process.returncode != 0:
        raise AssertionError(f"screenshot export exited with {process.returncode}")
    result = subprocess.run(["wl-paste", "--type", "image/png"], env=environment, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True, timeout=3)
    destination = root / "export.png"
    destination.write_bytes(result.stdout)
    with Image.open(io.BytesIO(result.stdout)) as image:
        if image.size != expected:
            raise AssertionError(f"physical PNG size differs: {image.size} != {expected}")
        rgba = image.convert("RGBA")
        if any(low != high for low, high in rgba.getextrema()):
            raise AssertionError("unannotated blank source export contains selection or interface pixels")
        return {"size": list(image.size), "rgba": list(rgba.getpixel((0, 0))), "sha256": hashlib.sha256(result.stdout).hexdigest()}


def idle(process, log):
    """【选区尺寸回归】【空闲资源】采集 process 八次真实资源与 log 缓冲提交，返回样本，拒绝周期增长"""
    session.wait_single_thread(process.pid)
    samples = []
    for _ in range(8):
        sample = session.resources(process.pid)
        sample["buffer_submissions"] = log.read_text().count(".attach(wl_buffer@")
        samples.append(sample)
        time.sleep(0.1)
    if any(sample["threads"] != 1 for sample in samples):
        raise AssertionError(f"idle editor has extra threads: {samples}")
    if len({sample["cpu_ticks"] for sample in samples}) != 1 or len({sample["buffer_submissions"] for sample in samples}) != 1:
        raise AssertionError(f"idle editor repeatedly woke or submitted frames: {samples}")
    return samples


def run(binary, root):
    """【选区尺寸回归】【完整操作】在 root 的私有会话用 binary 验证真实字段与尺寸，返回结果字典"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"), TMPDIR=str(root / "temporary"), MARK_SHOT_LANG="english", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/selection-size-test")
    Path(environment["TMPDIR"]).mkdir()
    config = root / "config.toml"
    config.write_text('[general]\noverlay="window"\nto=""\n[notifications]\nenabled=false\n[history]\nenabled=false\n[export]\nimageFrame=false\n')
    runtime = json.loads((Path(os.environ["XDG_RUNTIME_DIR"]) / "runtime.json").read_text())
    output = next(iter(runtime["outputs"].values()))
    scale = output["logical"]["scale"]
    report = {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "scale": scale, "compositor": session.backend().__name__}
    with session.input_context(), pointer_context(backend="x11" if session.backend() is session.niri else "virtual"), input_method(environment, root / "input-method") as method:
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        with INPUT.launch(binary, environment, config, None, root, "capture") as (process, log):
            INPUT.drag((0, 0), (300, 180))
            time.sleep(0.25)
            report["capture_order"] = INPUT.capture_before_overlay(log)
            report["creation"] = creation_geometry(log, output)
            initial_width, initial_height = report["creation"]["pixels"]
            report["initial"] = [field(environment, initial_width), field(environment, initial_height, "height")]
            snapshot(root, "initial")
            width_point = (6 + 6 + 47 / 2, 6 + 13)
            ratio_point = (6 + 165 + 93 / 2, 6 + 13)
            # 1. 【选区尺寸回归】【键盘延迟】上下键只修改待提交文字，取消后仍显示最初实际尺寸
            INPUT.click(width_point, "right")
            session.key("ctrl+r")
            session.paste_text("100")
            session.key("Up")
            snapshot(root, "arrow-pending")
            session.key("Escape")
            field(environment, initial_width)
            set_size(100)
            set_size(80, "height")
            report["exact_fields"] = [field(environment, 100), field(environment, 80, "height")]
            session.key("ctrl+r")
            session.key("Tab")
            session.paste_text("80")
            session.key("ctrl+r")
            session.key("ctrl+c")
            session.wait_clipboard(environment, "100")
            session.key("Escape")
            report["ctrl_r_from_height"] = True
            snapshot(root, "exact-size")
            # 2. 【选区尺寸回归】【尺寸拒绝】小于八像素的输入拒绝应用，真实字段仍返回一百像素
            set_size(7)
            field(environment, 100)
            snapshot(root, "minimum-rejected")
            time.sleep(1.8)
            # 3. 【选区尺寸回归】【比例抓取】弹层打开丢弃字段草稿，右键不选择，一比一选择立即联动实际高度
            session.key("ctrl+r")
            session.paste_text("88")
            node = popup(log, ratio_point)
            snapshot(root, "ratio-popup")
            INPUT.click(row(node, 1), "right")
            if len(active_tree(log.read_text())) != 1:
                raise AssertionError("right button dismissed or chose the ratio")
            INPUT.click(row(node, 1))
            closed(log)
            report["square_fields"] = [field(environment, 100), field(environment, 100, "height")]
            # 4. 【选区尺寸回归】【输入独占】Escape 只关闭比例列表，截图进程继续存在
            popup(log, ratio_point)
            session.key("Escape")
            closed(log)
            if process.poll() is not None:
                raise AssertionError("ratio Escape closed the screenshot editor")
            node = popup(log, ratio_point)
            INPUT.click(row(node, 0))
            closed(log)
            set_size(80, "height")
            report["input_method"] = native_field(method, environment, root, log)
            INPUT.move((500, 400))
            time.sleep(0.2)
            light = snapshot(root, "light")
            mode = output["modes"][output["current_mode"]]
            physical = (mode["width"], mode["height"])
            dark = theme_snapshot(process, log, root, "dark", physical)
            if ImageChops.difference(light, dark).getbbox() is None:
                raise AssertionError("theme shortcut did not change the visible panel")
            restored = theme_snapshot(process, log, root, "light-restored", physical)
            if light.tobytes() != restored.tobytes():
                raise AssertionError("returning to the light theme did not restore complete pixels")
            report["popup_records"] = popup_tree(log.read_text(), closed=True)
            report["idle"] = idle(process, log)
            report["export"] = png(environment, process, root, (100, 80))
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    """【选区尺寸回归】【执行入口】解析 binary 和新 root，打印完成结果，异常保留协议与截图证据"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=PROJECT / "target/debug/mark-shot")
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
