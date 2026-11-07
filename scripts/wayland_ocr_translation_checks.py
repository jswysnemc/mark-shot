"""【OCR 翻译回归】【证据断言】读取真实任务记录、原生弹层及空闲资源，不从应用内部状态推断成功"""

import json
from pathlib import Path
import shlex
import time

from PIL import ImageChops
import wayland_text_format_session as session
from wayland_ocr_popup_tree import active_tree


def configure(root):
    """【OCR 翻译回归】【私有配置】在 root 创建仅含本地命令的 TOML，返回配置路径；不读取用户配置或凭据"""
    command_file = Path(__file__).with_name("wayland_ocr_translation_command.py")
    command = "exec /usr/bin/python " + shlex.quote(str(command_file)) + " --root " + shlex.quote(str(root)) + " --input {input} --target {targetLanguage}"
    config = root / "config/mark-shot/config.toml"
    config.parent.mkdir(parents=True)
    config.write_text("[history]\nenabled = false\n[notifications]\nenabled = false\n[translation]\nprovider = 'plugin:unavailable'\ntargetLanguage = 'English'\ntimeoutMs = 10000\ncommand = " + json.dumps(command) + "\n")
    (root / "mode").write_text("hold")
    return config


def requests(root, count=None):
    """【OCR 翻译回归】【实际请求】读取 root 完整任务记录，count 指定等待数量，返回有序记录，超量与超时均失败"""
    deadline = time.monotonic() + 3
    while True:
        paths = sorted((root / "requests").glob("*.json"))
        values = [json.loads(path.read_text()) for path in paths]
        if count is None or len(values) == count:
            return values
        if len(values) > count or time.monotonic() >= deadline:
            raise AssertionError(f"expected {count} real requests, found {len(values)}")
        time.sleep(0.02)


def reclaimed(record):
    """【OCR 翻译回归】【资源释放】等待 record 直接子进程消失、后代停止及输入删除，无返回值，期限为一秒"""
    deadline = time.monotonic() + 1
    while True:
        leader = Path(f"/proc/{record['pid']}").exists()
        input_live = Path(record["input"]).exists()
        child_live = False
        if record["child"] is not None:
            try:
                child_live = not Path(f"/proc/{record['child']}/stat").read_text().rsplit(") ", 1)[1].startswith("Z")
            except FileNotFoundError:
                pass
        if not leader and not input_live and not child_live:
            return
        if time.monotonic() >= deadline:
            raise AssertionError(f"translation retained resources: leader={leader}, child={child_live}, input={input_live}")
        time.sleep(0.015)


def idle(pid, log):
    """【OCR 翻译回归】【静止资源】按 pid、log 验证任务结束后单线程、CPU 时钟不增加且不重复提交表面，返回两次资源记录"""
    session.wait_single_thread(pid)
    time.sleep(0.12)
    before = session.resources(pid)
    frames = log.read_text().count("wl_surface")
    time.sleep(0.8)
    after = session.resources(pid)
    assert before["threads"] == after["threads"] == 1
    assert before["cpu_ticks"] == after["cpu_ticks"]
    assert frames == log.read_text().count("wl_surface"), "idle translation submitted periodic frames"
    return {"before": before, "after": after}


def popup(log, count):
    """【OCR 翻译回归】【弹层呈现】等待 log 中 count 个原生节点完成配置与提交，返回节点，关闭后不复用旧记录"""
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        nodes = active_tree(log.read_text())
        if len(nodes) == count and all(node["rect"] is not None and node["frames"] for node in nodes):
            return nodes
        time.sleep(0.02)
    raise AssertionError(f"expected {count} presented native popups, found {active_tree(log.read_text())}")


def point(window, action):
    """【OCR 翻译回归】【原版坐标】按 window 和 action 返回当前分栏内部命中点，坐标来自 Qt 断点和整数布局"""
    width, height = window["width"], window["height"]
    horizontal = width >= 620
    extent = round((width - 24 if horizontal else height - 105) * 0.6)
    pane_x, pane_y = (16 + extent, 71) if horizontal else (8, 79 + extent)
    positions = {"toggle": (width - 117, 19), "action": (width - 54, 52),
                 "language": (130 if width >= 480 else 25, 52), "arrow": (width - 125, 52),
                 "source": (60, 115), "translation": (pane_x + 45, pane_y + 50),
                 "copy": (width - 18, pane_y + 10)}
    x, y = positions[action]
    return window["x"] + x, window["y"] + y


def notice_rows(before, after, window):
    """【OCR 翻译回归】【提示换行】按 before、after 与 window 返回真实提示字形行数，排除正文和页脚，不把单行跨基线误认为两行"""
    left, top = round(point(window, "translation")[0] - 45), window["y"] + window["height"] - 60
    difference = ImageChops.difference(before, after).crop((left, top, window["x"] + window["width"] - 8, top + 34))
    lengths, length = [], 0
    for y in range(difference.height):
        if difference.crop((0, y, difference.width, y + 1)).getbbox() is not None:
            length += 1
        elif length:
            lengths.append(length)
            length = 0
    lengths.append(length)
    return sum(length >= 3 for length in lengths)
