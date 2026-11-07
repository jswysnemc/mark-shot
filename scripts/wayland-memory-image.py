#!/usr/bin/env python3
"""【内存回归】【大图质量】在私有 4K 输出核对完整长图每个像素，不使用产品拼接生成期望图"""

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import time

import numpy as np
from PIL import Image
import wayland_text_format_session as session
from wayland_text_pointer import pointer_context

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("memory_image_support", PROJECT / "scripts/wayland-memory-baseline.py")
SUPPORT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SUPPORT)


def exact_pixels(path, steps):
    """【内存回归】【独立原图】按 path 与 steps 核对原生页面纹理，返回真实尺寸和差异像素数；尺寸或内容不符即失败"""
    image = Image.open(path).convert("RGB")
    expected_size = (3200, 1800 + steps * 48)
    assert image.size == expected_size, (image.size, expected_size)
    actual = np.asarray(image)
    x = np.arange(80, 3280, dtype=np.uint32) * np.uint32(2_246_822_519)
    different = 0
    for row in range(image.height):
        # 1. 【内存回归】【期望条带】独立执行页面的整数纹理公式，不读取产品拼接器或其导出快照
        value = ((row + 80) * 2_654_435_761) & 0xFFFFFFFF
        value = ((value << 7) | (value >> 25)) & 0xFFFFFFFF
        hashed = np.uint32(value) ^ x
        expected = np.stack((hashed & 255, (hashed >> 8) & 255, (hashed >> 16) & 255), axis=1).astype(np.uint8)
        different += int(np.any(actual[row] != expected, axis=1).sum())
    assert different == 0, f"large scroll export differs at {different} pixels"
    return {"size": list(image.size), "different_pixels": different,
            "png_sha256": hashlib.sha256(path.read_bytes()).hexdigest()}


def run(binary, page, root, steps):
    """【内存回归】【完整交付】在私有输出使用 binary 和 page 执行 steps 次滚动，root 保存实际 PNG，返回质量结果与指纹"""
    root.mkdir(parents=True, exist_ok=False)
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    with session.input_context(), pointer_context():
        session.sway.keyboard_ready()
        with SUPPORT.SCROLL.page(page, session.environment(), root) as (_page, trace):
            with SUPPORT.launch(binary, "rust", root / "capture", ["-S", "--to", "o"]) as (process, log):
                SUPPORT.presented(process, log)
                SUPPORT.SCROLL.INPUT.drag((80, 80), (3280, 1880))
                time.sleep(0.8)
                SUPPORT.SCROLL.INPUT.move((400, 400))
                pointer = SUPPORT.SCROLL.INPUT.CURRENT.get()[0]
                for index in range(steps):
                    pointer.scroll(dy=10.0)
                    SUPPORT.SCROLL.wait(lambda: SUPPORT.SCROLL.origin(trace) == (0, (index + 1) * 48),
                                        "large export page lost the actual scroll input")
                    time.sleep(0.18)
                time.sleep(0.7)
                controls = SUPPORT.panel_controls(SUPPORT.raw_capture(), "rust")
                previous = len(log.read_text())
                SUPPORT.SCROLL.INPUT.click(controls["annotate"])
                SUPPORT.SCROLL.wait(lambda: SUPPORT.initial_frame_ready(log.read_text()[previous:], "rust"),
                                    "large export did not present a keyboard-active result foreground")
                time.sleep(0.3)
                session.key("Return")
                assert process.wait(timeout=10) == 0, log.read_text()[-2000:]
                result = exact_pixels(root / "capture/output.png", steps)
    assert fingerprint == hashlib.sha256(binary.read_bytes()).hexdigest(), "quality candidate changed"
    result.update(binary_sha256=fingerprint, steps=steps, all_pixels_equal=True)
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    """【内存回归】【命令行】解析固定 binary、page、新 root 和 steps，执行原图验证，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--page", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--steps", type=int, default=128)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.page.resolve(), options.root.resolve(), options.steps), indent=2))


if __name__ == "__main__":
    main()
