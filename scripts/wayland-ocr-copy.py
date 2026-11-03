#!/usr/bin/env python3
"""【Wayland 测试】【OCR 复制】在隔离 niri 会话中验证识别结果复制，不下载模型"""

import argparse
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

from PIL import Image

from wayland_model_locator import download_position
import wayland_test_session as session


def child_environment():
    """【Wayland 测试】【会话环境】读取子会话状态并验证进程；返回隔离的环境变量"""
    return session.environment()


def key(value):
    """【Wayland 测试】【键盘动作】向子会话发送 value 指定的快捷键；失败时抛出异常"""
    subprocess.run(["niri-use", "--child", "key", value], check=True, stdout=subprocess.DEVNULL)


def wait_for_scan(process, log):
    """【Wayland 测试】【识别等待】等待真实后台识别完成；异常退出或超时返回错误"""
    deadline = time.monotonic() + 12
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError("the editor exited before recognition completed")
        text = log.read_text() if log.exists() else ""
        if "ocr: 3 line(s)" in text:
            return
        time.sleep(0.03)
    raise RuntimeError("OCR did not finish within 12 seconds")


def run(binary, models, artifacts, first_use, rapid):
    """【Wayland 测试】【复制回归】使用固定图片和本地模型驱动 OCR；只接受包含三行原文的文字剪贴板"""
    root = Path(__file__).resolve().parents[1]
    environment = child_environment()
    artifacts.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="mark-shot-ocr-test-") as directory:
        temporary = Path(directory)
        config = temporary / "config/mark-shot"
        data = temporary / "data/mark-shot/models"
        state = temporary / "state"
        config.mkdir(parents=True)
        data.mkdir(parents=True)
        required = ("pp-ocrv5_mobile_det.onnx", "pp-ocrv5_mobile_rec.onnx", "ppocrv5_dict.txt")
        if not first_use:
            for name in required:
                shutil.copyfile(models / name, data / name)
        (config / "config.toml").write_text('[general]\nto = "c"\n[notifications]\nenabled = false\n[ocr]\nresult_panel = false\n')
        environment.update(XDG_CONFIG_HOME=str(temporary / "config"), XDG_DATA_HOME=str(temporary / "data"), XDG_STATE_HOME=str(state))
        if first_use:
            environment.update(LANG="en_US.UTF-8", LC_ALL="en_US.UTF-8")
        output = artifacts / "process.log"
        with output.open("w") as stderr:
            mode = ["--ocr"]
            process = subprocess.Popen([str(binary), *mode, "--to", "c", str(root / "tests/fixtures/ocr.png")], env=environment, stderr=stderr)
            log = state / "mark-shot/mark-shot.log"
            try:
                if first_use:
                    time.sleep(0.2)
                    if list(data.iterdir()):
                        raise RuntimeError("first use downloaded models before consent")
                    subprocess.run(["niri-use", "--child", "screenshot", "--out", str(artifacts / "first-use.png")], check=True, stdout=subprocess.DEVNULL)
                    # 1. 填入已校验资源以复现下载完成事件，测试不需要再次访问网络
                    for name in required:
                        shutil.copyfile(models / name, data / name)
                    with Image.open(artifacts / "first-use.png") as screenshot:
                        position = download_position(screenshot)
                    session.control("click", "--logical", *position)
                wait_for_scan(process, log)
                if rapid:
                    subprocess.run(["niri-use", "--child", "key", "ctrl+a", "ctrl+c"], check=True, stdout=subprocess.DEVNULL)
                else:
                    key("ctrl+a")
                    time.sleep(0.08)
                    subprocess.run(["niri-use", "--child", "screenshot", "--out", str(artifacts / "after-select-all.png")], check=True, stdout=subprocess.DEVNULL)
                    key("ctrl+c")
                time.sleep(0.08)
                offered = subprocess.check_output(["wl-paste", "--list-types"], env=environment, text=True)
                (artifacts / "clipboard-types.txt").write_text(offered)
                if "text/plain" not in offered:
                    raise RuntimeError(f"OCR copied an image instead of text: {offered.strip()}")
                text = subprocess.check_output(["wl-paste"], env=environment, text=True)
                (artifacts / "recognized.txt").write_text(text)
                for line in ("Wayland Native Screenshot", "Mark Shot OCR 2026", "Local text recognition"):
                    if line not in text:
                        raise RuntimeError(f"recognition or clipboard omitted {line!r}")
                print("OCR text copy passed")
            finally:
                if log.exists():
                    shutil.copyfile(log, artifacts / "application.log")
                if process.poll() is None:
                    key("Escape")
                    try:
                        process.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        process.terminate()
                        process.wait(timeout=3)


def main():
    """【Wayland 测试】【入口】解析构建产物、已下载模型和输出目录；测试失败返回非零退出码"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--models", required=True, type=Path)
    parser.add_argument("--artifacts", type=Path, default=Path("target/wayland-ocr-copy"))
    parser.add_argument("--first-use", action="store_true")
    parser.add_argument("--rapid", action="store_true")
    options = parser.parse_args()
    run(options.binary.resolve(), options.models.resolve(), options.artifacts.resolve(), options.first_use, options.rapid)


if __name__ == "__main__":
    main()
