#!/usr/bin/env python3
"""【输入法回归】【真实中文】在私有 niri 和独立 D-Bus 中验证 Fcitx 候选、焦点与文字外框"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import wayland_test_session as session
from PIL import Image
from wayland_fcitx_runtime import prepare_rime, private_bus, remote
from wayland_test_keyboard import runtime_environment, send


def wait_for(process, path, predicate, description, timeout=8):
    """【输入法回归】【协议等待】按 predicate 核验 process 的 path 日志；返回日志，timeout 秒内未满足 description 则失败"""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        text = path.read_text(errors="replace")
        if process.poll() is not None:
            raise AssertionError(
                f"editor exited during {description}: {process.returncode}"
            )
        if predicate(text):
            return text
        time.sleep(0.04)
    raise AssertionError(f"timed out waiting for {description}: {path}")


def surrounding(process, path, expected):
    """【输入法回归】【正文确认】等待 process 向 path 发送 expected 周边正文；返回协议日志，候选不能代替正文"""
    marker = f"set_surrounding_text(Some({json.dumps(expected, ensure_ascii=False)}),"
    return wait_for(process, path, lambda text: marker in text, repr(expected))


def capture(process, path):
    """【输入法回归】【界面读取】保存 process 的私有画面到 path；返回 RGB 图片，进程退出时失败"""
    if process.poll() is not None:
        raise AssertionError(f"editor exited before screenshot: {process.returncode}")
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def frame_pixels(image, point):
    """【输入法回归】【虚线识别】按 image 和 point 统计编辑区青绿色外框像素；返回数量，排除顶部工具栏"""
    x, y = point
    data = image.crop((x - 6, y - 6, x + 250, y + 80)).tobytes()
    return sum(
        green > 160 and green > red + 40 and green > blue
        for red, green, blue in zip(data[0::3], data[1::3], data[2::3])
    )


def scenario(binary, environment, root, overlay):
    """【输入法回归】【场景执行】按 binary、environment 和 overlay 验证真实输入；root 保存证据，返回焦点与候选统计"""
    config = Path(environment["XDG_CONFIG_HOME"]) / "mark-shot/config.toml"
    config.parent.mkdir(parents=True, exist_ok=True)
    config.write_text(
        f'[general]\noverlay="{overlay}"\n[history]\nenabled=false\n'
        '[notifications]\nenabled=false\n[ui]\nlanguage="chinese"\n'
    )
    state = Path(environment["XDG_STATE_HOME"]) / "mark-shot/tools.toml"
    state.parent.mkdir(parents=True, exist_ok=True)
    state.write_text(
        'font_size=20.0\nfont_size_unit="pt"\ntext_family="Inter"\nmagnifier=false\n'
    )
    root.mkdir(parents=True, exist_ok=True)
    path = root / "process.log"
    command = [
        str(binary),
        "--fullscreen",
        "--fullscreen-default-tool",
        "text",
        "--to",
        "s",
        "--default-color",
        "#FF4D4D",
        "--output",
        str(root / "export"),
    ]
    if overlay == "window":
        source = root / "source.png"
        Image.new("RGB", (1600, 900), (30, 34, 38)).save(source)
        command.append(str(source))
    with path.open("w") as log:
        process = subprocess.Popen(
            command,
            env=dict(environment, WAYLAND_DEBUG="client"),
            stdout=log,
            stderr=log,
        )
        try:
            wait_for(
                process,
                path,
                lambda text: re.search(r"zwp_text_input_v3@\d+\.enter", text),
                "text input focus",
            )
            initial = capture(process, root / "initial.png")
            point = round(initial.width * 0.62), round(initial.height * 0.52)
            session.control("click", "--logical", *point)
            surrounding(process, path, "")
            time.sleep(0.15)
            # 1. 【输入法回归】【焦点稳定】先锁定用户报告的循环，避免把无法输入归为输入法部署失败
            text = path.read_text(errors="replace")
            leaves = len(re.findall(r"wl_keyboard@\d+\.leave", text))
            if leaves:
                raise AssertionError(
                    f"text activation lost keyboard focus {leaves} times: {path}"
                )
            empty = capture(process, root / "empty.png")
            assert frame_pixels(empty, point) > 10, (
                "empty text has no dashed editing frame"
            )
            remote(environment, "-s", "rime")
            remote(environment, "-c")
            session.key("x")
            surrounding(process, path, "x")
            session.key("backspace")
            surrounding(process, path, "")
            send("ctrl+space")
            current, active = remote(environment, "-n"), remote(environment)
            assert (current, active) == ("rime", "2"), (
                f"Chinese trigger did not activate Rime: {current}, {active}"
            )
            session.key("n")
            session.key("i")
            wait_for(
                process,
                path,
                lambda text: '.preedit_string, (Some("ni")' in text,
                "Rime candidate ni",
            )
            capture(process, root / "candidate-ni.png")
            # 2. 【输入法回归】【连续确认】候选确认后继续输入，真实正文必须依次为你、你好和你好x
            session.key("space")
            surrounding(process, path, "你")
            for value in "hao":
                session.key(value)
            session.key("space")
            surrounding(process, path, "你好")
            chinese = capture(process, root / "committed.png")
            assert frame_pixels(chinese, point) > 10, (
                "committed text lost its editing frame"
            )
            send("ctrl+space")
            active = remote(environment)
            assert active == "1", f"English trigger did not deactivate Rime: {active}"
            session.key("x")
            surrounding(process, path, "你好x")
            session.key("backspace")
            session.key("ctrl+a")
            session.key("ctrl+c")
            session.wait_clipboard(environment, "你好")
            # 3. 【输入法回归】【重复绘制】等待至少一个光标周期，文字外框与键盘焦点保持
            time.sleep(0.6)
            later = capture(process, root / "later.png")
            assert frame_pixels(later, point) > 10, (
                "caret blink removed the editing frame"
            )
            text = path.read_text(errors="replace")
            assert not re.search(r"wl_keyboard@\d+\.leave", text), (
                "Chinese input lost keyboard focus"
            )
            if overlay == "layer":
                assert len(re.findall(r"set_keyboard_interactivity\(", text)) == 1, (
                    "text input changed layer focus policy"
                )
            return {
                "overlay": overlay,
                "preedit_batches": text.count(".preedit_string,"),
                "commits": text.count(".commit_string,"),
                "text": "你好",
                "focus_losses": 0,
            }
        finally:
            if process.poll() is None:
                process.terminate()
            process.wait(timeout=5)


def worker(binary, root):
    """【输入法回归】【独立输入法】按 binary 在 root 配置私有 Fcitx；返回两种窗口验证结果，结束时释放进程"""
    environment = session.environment()
    for name, directory in (
        ("HOME", "home"),
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_DATA_HOME", "data"),
        ("XDG_STATE_HOME", "state"),
        ("XDG_CACHE_HOME", "cache"),
    ):
        target = root / directory
        target.mkdir(parents=True, exist_ok=True)
        environment[name] = str(target)
    os.environ.update(environment)
    profile = root / "config/fcitx5/profile"
    profile.parent.mkdir(parents=True, exist_ok=True)
    profile.write_text(
        "[Groups/0]\nName=Default\nDefault Layout=us\nDefaultIM=rime\n"
        "[Groups/0/Items/0]\nName=keyboard-us\n[Groups/0/Items/1]\nName=rime\n[GroupOrder]\n0=Default\n"
    )
    prepare_rime(root, environment)
    with (root / "fcitx.log").open("w") as log:
        fcitx = subprocess.Popen(
            [
                "fcitx5",
                "-D",
                "--disable",
                "all",
                "--enable",
                "wayland,waylandim,keyboard,rime,classicui,dbus",
                "--verbose",
                "waylandim=5,key_trace=5",
            ],
            env=environment,
            stdout=log,
            stderr=log,
        )
        try:
            wait_for(
                fcitx,
                root / "fcitx.log",
                lambda text: "Loaded addon rime" in text,
                "private Rime initialization",
                timeout=15,
            )
            results = [
                scenario(binary, environment, root / overlay, overlay)
                for overlay in ("layer", "window")
            ]
            print(json.dumps(results, ensure_ascii=False))
        finally:
            fcitx.terminate()
            fcitx.wait(timeout=5)


def main():
    """【输入法回归】【命令入口】读取 binary 和 artifacts 参数并校验私有桌面；返回退出状态，不连接用户输入法"""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/mark-shot"))
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--private-bus", action="store_true", help=argparse.SUPPRESS)
    arguments = parser.parse_args()
    runtime_environment()
    root = arguments.artifacts.resolve()
    root.mkdir(parents=True, exist_ok=True)
    if arguments.private_bus:
        worker(arguments.binary.resolve(), root)
    else:
        with tempfile.TemporaryDirectory(
            prefix="mark-shot-fcitx-", dir=root
        ) as temporary:
            command = [
                "dbus-run-session",
                "--config-file",
                str(private_bus(Path(temporary))),
                "--",
                sys.executable,
                str(Path(__file__).resolve()),
                "--binary",
                str(arguments.binary.resolve()),
                "--artifacts",
                temporary,
                "--private-bus",
            ]
            result = subprocess.run(command, check=False, timeout=60)
            # 1. 【输入法回归】【证据保留】只复制日志与截图，输入法配置仍随私有目录释放
            for overlay in ("layer", "window"):
                source = Path(temporary) / overlay
                if source.exists():
                    shutil.copytree(source, root / overlay, dirs_exist_ok=True)
            shutil.copyfile(Path(temporary) / "fcitx.log", root / "fcitx.log")
            return result.returncode
    return 0


if __name__ == "__main__":
    sys.exit(main())
