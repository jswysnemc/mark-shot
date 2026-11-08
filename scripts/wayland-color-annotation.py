#!/usr/bin/env python3
"""【取色标注回归】【原生导出】在私有桌面创建标注并验证最终 PNG 含有标签，原图分辨率保持不变"""

import argparse
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import time

from PIL import Image, ImageChops
import wayland_text_format_session as session
from wayland_text_pointer import pointer_context

SPEC = importlib.util.spec_from_file_location("support", Path(__file__).with_name("wayland-memory-baseline.py"))
SUPPORT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SUPPORT)


def run(binary, root):
    """【取色标注回归】【场景执行】按 binary 和 root 创建纯色图片，在原生窗口取样并拖动标签，返回导出验证结果"""
    root.mkdir(parents=True, exist_ok=False)
    original = Image.new("RGBA", (600, 340), (50, 100, 150, 255))
    source = root / "original.png"
    original.save(source)
    with session.input_context(), pointer_context():
        session.sway.keyboard_ready()
        with SUPPORT.launch(binary, "rust", root / "app", [str(source)]) as (process, log):
            shown = SUPPORT.SCROLL.wait(lambda: next((node for node in SUPPORT.SCROLL.tree(session.environment()) if node.get("pid") == process.pid and node.get("app_id") == "mark-shot-editor"), None), "image editor did not open")
            SUPPORT.SCROLL.command(session.environment(), f'[con_id={shown["id"]}] focus, fullscreen enable')
            time.sleep(0.4)
            session.key("i")
            SUPPORT.SCROLL.INPUT.drag((600, 350), (780, 470))
            time.sleep(0.2)
            SUPPORT.raw_capture().save(root / "annotation.png")
            session.key("v")
            session.key("ctrl+c")
            assert process.wait(timeout=8) == 0, log.read_text()[-2000:]
            png = subprocess.check_output(["wl-paste", "--type", "image/png"], env=session.environment(), timeout=8)
            (root / "copied.png").write_bytes(png)
            result = Image.open(io.BytesIO(png)).convert("RGBA")
            assert result.size == original.size
            difference = ImageChops.difference(result, original)
            changed = sum(any(pixel) for pixel in difference.get_flattened_data())
            assert changed > 1000, "export did not include the color label"
            subprocess.run(["wl-copy", "--clear"], env=session.environment(), check=True, timeout=3)
    report = dict(binary=str(binary), sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), changed_pixels=changed, original_size_preserved=True, passed=True)
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


def main():
    """【取色标注回归】【命令入口】解析二进制和私有输出目录并执行场景，无参数"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    run(options.binary.resolve(), options.root.resolve())


if __name__ == "__main__":
    main()
