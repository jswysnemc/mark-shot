#!/usr/bin/env python3
"""【Windows工作流】【实际应用】只在私有Wine与Xvfb驱动本次候选，验证原生捕获、编辑、保存与系统剪贴板"""

import argparse
from contextlib import contextmanager
import hashlib
import json
from pathlib import Path
import subprocess
import time

from PIL import Image, ImageChops, ImageGrab
from windows_test_runtime import run


def path_w(path, environment):
    """【Windows工作流】【系统路径】按path与environment获取本前缀中的Windows路径，返回系统字符串"""
    return subprocess.check_output(["winepath", "-w", str(path)], env=environment, text=True, timeout=8).strip()


def xdo(environment, *arguments):
    """【Windows工作流】【私有输入】按environment与arguments执行私有X11操作，返回输出，绝不使用用户显示"""
    return subprocess.check_output(["xdotool", *map(str, arguments)], env=environment, text=True, timeout=8).strip()


def window(environment, title, process=None, root=None):
    """【Windows工作流】【窗口等待】按environment、title与可选process等待唯一窗口，失败在root保存画面并报告数量或退出码"""
    deadline = time.monotonic() + 20
    values = []
    while time.monotonic() < deadline:
        try:
            values = xdo(environment, "search", "--onlyvisible", "--name", "^" + title + "$").splitlines()
            if len(values) == 1:
                return values[0]
        except subprocess.CalledProcessError:
            pass
        if process is not None and process.poll() is not None:
            raise AssertionError(f"Windows process exited before mapping {title}: {process.returncode}, matches={values}")
        time.sleep(0.04)
    if root is not None:
        snapshot(environment, root, "failed-window-lookup")
    raise AssertionError(f"Windows window did not become unique: {title}, matches={values}")


def snapshot(environment, root, name):
    """【Windows工作流】【实际画面】按environment读取私有显示并保存root中的name，返回RGB图像"""
    image = ImageGrab.grab(xdisplay=environment["DISPLAY"]).convert("RGB")
    image.save(root / (name + ".png"))
    return image


def key(environment, identity, value):
    """【Windows工作流】【原生键盘】按identity聚焦本次实际窗口并发送value，无返回值"""
    xdo(environment, "windowfocus", "--sync", identity)
    xdo(environment, "key", "--clearmodifiers", value)


@contextmanager
def application(binary, environment, root, config, log, *arguments):
    """【Windows工作流】【应用范围】按binary、environment与arguments启动实际应用，返回进程，结束时回收未退出实例"""
    process = subprocess.Popen(["wine", binary, "--config", config, "--nonotify", *arguments], env=environment, cwd=root,
                               stdout=log, stderr=log)
    try:
        yield process
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=3)


def clipboard(probe, environment, root, name):
    """【Windows工作流】【系统结果】用probe读取应用实际复制的图像并保存到root的name，返回完整RGBA图像"""
    target = root / (name + ".png")
    subprocess.run(["wine", probe, "image", path_w(target, environment)], env=environment, cwd=root, check=True, timeout=8)
    return Image.open(target).convert("RGBA")


def exact(actual, expected, description):
    """【Windows工作流】【原件校验】按actual与expected逐字节比较尺寸和RGBA，失败给出description，无返回值"""
    assert actual.size == expected.size and actual.tobytes() == expected.tobytes(), description


def wait_theme(environment, root, config, identity, mode, initial):
    """【Windows工作流】【主题确认】按mode发送一次切换，分别等待config保存与实际画面变化，返回呈现图像"""
    key(environment, identity, "ctrl+shift+l")
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        values = config.read_text()
        if f'theme = "{mode}"' in values or f'theme="{mode}"' in values:
            image = snapshot(environment, root, mode)
            if ImageChops.difference(image.crop((0, 0, image.width, 100)), initial.crop((0, 0, image.width, 100))).getbbox():
                return image
        time.sleep(0.04)
    raise AssertionError(f"Windows theme did not save and present {mode}")


def fixture(probe, environment, root):
    """【Windows工作流】【屏幕样本】按probe与environment启动独立原生Windows像素窗口，返回进程和预期RGBA，不替主应用捕获"""
    image = Image.new("RGBA", (1280, 800))
    image.putdata([(31 + x % 171, 37 + y % 163, 43 + (x + y) % 151, 255) for y in range(800) for x in range(1280)])
    path = root / "screen-source.png"
    image.save(path)
    process = subprocess.Popen(["wine", probe, "fixture"], env=environment, cwd=root,
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    deadline = time.monotonic() + 6
    while time.monotonic() < deadline:
        actual = ImageGrab.grab(xdisplay=environment["DISPLAY"]).convert("RGBA")
        if actual.tobytes() == image.tobytes():
            return process, image
        if process.poll() is not None:
            raise AssertionError("private screen fixture exited before mapping")
        time.sleep(0.04)
    process.terminate()
    process.wait(timeout=3)
    raise AssertionError("private screen fixture did not present its exact pixels")


def execute(command, environment, root, log):
    """【Windows工作流】【同一候选回归】按command、environment、root与log执行实际流程，返回完成状态并保存逐项证据"""
    binary, probe = command
    config_path = root / "config.toml"
    config_path.write_text('[general]\nto=""\n[ui]\ntheme="light"\n[notifications]\nenabled=false\n[history]\nenabled=false\n')
    config = path_w(config_path, environment)
    screen, expected = fixture(probe, environment, root)
    report = {"binary_sha256": hashlib.sha256(Path(binary).read_bytes()).hexdigest(), "cases": {}, "real_windows_validation": False}
    try:
        displays = subprocess.check_output(["wine", binary, "--config", config, "--list-displays"], env=environment, cwd=root, text=True, timeout=8)
        outputs = json.loads(displays)
        assert len(outputs) == 1 and outputs[0]["width"] == 1280 and outputs[0]["height"] == 800
        report["cases"]["displays"] = outputs
        captured = root / "region.png"
        result = subprocess.check_output(["wine", binary, "--config", config, "--nonotify", "--speed", "--capture-to", path_w(captured, environment),
                                          "--region", "100,120,200,160"], env=environment, cwd=root, text=True, timeout=20)
        exact(Image.open(captured).convert("RGBA"), expected.crop((100, 120, 300, 280)), "native capture changed original region pixels")
        report["cases"]["capture"] = json.loads(result)
        with application(binary, environment, root, config, log, "--fullscreen", "--to", "c") as process:
            identity = window(environment, "Mark Shot", process, root)
            time.sleep(0.4)
            light = snapshot(environment, root, "editor-light")
            dark = wait_theme(environment, root, config_path, identity, "dark", light)
            wait_theme(environment, root, config_path, identity, "light", dark)
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                restored = snapshot(environment, root, "editor-restored")
                if ImageChops.difference(light, restored).getbbox() is None:
                    break
                time.sleep(0.04)
            else:
                raise AssertionError("Windows editor did not exactly restore its light frame")
            key(environment, identity, "o")
            time.sleep(0.4)
            guide = snapshot(environment, root, "first-ocr-guide")
            assert ImageChops.difference(restored, guide).getbbox(), "first Windows OCR use did not display model guidance"
            key(environment, identity, "p")
            key(environment, identity, "ctrl+c")
            assert process.wait(timeout=8) == 0
        original = clipboard(probe, environment, root, "clipboard-original")
        exact(original, expected, "theme or OCR guide entered the exported screenshot")
        report["cases"]["editor"] = {"light_dark_light": True, "first_ocr_guide": True, "full_original_clipboard": True}
        saved = root / "saved"
        saved.mkdir()
        with application(binary, environment, root, config, log, "--fullscreen", "--to", "c", "--output", path_w(saved, environment)) as process:
            identity = window(environment, "Mark Shot", process, root)
            time.sleep(0.3)
            key(environment, identity, "p")
            xdo(environment, "mousemove", "--sync", 280, 270)
            xdo(environment, "mousedown", 1)
            for x, y in [(305, 282), (330, 292), (350, 311), (374, 322)]:
                xdo(environment, "mousemove", "--sync", x, y)
            xdo(environment, "mouseup", 1)
            time.sleep(0.2)
            key(environment, identity, "ctrl+c")
            assert process.wait(timeout=8) == 0
        edited = clipboard(probe, environment, root, "clipboard-edited")
        assert edited.size == expected.size
        difference = ImageChops.difference(edited.convert("RGB"), expected.convert("RGB")).getbbox()
        assert difference and difference[0] >= 260 and difference[1] >= 250 and difference[2] <= 400 and difference[3] <= 350, "pen export changed pixels outside its stroke"
        files = list(saved.rglob("*.png"))
        assert len(files) == 1, "the editor did not save exactly one complete screenshot"
        exact(Image.open(files[0]).convert("RGBA"), edited, "saving changed the clipboard screenshot")
        report["cases"]["annotation_save"] = {"pen_bounds": difference, "saved_equals_clipboard": True}
        with application(binary, environment, root, config, log, "--pin-clipboard") as process:
            identity = window(environment, "Pinned Mark Shot", process, root)
            time.sleep(0.3)
            snapshot(environment, root, "pinned-image")
            key(environment, identity, "ctrl+c")
            time.sleep(0.2)
            pinned = clipboard(probe, environment, root, "clipboard-pinned")
            exact(pinned, edited, "pinning changed original screenshot pixels")
            key(environment, identity, "Escape")
            assert process.wait(timeout=8) == 0
        report["cases"]["pin"] = {"original_resolution": edited.size, "original_clipboard": True}
        report["passed"] = True
        return subprocess.CompletedProcess(command, 0)
    finally:
        (root / "workflow.json").write_text(json.dumps(report, indent=2) + "\n")
        screen.terminate()
        screen.wait(timeout=3)


def main():
    """【Windows工作流】【私有入口】读取候选、验证程序与新root，输出同一构建结果，无参数、无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--probe", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    report = run(options.root.resolve(), [str(options.binary.resolve()), str(options.probe.resolve())], execute=execute)
    print(json.dumps(report, indent=2))
    raise SystemExit(report["exit_code"])


if __name__ == "__main__":
    main()
