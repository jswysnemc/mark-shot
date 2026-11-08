#!/usr/bin/env python3
"""【图片编辑回归】【普通窗口】在私有 Sway 调整编辑窗口并逐像素验证原图导出"""

import argparse
import hashlib
import importlib.util
import io
import json
import re
from pathlib import Path
import subprocess
import time

from PIL import Image, ImageChops
import wayland_text_format_session as session

SPEC = importlib.util.spec_from_file_location("editor_support", Path(__file__).with_name("wayland-memory-baseline.py"))
SUPPORT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SUPPORT)


def verify_resize_commits(text):
    """【图片编辑回归】【完整提交】按 text 核对首个绘制帧后每次缓冲附着都有帧回调，返回已校验提交数，拒绝尺寸变更时提交未绘制槽位"""
    surfaces = {}
    checked = 0
    for line in text.splitlines():
        match = re.search(r"-> wl_surface([@#]\d+)\.(attach|frame|commit|destroy)\(", line)
        if not match:
            continue
        surface, action = match.groups()
        if action == "destroy":
            surfaces.pop(surface, None)
            continue
        state = surfaces.setdefault(surface, dict(presented=False, attached=False, frame=False))
        if action == "attach" and "wl_buffer" in line:
            state["attached"] = True
        elif action == "frame":
            state["frame"] = True
        elif action == "commit":
            if state["attached"] and state["presented"]:
                assert state["frame"], f"image resize attached an unpainted buffer on {surface}"
                checked += 1
            state["presented"] |= state["frame"]
            state["attached"] = state["frame"] = False
    assert checked >= 3, "insufficient resized image frames"
    return checked


def run(binary, root, implementation, snapshot, settings_roundtrip=False):
    """【图片编辑回归】【真实流程】按 binary、root、implementation 和 snapshot 打开透明图，settings_roundtrip 验证设置返回；返回窗口和原图报告"""
    root.mkdir(parents=True, exist_ok=False)
    original = Image.new("RGBA", (600, 340))
    original.putdata([(40 + column % 170, 70 + row % 150, 140, 255)
                      if (column // 45 + row // 35) % 3 else (0, 0, 0, 0)
                      for row in range(340) for column in range(600)])
    image = root / "original.png"
    original.save(image)
    environment = session.environment()
    with session.input_context():
        session.sway.keyboard_ready()
        flags = ["--history-edit-snapshot"] if snapshot else [str(image)]
        with SUPPORT.launch(binary, implementation, root / "app", flags, image.read_bytes() if snapshot else None) as (process, log):
            def window():
                """【图片编辑回归】【窗口身份】只查询本次进程创建的桌面窗口，返回窗口树节点或空值"""
                assert process.poll() is None, log.read_text()[-2000:]
                return next((node for node in SUPPORT.SCROLL.tree(environment)
                             if node.get("pid") == process.pid and node.get("app_id")), None)

            shown = SUPPORT.SCROLL.wait(window, "image editor did not open a normal desktop window")
            assert not shown.get("fullscreen_mode"), "image editor forced fullscreen"
            if implementation == "rust":
                assert shown["app_id"] == "mark-shot-editor", shown
            if settings_roundtrip:
                time.sleep(0.35)
                session.key("ctrl+comma")
                SUPPORT.SCROLL.wait(lambda: any(node.get("pid") == process.pid and node.get("app_id") == "mark-shot-settings" for node in SUPPORT.SCROLL.tree(environment)), "settings did not open from the editor")
                time.sleep(0.25)
                SUPPORT.raw_capture().save(root / "settings-roundtrip.png")
                session.key("Escape")
                shown = SUPPORT.SCROLL.wait(lambda: next((node for node in SUPPORT.SCROLL.tree(environment) if node.get("pid") == process.pid and node.get("app_id") == "mark-shot-editor"), None), "settings did not restore the image editor")
            identifier = shown["id"]
            sizes = []
            for width, height in [(960, 620), (580, 400), (800, 520)]:
                SUPPORT.SCROLL.command(environment, f'[con_id={identifier}] floating enable, resize set width {width} px height {height} px, move position center, focus')
                time.sleep(0.35)
                shown = window()
                if implementation == "rust":
                    assert shown["rect"]["width"] == width and shown["rect"]["height"] == height, shown
                sizes.append(shown["rect"])
                SUPPORT.raw_capture().save(root / f"window-{width}.png")
            commits = verify_resize_commits(log.read_text()) if implementation == "rust" else None
            session.key("ctrl+c")
            process.wait(timeout=8)
            copied = subprocess.run(["wl-paste", "--no-newline", "--type", "image/png"], env=environment, capture_output=True, check=True, timeout=8).stdout
            (root / "copied.png").write_bytes(copied)
            with Image.open(io.BytesIO(copied)) as result:
                result = result.convert("RGBA")
                assert result.size == original.size
                assert ImageChops.difference(original, result).getbbox(alpha_only=False) is None, "window resize changed exported original pixels"
            subprocess.run(["wl-copy", "--clear"], env=environment, check=True, timeout=3)
    report = dict(binary=str(binary), sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                  implementation=implementation, snapshot=snapshot, settings_roundtrip=settings_roundtrip, sizes=sizes, painted_resize_commits=commits, original_pixels_equal=True, passed=True)
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    """【图片编辑回归】【命令入口】读取二进制、实现和私有结果目录，执行窗口验证并输出结论，返回状态码"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--implementation", choices=("rust", "qt"), default="rust")
    parser.add_argument("--snapshot", action="store_true")
    parser.add_argument("--settings-roundtrip", action="store_true")
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve(), options.implementation, options.snapshot, options.settings_roundtrip), indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
