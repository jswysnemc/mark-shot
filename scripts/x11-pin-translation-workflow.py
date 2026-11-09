#!/usr/bin/env python3
"""【贴图翻译回归】【原生菜单】覆盖真实HTTP翻译、可见失败、重试、原图保留与首次资源引导"""

import argparse
import importlib.util
import io
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import time
from PIL import Image, ImageChops, ImageDraw, ImageGrab


def load(filename):
    """【贴图翻译回归】【模块读取】按filename读取相邻回归模块，返回模块对象"""
    spec = importlib.util.spec_from_file_location(filename, Path(__file__).with_name(filename))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def wait(predicate, message, timeout=5):
    """【贴图翻译回归】【有界等待】按predicate等待实际结果，超过timeout报告message，返回匹配结果"""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        value = predicate()
        if value:
            return value
        time.sleep(0.04)
    raise AssertionError(message)


def verify(runtime, service):
    """【贴图翻译回归】【完整流程】按runtime和service驱动真实菜单与HTTP，返回检查记录"""
    runtime.environment["MARK_SHOT_LANG"] = "chinese"
    config = runtime.root / "test.toml"
    config.write_text('[notifications]\nenabled=false\n[translation]\nprovider="builtin"\ntarget_language="English"\napiKey="fixture-key"\napiBase=' + json.dumps(service.endpoint) + '\n')
    image = Image.new("RGB", (420, 240), "white")
    ImageDraw.Draw(image).text((35, 35), "Original sample text", fill="black", font_size=21)
    encoded = io.BytesIO()
    image.save(encoded, "PNG")
    metadata = json.dumps({"source_tokens": [{"text": "Original sample text", "box": [35, 35, 310, 30], "line": 0, "index": 0, "confidence": 1.0}], "blocks": [], "show_translation": False}).encode()
    payload = b"MS-PIN-TRANSLATION\0" + struct.pack("<I", len(metadata)) + metadata + encoded.getvalue()
    pin = runtime.launch([str(runtime.binary), "--pin", "--translation-seed", "--position", "100,120"], "pin", stdin=subprocess.PIPE)
    pin.stdin.write(payload)
    pin.stdin.close()
    window = runtime.window(pin)
    runtime.command("xdotool", "windowfocus", window)

    def frame(name=None):
        """【贴图翻译回归】【实际画面】按可选name采集固定贴图，返回像素，不读取内部状态"""
        result = ImageGrab.grab(xdisplay=runtime.environment["DISPLAY"]).convert("RGB").crop((100, 120, 520, 360))
        if name:
            result.save(runtime.root / f"{name}.png")
        return result

    def menu(index):
        """【贴图翻译回归】【菜单输入】按可用项index通过真实右键、导航与确认交付动作，无返回值"""
        runtime.command("xdotool", "mousemove", "180", "200", "click", "3")
        time.sleep(0.08)
        runtime.command("xdotool", "key", "Home", *(["Down"] * index), "Return")
        time.sleep(0.1)

    time.sleep(0.15)
    original = frame("original")
    body = (0, 0, 420, 160)
    notice = (0, 175, 420, 240)
    menu(9)
    wait(lambda: len(service.requests) == 1, "right-click Translate did not reach the native provider")
    wait(lambda: ImageChops.difference(frame().crop(notice), original.crop(notice)).getbbox(), "HTTP failure was invisible with desktop notifications disabled")
    frame("http-failure")
    assert not ImageChops.difference(frame().crop(body), original.crop(body)).getbbox(), "failed request changed original text"
    service.mode = "empty"
    menu(9)
    wait(lambda: len(service.requests) == 2, "failed translation could not be retried")
    time.sleep(0.2)
    frame("empty-response")
    assert not ImageChops.difference(frame().crop(body), original.crop(body)).getbbox(), "empty translation replaced source text"
    service.mode = "delay"
    menu(9)
    wait(lambda: len(service.requests) == 3, "empty translation was cached as a successful result")
    progress = frame("translating")
    assert ImageChops.difference(progress.crop(notice), original.crop(notice)).getbbox(), "translation has no progress indication"
    service.release.set()
    wait(lambda: ImageChops.difference(frame().crop(body), original.crop(body)).getbbox(), "translated text was not rendered onto the pinned preview")
    frame("translated")
    assert service.requests[-1]["target_language"] == "English"
    assert service.requests[-1]["segments"][0]["text"] == "Original sample text"
    runtime.command("xdotool", "windowfocus", window, "key", "ctrl+c")
    copied = Image.open(io.BytesIO(runtime.command("xclip", "-selection", "clipboard", "-target", "image/png", "-out"))).convert("RGB")
    assert not ImageChops.difference(image, copied).getbbox(), "translation or progress was baked into copied original"
    menu(11)
    wait(lambda: not ImageChops.difference(frame().crop(body), original.crop(body)).getbbox(), "original text display did not restore")
    menu(11)
    wait(lambda: ImageChops.difference(frame().crop(body), original.crop(body)).getbbox(), "translated text toggle did not reuse cache")
    assert len(service.requests) == 3
    wait(lambda: not ImageChops.difference(frame().crop(notice), original.crop(notice)).getbbox(), "completion notice did not expire", timeout=5)
    runtime.command("xdotool", "windowfocus", window, "key", "Escape")
    pin.wait(timeout=4)
    image_path = runtime.root / "cold-image.png"
    image.save(image_path)
    cold = runtime.launch([str(runtime.binary), "--pin-image", str(image_path)], "cold-pin")
    cold_window = runtime.window(cold)
    runtime.command("xdotool", "windowfocus", cold_window, "mousemove", "--window", cold_window, "50", "50", "click", "3")
    time.sleep(0.1)
    runtime.command("xdotool", "key", "Home", *(["Down"] * 9), "Return")

    def guide():
        """【贴图翻译回归】【引导定位】无参数，返回私有进程真实可见的OCR资源窗口"""
        result = subprocess.run(["xdotool", "search", "--onlyvisible", "--name", "OCR$"], env=runtime.environment, capture_output=True, timeout=2)
        return result.stdout.decode().splitlines()

    dialog = wait(guide, "missing OCR resources did not open the guide")[0]
    runtime.command("xdotool", "windowfocus", dialog, "key", "Escape")
    wait(lambda: not guide(), "cancel did not close the OCR guide")
    assert cold.poll() is None and len(service.requests) == 3, "canceled OCR guide closed the pin or sent text"
    runtime.command("xdotool", "windowfocus", cold_window, "mousemove", "--window", cold_window, "50", "50", "click", "3")
    time.sleep(0.1)
    runtime.command("xdotool", "key", "Home", *(["Down"] * 9), "Return")
    wait(guide, "cancel left Translate permanently disabled")
    return ["right-click native HTTP request", "visible HTTP failure", "empty result rejection", "retry after failure", "visible translation progress", "translated image preview", "language and original request text", "original PNG preservation", "original/translation toggle and cache", "completion notice expiration", "OCR resource guide", "cancel preserves image and allows retry"]


def main():
    """【贴图翻译回归】【独立入口】无参数，创建私有显示与HTTP服务，验证后回收所属资源"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/mark-shot"))
    options = parser.parse_args()
    root = (options.root or Path(tempfile.mkdtemp(prefix="mark-shot-pin-translation-"))).resolve()
    root.mkdir(parents=True, exist_ok=True, mode=0o700)
    runtime = load("x11-desktop-workflow.py").Runtime(root, options.binary.resolve())
    service = load("pin-translation-test-server.py").Service()
    try:
        runtime.start()
        result = {"checks": verify(runtime, service)}
        (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result))
    finally:
        runtime.close()
        service.close()


if __name__ == "__main__":
    main()
