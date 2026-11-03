#!/usr/bin/env python3
"""【界面回归】【真实输入】在隔离 Wayland 会话中验证贴图菜单、变换、抓取与独立交接"""

import argparse
import hashlib
import io
import json
from pathlib import Path
import shutil
import subprocess
import time

from PIL import Image, ImageDraw
from wayland_model_locator import download_position
import wayland_test_session as session


def capture(artifacts, name):
    """【界面回归】【屏幕取样】保存 name 对应隔离桌面截图并返回 RGB 图像，避免复用旧证据"""
    path = artifacts / f"{name}.png"
    session.screenshot(path)
    return Image.open(path).convert("RGB")


def image_rect(artifacts, name):
    """【界面回归】【贴图定位】按 name 截图中的红绿原图和青色边框返回逻辑矩形，找不到时失败"""
    image = capture(artifacts, name)
    points = []
    pixels = image.tobytes()
    for index in range(image.width * image.height):
        red, green, blue = pixels[index * 3:index * 3 + 3]
        if ((red > 195 and 40 < green < 75 and 50 < blue < 85)
                or (green > 170 and 30 < red < 75 and 60 < blue < 100)
                or (green > 210 and 70 < red < 120 and 190 < blue < 230)):
            points.append((index % image.width, index // image.width))
    if not points:
        raise AssertionError(f"the pinned source image did not appear: {name}")
    return min(point[0] for point in points), min(point[1] for point in points), max(point[0] for point in points) + 1, max(point[1] for point in points) + 1


def center(rect):
    """【界面回归】【矩形中心】返回 rect 的中心逻辑坐标，不换算为物理像素"""
    return round((rect[0] + rect[2]) / 2), round((rect[1] + rect[3]) / 2)


def right(rect):
    """【界面回归】【右键菜单】在 rect 中心发送右键并等待菜单配置，不发送左键确认"""
    x, y = center(rect)
    session.control("click", "--logical", x, y, "--button", "right")
    time.sleep(0.15)


def key(value):
    """【界面回归】【原生按键】用虚拟键盘向小贴图发送 value，避免 Xvfb 窗口聚焦移动硬件指针"""
    session.control("key", value)
    time.sleep(0.05)


def clipboard(environment):
    """【界面回归】【原图读取】读取 environment 中 PNG 剪贴板并返回未缩放 RGBA 图像"""
    png = subprocess.check_output(["wl-paste", "--no-newline", "--type", "image/png"], env=environment, timeout=3)
    return Image.open(io.BytesIO(png)).convert("RGBA")


def wait_window(app_id, timeout=4):
    """【界面回归】【窗口等待】等待 app_id 对应唯一原生窗口，重复或超时均失败"""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        matching = [item for item in session.windows() if item["app_id"] == app_id]
        if len(matching) == 1:
            return matching[0]
        time.sleep(0.04)
    raise AssertionError(f"window did not appear: {app_id}")


def same(actual, expected):
    """【界面回归】【像素对照】检查 actual 与 expected 的原图分辨率和全部通道，不接受预览缩放"""
    if actual.size != expected.size or actual.tobytes() != expected.tobytes():
        raise AssertionError(f"source export changed: {actual.size} != {expected.size}")


def wait_closed(app_id, timeout=3):
    """【界面回归】【窗口关闭】等待 app_id 表面从合成器消失，超过 timeout 秒仍残留时失败"""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if not any(item["app_id"] == app_id for item in session.windows()):
            return
        time.sleep(0.04)
    raise AssertionError(f"the native window was not destroyed: {app_id}")


def handoffs(rect, expected, artifacts, environment):
    """【界面回归】【独立交接】从 rect 贴图打开编辑及首次 OCR；核对 expected 像素和无下载状态，保存 artifacts 证据"""
    right(rect)
    for _ in range(7):
        key("Down")
    key("Return")
    editor = wait_window("mark-shot")
    capture(artifacts, "pin-editor")
    key("ctrl+c")
    same(clipboard(environment), expected)
    wait_closed("mark-shot")

    # 1. 【界面回归】【首次引导】启用文字选择只打开模型面板，测试不点击下载或写入模型
    right(rect)
    for _ in range(10):
        key("Down")
    key("Return")
    guide = wait_window("mark-shot")
    time.sleep(0.2)
    image = capture(artifacts, "pin-ocr-first-use")
    download_position(image, index=0)
    models = Path(environment["XDG_DATA_HOME"]) / "mark-shot/models"
    if models.exists() and any(models.iterdir()):
        raise AssertionError("first-use pin OCR downloaded model files without consent")
    session.control("close", guide["id"])
    wait_closed("mark-shot")
    if editor["pid"] == guide["pid"]:
        raise AssertionError("the editor and OCR guide did not use independent tasks")


def cached_text(binary, models, artifacts, environment, config):
    """【界面回归】【内置识别】复制 models 已校验资源，验证 binary 贴图中文字拖选与全文复制；证据写入 artifacts，不下载模型"""
    destination = Path(environment["XDG_DATA_HOME"]) / "mark-shot/models"
    destination.mkdir(parents=True, exist_ok=True)
    for name in ("pp-ocrv5_mobile_det.onnx", "pp-ocrv5_mobile_rec.onnx", "ppocrv5_dict.txt"):
        shutil.copyfile(models / name, destination / name)
    fixture = Path(__file__).resolve().parents[1] / "tests/fixtures/ocr.png"
    with (artifacts / "pin-ocr.log").open("w") as errors:
        process = subprocess.Popen([str(binary), "--pin", "--position", "80,180", str(fixture), "--config", str(config)],
                                   env=environment, stdout=subprocess.DEVNULL, stderr=errors)
        try:
            time.sleep(0.4)
            rect = image_rect(artifacts, "pin-ocr-initial")
            right(rect)
            for _ in range(10):
                key("Down")
            key("Return")
            time.sleep(0.2)
            session.wait_single_thread(process.pid, timeout=20)
            time.sleep(0.15)
            rect = image_rect(artifacts, "pin-ocr-ready")

            # 1. 【界面回归】【文字拖选】只有贴图自身识别就绪，左键拖选后 Ctrl+C 才提供文字而不是原图
            session.control("move", "--logical", rect[0] + 40, rect[1] + 60)
            time.sleep(0.08)
            session.control("drag", "--from", rect[0] + 40, rect[1] + 60,
                            "--to", rect[0] + 490, rect[1] + 60, "--steps", 12)
            time.sleep(0.1)
            capture(artifacts, "pin-ocr-selection")
            session.control("screenshot", "--out", artifacts / "pin-ocr-cursor.png", "--cursor")
            key("ctrl+c")
            selected = session.read_clipboard(environment)
            if "Wayland Native Screenshot" not in selected:
                raise AssertionError(f"the pin did not copy its selected OCR line: {selected!r}")
            right(rect)
            for _ in range(10):
                key("Down")
            key("Return")
            complete = session.read_clipboard(environment)
            for line in ("Wayland Native Screenshot", "Mark Shot OCR 2026", "Local text recognition"):
                if line not in complete:
                    raise AssertionError(f"Copy Image Text lost {line!r}: {complete!r}")
            if any(item["app_id"] == "mark-shot" for item in session.windows()):
                raise AssertionError("cached text selection unexpectedly launched a separate editor")
            (artifacts / "pin-recognized.txt").write_text(complete)

            # 2. 【界面回归】【模式切换】关闭文字选择后原图复制仍逐像素一致，识别任务不保留空闲线程
            right(rect)
            for _ in range(11):
                key("Down")
            key("Return")
            click = center(rect)
            session.control("move", "--logical", *click)
            key("ctrl+c")
            same(clipboard(environment), Image.open(fixture).convert("RGBA"))
            before = session.resources(process.pid)
            time.sleep(1)
            after = session.resources(process.pid)
            if after["threads"] != 1 or after["cpu_ticks"] != before["cpu_ticks"]:
                raise AssertionError(f"completed OCR kept idle workers: {before} -> {after}")
            key("Escape")
            process.wait(timeout=3)
            return {"cached_text_selection": True, "copy_image_text": True, "ocr_idle": after}
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)


def run(binary, artifacts, models=None):
    """【界面回归】【验证流程】使用 binary 执行真实菜单动作，将独立证据写入新 artifacts 目录"""
    environment = session.environment()
    artifacts.mkdir(parents=True, exist_ok=False)
    config = artifacts / "config.toml"
    config.write_text('[general]\nto = ""\noverlay = "window"\n[save]\ndirectory = ' + json.dumps(str(artifacts / "saved")) + '\nmonth_format = ""\n[notifications]\nenabled = false\n[history]\nenabled = false\n[log]\nlevel = "info"\n')
    source = Image.new("RGBA", (240, 120), (210, 55, 65, 255))
    ImageDraw.Draw(source).rectangle((120, 0, 239, 119), fill=(50, 190, 80, 255))
    source_path = artifacts / "source.png"
    source.save(source_path)
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    with (artifacts / "pin.log").open("w") as log:
        process = subprocess.Popen([str(binary), "--pin", "--position", "110,150", str(source_path), "--config", str(config)], env=dict(environment, LANG="en_US.UTF-8", LC_ALL="en_US.UTF-8"), stdout=subprocess.DEVNULL, stderr=log)
        try:
            time.sleep(0.4)
            rect = image_rect(artifacts, "pin-initial")
            if abs(rect[0] - 110) > 2 or abs(rect[1] - 150) > 2:
                raise AssertionError(f"the selected screenshot position was not restored: {rect}")
            right(rect)
            capture(artifacts, "pin-menu")
            key("Down")
            key("Down")
            key("Return")
            time.sleep(0.18)
            rect = image_rect(artifacts, "pin-rotated")
            (artifacts / "click.json").write_text(session.control("click", "--logical", *center(rect)))
            session.control("screenshot", "--out", artifacts / "pin-cursor.png", "--cursor")
            key("ctrl+c")
            same(clipboard(environment), source.transpose(Image.Transpose.ROTATE_270))

            # 1. 【界面回归】【显示缩放】滚轮改变显示大小，但复制仍使用旋转后的原图
            session.control("scroll", "--dy", -30, "--to", *center(rect))
            time.sleep(0.15)
            zoomed = image_rect(artifacts, "pin-zoomed")
            if zoomed == rect:
                raise AssertionError("scrolling did not change the pin size")
            key("ctrl+c")
            same(clipboard(environment), source.transpose(Image.Transpose.ROTATE_270))

            # 2. 【界面回归】【抓取释放】通过菜单打开另存为，必须能按 Escape 取消并回到贴图
            right(zoomed)
            key("Up")
            key("Up")
            key("Return")
            wait_window("mark-shot-save-as")
            capture(artifacts, "pin-save-dialog")
            key("Escape")
            deadline = time.monotonic() + 3
            while time.monotonic() < deadline and any(item["app_id"] == "mark-shot-save-as" for item in session.windows()):
                time.sleep(0.04)
            if process.poll() is not None or any(item["app_id"] == "mark-shot-save-as" for item in session.windows()):
                raise AssertionError(f"Save As cancellation did not return control to the pin: {process.poll()}")
            rect = image_rect(artifacts, "pin-after-save-cancel")
            right(rect)
            key("Up")
            key("Up")
            key("Return")
            dialog = session.size_window(wait_window("mark-shot-save-as"), 660, 440)
            time.sleep(0.2)
            capture(artifacts, "pin-save-confirm")
            (artifacts / "save-window.json").write_text(json.dumps(dialog, indent=2))
            session.click(dialog, 604, 404)
            capture(artifacts, "pin-save-after-click")
            wait_closed("mark-shot-save-as")
            saved = list((artifacts / "saved").glob("*.png"))
            if len(saved) != 1:
                raise AssertionError(f"Save As did not create exactly one source image: {saved}")
            same(Image.open(saved[0]).convert("RGBA"), source.transpose(Image.Transpose.ROTATE_270))

            # 3. 【界面回归】【置顶角色】关闭置顶后出现普通窗口，再开启置顶时仍显示同一原图
            right(rect)
            for _ in range(6):
                key("Down")
            key("Return")
            item = wait_window("mark-shot-pin")
            if not item["floating"]:
                session.control("float", item["id"])
            time.sleep(0.2)
            rect = image_rect(artifacts, "pin-window-role")
            right(rect)
            for _ in range(6):
                key("Down")
            key("Return")
            time.sleep(0.2)
            if any(item["app_id"] == "mark-shot-pin" for item in session.windows()):
                raise AssertionError("the ordinary pin window did not return to layer-shell")
            rect = image_rect(artifacts, "pin-top-role")
            model_directory = Path(environment["XDG_DATA_HOME"]) / "mark-shot/models"
            if model_directory.exists():
                raise AssertionError("ordinary pin operations initialized OCR models")
            handoffs(rect, source.transpose(Image.Transpose.ROTATE_270), artifacts, environment)
            rect = image_rect(artifacts, "pin-after-handoffs")
            before = session.resources(process.pid)
            time.sleep(1.0)
            after = session.resources(process.pid)
            if after["cpu_ticks"] > before["cpu_ticks"] + 1:
                raise AssertionError(f"idle pin still redraws: {before} -> {after}")
            session.control("click", "--logical", *center(rect), "--count", 2)
            process.wait(timeout=3)
            if process.returncode != 0:
                raise AssertionError(f"pin exited with {process.returncode}")
            result = {"sha256": fingerprint, "position": True, "native_popup": True, "rotation_source_pixels": True,
                      "zoom_preserves_export": True, "menu_save_cancel": True, "save_source_pixels": True, "layer_window_transition": True,
                      "double_click_close": True, "edit_source_pixels": True, "ocr_first_use_consent": True, "idle": after}
            if models:
                result.update(cached_text(binary, models, artifacts, environment, config))
            (artifacts / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)
    print("Wayland pin menu, source export, save grab, topmost transition and idle checks passed")


def main():
    """【界面回归】【命令入口】读取二进制和新证据目录并执行验证，异常保持非零退出码"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, default=Path("target/wayland-ui-parity"))
    parser.add_argument("--models", type=Path, help="existing PP-OCRv5 models for cached pin text-selection checks")
    options = parser.parse_args()
    run(options.binary.resolve(), options.artifacts.resolve(), options.models.resolve() if options.models else None)


if __name__ == "__main__":
    main()
