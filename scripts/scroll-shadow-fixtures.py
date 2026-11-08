#!/usr/bin/env python3
"""【滚动截图回归】【浏览器采样】生成脱敏的 Chromium 连续帧和无浮层参考图"""

import argparse
from pathlib import Path
import subprocess
import uuid

PROJECT = Path(__file__).resolve().parents[1]
CASES = ("light", "dark", "translucent", "both")
ORIGINS = (0, 96, 192, 288)


def generate(root, chromium):
    """【滚动截图回归】【连续采集】按 root 保存截图，用 chromium 启动独立会话，结束后关闭，无返回值"""
    session = "shadow-" + uuid.uuid4().hex[:12]
    prefix = ["agent-browser", "--session", session, "--executable-path", str(chromium)]

    def browser(*arguments):
        """【滚动截图回归】【浏览器命令】按 arguments 执行本次独立会话动作，返回标准输出，失败直接报告"""
        result = subprocess.run([*prefix, *map(str, arguments)], text=True, capture_output=True, timeout=30)
        if result.returncode:
            raise RuntimeError(result.stdout + result.stderr)
        return result.stdout

    page = PROJECT / "tests/fixtures/scroll-shadow/page.html"
    try:
        browser("open", page.as_uri())
        browser("set", "viewport", 640, 480)
        for case in CASES:
            destination = root / case
            destination.mkdir(parents=True, exist_ok=True)
            browser("open", page.as_uri() + "?case=" + case)
            browser("eval", "document.fonts.ready")
            for origin in ORIGINS:
                browser("eval", f"window.scrollTo(0, {origin})")
                browser("wait", "--fn", f"window.scrollY === {origin}")
                browser("screenshot", destination / f"frame-{origin}.png")
            browser("eval", "document.body.classList.add('clean'); window.scrollTo(0, 0)")
            browser("screenshot", destination / "reference.png", "--full")
            print(f"Captured {case}", flush=True)
    finally:
        browser("close")


def native_frames(source, destination):
    """【滚动截图回归】【原生样本】从 source 私有 1.2 倍输出裁出已知选区到 destination，返回无值"""
    from PIL import Image
    destination.mkdir(parents=True, exist_ok=True)
    for index, name in enumerate(("source-first.png", "source-0.png", "source-1.png")):
        with Image.open(source / name) as image:
            image.convert("RGB").crop((48, 48, 384, 384)).save(destination / f"frame-{index}.png")


def main():
    """【滚动截图回归】【采样入口】无参数，读取目录与浏览器选项后生成样本，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=PROJECT / "tests/fixtures/scroll-shadow")
    parser.add_argument("--chromium", type=Path, default=Path("/usr/bin/chromium"))
    parser.add_argument("--native-source", type=Path)
    options = parser.parse_args()
    if options.native_source:
        native_frames(options.native_source.resolve(), options.root.resolve() / "fractional")
    else:
        generate(options.root.resolve(), options.chromium.resolve())


if __name__ == "__main__":
    main()
