"""【窗口输入法测试】【私有窗口】固定结果帧、窗口位置和真实呈现等待，不连接用户桌面"""

import json
from pathlib import Path
import re
import struct
import subprocess
import time

from PIL import Image, ImageChops
import wayland_text_format_session as session
from wayland_text_pointer import click


ROOT = Path(__file__).resolve().parents[1]
ROWS = [["Item", "Count", "Total"], ["Paper", "12", "36"], ["Ink", "2", "24"]]


def frame():
    """【窗口输入法测试】【结果帧】返回真实结果传输格式的固定原文、表格和 PNG，不调用模型，无参数"""
    lines = [{"text": text, "bounds": [38 + column * 220, 32 + row * 66, max(26, len(text) * 15), 24]} for row, values in enumerate(ROWS) for column, text in enumerate(values)]
    header = json.dumps({"text": "base", "lines": lines}).encode()
    png = (ROOT / "tests/fixtures/ocr-table.png").read_bytes()
    return b"MSOCR001" + struct.pack("<II", len(header), len(png)) + header + png


def find_node(value, pid):
    """【窗口输入法测试】【树定位】在 value 的私有 Sway 树中寻找 pid 对应窗口，返回节点或空值"""
    if value.get("pid") == pid and value.get("app_id") == "mark-shot-ocr-result":
        return value
    for child in value.get("nodes", []) + value.get("floating_nodes", []):
        found = find_node(child, pid)
        if found:
            return found
    return None


def geometry(pid, environment):
    """【窗口输入法测试】【窗口位置】在 environment 的隔离会话定位 pid 并设为 760×460，返回逻辑矩形"""
    if session.backend() is session.niri:
        window = session.niri.size_window(session.niri.wait_window(pid), 760, 460)
        left, top = window["floating_pos"]
        width, height = window["size"]
        return {"x": left, "y": top, "width": width, "height": height}
    binary = ROOT / "target/test-tools/sway/usr/bin/swaymsg"
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        tree = json.loads(subprocess.check_output([str(binary), "-r", "-t", "get_tree"], env=environment))
        node = find_node(tree, pid)
        if node:
            break
        time.sleep(0.04)
    else:
        raise AssertionError("the private OCR window was not mapped")
    subprocess.run([str(binary), f"[con_id={node['id']}] floating enable"], env=environment, check=True, stdout=subprocess.DEVNULL)
    time.sleep(0.15)
    subprocess.run([str(binary), f"[con_id={node['id']}] resize set width 760 px height 460 px, move position center, focus"], env=environment, check=True, stdout=subprocess.DEVNULL)
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        tree = json.loads(subprocess.check_output([str(binary), "-r", "-t", "get_tree"], env=environment))
        rect = find_node(tree, pid)["rect"]
        if (rect["width"], rect["height"]) == (760, 460):
            time.sleep(0.08)
            return rect
        time.sleep(0.04)
    raise AssertionError("private Sway did not apply the requested result window size")


def press(window, x, y):
    """【窗口输入法测试】【窗口点击】把 window 内的 x、y 转为私有全局位置并点击，无返回值"""
    click((window["x"] + x, window["y"] + y))
    time.sleep(0.04)


def screenshot(root, name):
    """【窗口输入法测试】【真实画面】把私有截图保存到 root 的 name，返回 RGB 像素"""
    path = root / (name + ".png")
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def cursor_rect(log):
    """【窗口输入法测试】【原生位置】从 log 的真实请求取得最近逻辑光标矩形，返回四元组，不从广告推断启用"""
    values = re.findall(r"set_cursor_rectangle\((-?\d+), (-?\d+), (\d+), (\d+)\)", log.read_text())
    if not values:
        raise AssertionError("the window did not send a native cursor rectangle")
    return tuple(map(int, values[-1]))


def candidate_pixels(before, after, window, old, new):
    """【窗口输入法测试】【候选字形】比较 before、after 中 old 与 new 光标之间的上部文字区域，返回差异像素数，排除光标和下划线"""
    left = round(window["x"] + old[0] + 2)
    right = round(window["x"] + new[0] - 2)
    top = round(window["y"] + new[1])
    bottom = top + round(new[3] * 0.65)
    if right <= left:
        raise AssertionError("preedit did not advance the real shaped cursor")
    difference = ImageChops.difference(before.crop((left, top, right, bottom)), after.crop((left, top, right, bottom)))
    return sum(pixel != (0, 0, 0) for pixel in difference.getdata())


def presented_candidate(root, name, before, window, old, log):
    """【窗口输入法测试】【候选等待】等待 root 的 name 画面出现相对 before 的真实候选字形，window、old、log 定位区域，返回截图和像素数"""
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        after = screenshot(root, name)
        pixels = candidate_pixels(before, after, window, old, cursor_rect(log))
        if pixels > 6:
            return after, pixels
        time.sleep(0.04)
    raise AssertionError("preedit activation did not produce real candidate glyphs")


def newer_state(method, previous, text, active=True):
    """【窗口输入法测试】【新状态】等待 method 在 previous 之后的真实状态匹配 text、active，返回状态；不复用相同文字的旧快照"""
    method.wait(lambda line: line.startswith("state\t") and method.states[-1]["serial"] > previous and method.states[-1]["active"] == active and (text is None or method.states[-1]["text"] == text))
    return method.states[-1]
