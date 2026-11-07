"""【Windows识别主题】【独立窗口】验证无键盘辅助的外部主题刷新与正文保持"""

import os
import re
import tempfile
import time

from PIL import ImageChops
from windows_ocr_actions import copy_all, rectangle, snapshot, xdo


def save_mode(config, mode):
    """【Windows识别主题】【外部保存】原子替换config内唯一主题为mode，无返回值，不改动其他配置"""
    text, count = re.subn(r'(?m)^(theme\s*=\s*)"(?:light|dark)"$',
                          lambda match: match[1] + '"' + mode + '"', config.read_text())
    assert count == 1, "theme configuration is missing or ambiguous"
    with tempfile.NamedTemporaryFile(dir=config.parent, delete=False) as file:
        file.write(text.encode())
        file.flush()
        os.fsync(file.fileno())
        temporary = file.name
    os.replace(temporary, config)


def check(probe, environment, root, identity, expected):
    """【Windows识别主题】【明暗往返】在identity验证实际标题栏明暗及恢复，正文必须仍为expected，返回像素证据"""
    bounds = rectangle(environment, identity)
    region = (bounds["x"], bounds["y"], bounds["x"] + bounds["width"], bounds["y"] + 38)
    xdo(environment, "mousemove", "--sync", 10, 10)
    # 1. 【Windows识别主题】【提示期限】等待复制反馈复位再比较标题栏，正文光标不进入裁切范围
    time.sleep(1.2)
    light = snapshot(environment, root, "result-theme-light").crop(region)
    reports = {}
    for mode in ("dark", "light"):
        save_mode(root / "config.toml", mode)
        deadline = time.monotonic() + 6
        while time.monotonic() < deadline:
            image = snapshot(environment, root, "result-theme-" + mode + "-updated").crop(region)
            difference = ImageChops.difference(light, image).getbbox()
            changed = sum(sum(a) - sum(b) > 200 for a, b in zip(light.getdata(), image.getdata()))
            if mode == "dark" and changed > light.width * light.height // 3:
                reports["dark_changed_pixels"] = changed
                break
            if mode == "light" and difference is None:
                reports["light_exact_restore"] = True
                break
            time.sleep(0.05)
        else:
            raise AssertionError(f"independent OCR window did not present {mode} from external configuration")
    # 2. 【Windows识别主题】【内容隔离】主题恢复后再次使用真实剪贴板确认完整正文，没有输入事件协助刷新
    copy_all(probe, environment, root, identity, "after-result-theme", expected)
    reports["text_preserved"] = True
    return reports
