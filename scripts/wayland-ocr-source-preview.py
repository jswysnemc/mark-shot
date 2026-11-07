#!/usr/bin/env python3
"""【原图回归】【真实会话】验证缩放平移、原生菜单、原图编辑与贴图交接，不调用识别模型"""

import argparse
import hashlib
import io
import json
import os
import re
import select
import subprocess
import time
from pathlib import Path

import wayland_text_format_session as session
from PIL import Image
from wayland_ocr_preview_checks import equal_pixels, keyboard_release_grab
from wayland_ocr_source_menu_checks import popup_configurations
from wayland_text_pointer import CURRENT, click, pointer_context
from wayland_virtual_pointer import transform_for_pointer
from wayland_window_input_session import ROWS, frame, geometry
from wayland_ocr_window_layout import source_point, tab_point, table_point

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = json.loads((ROOT / "tests/fixtures/qt-ocr-source-preview.json").read_text())


def move(point):
    """【原图回归】【指针移动】把 point 转为已校验私有输出坐标，保留同一指针设备，无返回值"""
    pointer, logical, _, _ = CURRENT.get()
    pointer.move_abs(*transform_for_pointer(point[0] - logical["x"], point[1] - logical["y"],
        logical["width"], logical["height"], logical.get("pointer_transform", logical["transform"])))
    time.sleep(0.08)


def picture(path, window):
    """【原图回归】【可见原图】保存 path 的真实截图并返回 window 原图视区，排除标题和反馈文字"""
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGBA").crop((round(window["x"] + 8), round(window["y"] + 37),
            round(window["x"] + window["width"] - 8), round(window["y"] + window["height"] - 26)))


def windows(environment):
    """【原图回归】【私有窗口】返回 environment 对应合成器中的实际窗口，不读取其他桌面"""
    if session.backend() is session.niri:
        return session.niri.windows()
    binary = ROOT / "target/test-tools/sway/usr/bin/swaymsg"
    tree = json.loads(subprocess.check_output([str(binary), "-r", "-t", "get_tree"], env=environment))
    result = []
    pending = [tree]
    while pending:
        node = pending.pop()
        pending.extend(node.get("nodes", []) + node.get("floating_nodes", []))
        if node.get("pid") and node.get("app_id"):
            result.append(node)
    return result


def control(environment, item, action):
    """【原图回归】【窗口动作】对 environment 中已定位的 item 执行 action，限定本次窗口，无返回值"""
    if session.backend() is session.niri:
        session.niri.control(action, item["id"])
    else:
        command = "focus" if action == "focus" else "kill"
        subprocess.run([str(ROOT / "target/test-tools/sway/usr/bin/swaymsg"), f"[con_id={item['id']}] {command}"],
            env=environment, check=True, stdout=subprocess.DEVNULL)
    time.sleep(0.15)


def layer_child(environment, binary, log, argument="--pin", namespace="mark-shot-pin"):
    """【原图回归】【真实浮层】按 argument、namespace 从默认 layer-shell 交接中定位本次进程和协议矩形，返回身份，不强制改为普通窗口"""
    deadline = time.monotonic() + 12
    while time.monotonic() < deadline:
        result = subprocess.run(["pgrep", "-x", "mark-shot"], capture_output=True, text=True)
        matching = []
        for value in result.stdout.split():
            process = Path("/proc") / value
            try:
                settings = dict(item.split(b"=", 1) for item in (process / "environ").read_bytes().split(b"\0") if b"=" in item)
                if (process / "exe").resolve() == binary and argument.encode() in (process / "cmdline").read_bytes().split(b"\0") and settings.get(b"XDG_RUNTIME_DIR") == environment["XDG_RUNTIME_DIR"].encode() and settings.get(b"XDG_CONFIG_HOME") == environment["XDG_CONFIG_HOME"].encode():
                    matching.append(int(value))
            except (FileNotFoundError, PermissionError, ProcessLookupError):
                continue
        text = log.read_text() if log.exists() else ""
        roles = re.findall(r'get_layer_surface\(zwlr_layer_surface_v1@(\d+), wl_surface@(\d+),[^\n]*Some\("' + re.escape(namespace) + r'"\)', text)
        if len(matching) == 1 and roles:
            layer, surface = roles[-1]
            configured = re.search(rf"zwlr_layer_surface_v1@{layer}\.ack_configure\(\d+\)", text)
            presented = re.search(rf"wl_surface@{surface}\.attach\(wl_buffer@\d+,", text)
            sizes = re.findall(rf"zwlr_layer_surface_v1@{layer}\.set_size\((\d+), (\d+)\)", text)
            positions = re.findall(rf"zwlr_layer_surface_v1@{layer}\.set_margin\((-?\d+), -?\d+, -?\d+, (-?\d+)\)", text)
            if configured and presented and (namespace != "mark-shot-pin" or (sizes and positions)):
                os.kill(matching[0], 0)
                point = None
                if sizes and positions:
                    width, height = map(int, sizes[-1])
                    top, left = map(int, positions[-1])
                    point = (left + width / 2, top + height / 2)
                return {"pid": matching[0], "point": point}
        if len(matching) > 1:
            raise AssertionError("ambiguous owned layer-shell pin")
        time.sleep(0.05)
    raise AssertionError(f"default layer-shell image action did not map an owned child: {argument}")


def original_clipboard(environment, path):
    """【原图回归】【原始像素】从 environment 读取持久 PNG 到 path，与固定原图逐像素比较，无返回值"""
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        result = subprocess.run(["wl-paste", "--no-newline", "--type", "image/png"], env=environment, capture_output=True)
        if result.returncode == 0:
            path.write_bytes(result.stdout)
            with Image.open(io.BytesIO(result.stdout)) as actual, Image.open(ROOT / "tests/fixtures/ocr-table.png") as expected:
                assert equal_pixels(actual, expected), "image action copied a scaled, panned or edited-text snapshot"
            return
        time.sleep(0.05)
    raise AssertionError("image action did not deliver a persistent original PNG")


def popup(window, log):
    """【原图回归】【原生菜单】在 window 原图上右键，等待 log 的新角色配置，返回真实菜单矩形"""
    previous = len(popup_configurations(log.read_text()))
    click((window["x"] + 130, window["y"] + 190), "right")
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        values = popup_configurations(log.read_text())
        if len(values) > previous:
            assert list(values[-1][2:]) == FIXTURE["menu"]["size"], "image menu geometry differs from actual Qt"
            time.sleep(0.1)
            return values[-1]
        time.sleep(0.03)
    raise AssertionError("right-click did not open a native source image menu")


def choose(window, log, label):
    """【原图回归】【动作时机】点击 window 菜单的真实 label 行，检查 log 销毁先于交接，无返回值"""
    rect = popup(window, log)
    row = next(row for row in FIXTURE["menu"]["rows"] if row["label"] == label)
    x, y, width, height = row["rect"]
    previous = len(log.read_text())
    click((window["x"] + rect[0] + x + width / 2, window["y"] + rect[1] + y + height / 2))
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        text = log.read_text()[previous:]
        if "【文字识别】【原图交接】开始交接" in text:
            assert text.index(".destroy()") < text.index("【文字识别】【原图交接】开始交接"), "image handover started while popup grab was alive"
            return
        time.sleep(0.03)
    raise AssertionError("native image menu did not deliver its action")


def run(binary, root):
    """【原图回归】【完整验证】在私有会话执行 binary，root 保存实际截图、协议和同指纹结果，失败保留证据并回收本次进程"""
    environment = session.environment()
    root.mkdir(parents=True, exist_ok=False)
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    config = root / "config/mark-shot/config.toml"
    config.parent.mkdir(parents=True)
    config.write_text("[history]\nenabled = false\n[notifications]\nenabled = false\n")
    log = root / "process.log"
    owned = []
    with session.input_context(), pointer_context(), log.open("w") as output:
        process = subprocess.Popen([str(binary), "--ocr-result", "--nonotify"], env=dict(environment, WAYLAND_DEBUG="client"),
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=output)
        try:
            process.stdin.write(frame())
            process.stdin.close()
            assert select.select([process.stdout], [], [], 5)[0] and process.stdout.readline() == b"\n"
            window = geometry(process.pid, environment)
            # 1. 【原图回归】【复制绑定】原图页标题复制仍取得原文，不改为复制图像
            click(tab_point(window, "Image"))
            subprocess.run(["wl-copy", "--", "image sentinel"], env=environment, check=True, timeout=3)
            click(source_point(window, "Image"))
            session.wait_clipboard(environment, "base")
            # 2. 【原图回归】【独立表格入口】右键只聚焦，Space 释放打开真实原生格式菜单，标题复制仍取得原文
            click(tab_point(window, "Table"))
            previous = len(popup_configurations(log.read_text()))
            click(table_point(window), "right")
            assert len(popup_configurations(log.read_text())) == previous
            protocol_start = len(log.read_text())
            session.key("Space")
            deadline = time.monotonic() + 3
            while len(popup_configurations(log.read_text())) == previous and time.monotonic() < deadline:
                time.sleep(0.03)
            assert len(popup_configurations(log.read_text())) > previous, "table button Space release did not create a native menu"
            table_grab = keyboard_release_grab(log.read_text()[protocol_start:])
            session.key("Home")
            session.key("Return")
            session.wait_clipboard(environment, "\n".join("\t".join(row) for row in ROWS))
            click(source_point(window, "Image"))
            session.wait_clipboard(environment, "base")
            click(tab_point(window, "Image"))
            baseline = picture(root / "fit.png", window)
            # 3. 【原图回归】【原图手势】真实滚轮改变呈现，左键平移改变原图，双击恢复完全相同视区
            point = (window["x"] + 300, window["y"] + 240)
            move(point)
            pointer = CURRENT.get()[0]
            for _ in range(6):
                pointer.scroll(dy=-15)
            time.sleep(0.2)
            zoomed = picture(root / "zoomed.png", window)
            assert not equal_pixels(baseline, zoomed), "source image wheel did not zoom"
            pointer.press()
            move((point[0] + 120, point[1] + 40))
            pointer.release()
            time.sleep(0.12)
            panned = picture(root / "panned.png", window)
            assert not equal_pixels(zoomed, panned), "source image left drag did not pan"
            click(point)
            time.sleep(0.1)
            click(point)
            reset = picture(root / "reset.png", window)
            assert equal_pixels(baseline, reset), "double-click did not restore the original fitted view"
            # 4. 【原图回归】【菜单隔离】右键和外部点击不平移，菜单快捷键不能写入原文或覆盖剪贴板
            popup(window, log)
            session.screenshot(root / "image-menu.png")
            session.key("ctrl+c")
            assert session.read_clipboard(environment) == "base"
            click((window["x"] + 430, window["y"] + 250))
            time.sleep(0.12)
            outside = picture(root / "outside-canceled.png", window)
            assert equal_pixels(reset, outside), "outside popup dismissal was forwarded to image pan"
            # 5. 【原图回归】【独立交接】贴图和重编辑都交接原始 PNG，不交接预览缩放或正文
            choose(window, log, "Pin image")
            pinned = layer_child(environment, binary, root / "state/mark-shot/mark-shot.log")
            owned.append(pinned)
            child_log = root / "state/mark-shot/mark-shot.log"
            requests_before = len(child_log.read_text())
            click(pinned["point"])
            time.sleep(0.15)
            assert "mark-shot-pin-preview" not in child_log.read_text()[requests_before:], "plain pin click created an unnecessary input-disrupting preview"
            session.key("ctrl+c")
            original_clipboard(environment, root / "pin-original.png")
            session.key("Escape")
            time.sleep(0.15)
            parent = next(item for item in windows(environment) if item["pid"] == process.pid)
            control(environment, parent, "focus")
            choose(window, log, "Edit")
            editor = layer_child(environment, binary, child_log, "--history-edit-snapshot", "mark-shot")
            owned.append(editor)
            session.wait_single_thread(editor["pid"], timeout=15)
            session.key("ctrl+c")
            original_clipboard(environment, root / "edit-original.png")
            control(environment, parent, "focus")
            # 6. 【原图回归】【静止资源】已放大页面没有周期帧和常驻线程，关闭父窗口后原图剪贴板仍存在
            move(point)
            pointer.scroll(dy=-15)
            time.sleep(0.2)
            before = session.resources(process.pid)
            requests = log.read_text().count("wl_surface")
            time.sleep(1.1)
            after = session.resources(process.pid)
            assert before["threads"] == after["threads"] == 1 and before["cpu_ticks"] == after["cpu_ticks"]
            assert log.read_text().count("wl_surface") == requests
            session.key("Escape")
            assert process.wait(timeout=3) == 0
            original_clipboard(environment, root / "persistent-original.png")
            assert not (root / "data/mark-shot/models").exists()
            proof = {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "bytes": binary.stat().st_size,
                "compositor": "sway" if session.backend() is session.sway else "niri", "qt_menu_size": FIXTURE["menu"]["size"],
                "title_copy_kept_source": True, "wheel_zoom_presented": True, "left_pan_presented": True, "double_click_reset_exact": True,
                "native_popup": True, "right_and_outside_not_forwarded": True, "popup_destroyed_before_handover": True,
                "pin_original_pixels": True, "edit_original_pixels": True, "persistent_original_clipboard": True,
                "default_layer_shell_pin": True,
                "plain_pin_click_kept_keyboard_focus": True,
                "table_space_release_native_menu": True,
                "table_space_grab": table_grab,
                "idle_before": before, "idle_after": after, "no_idle_frames": True, "no_ocr": True}
            (root / "result.json").write_text(json.dumps(proof, indent=2) + "\n")
        finally:
            for item in owned:
                process_path = Path("/proc") / str(item["pid"])
                try:
                    settings = dict(value.split(b"=", 1) for value in (process_path / "environ").read_bytes().split(b"\0") if b"=" in value)
                    if (process_path / "exe").resolve() == binary and settings.get(b"XDG_RUNTIME_DIR") == environment["XDG_RUNTIME_DIR"].encode():
                        os.kill(item["pid"], 15)
                except (FileNotFoundError, ProcessLookupError):
                    pass
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)


def main():
    """【原图回归】【命令入口】读取目标二进制和独立证据目录，执行验证，无参数，失败返回非零状态"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    args = parser.parse_args()
    run(args.binary.resolve(), args.root.resolve())


if __name__ == "__main__":
    main()
