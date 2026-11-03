#!/usr/bin/env python3
"""【文字菜单回归】【抓取释放】在私有会话打开并关闭原生菜单，严格验证下一次指针与新草稿位置"""

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tempfile
import time

from PIL import Image
import wayland_text_format_session as session
from wayland_text_pointer import pointer_context, click


def last_motion(protocol):
    """【文字菜单回归】【协议坐标】解析 protocol 最后销毁之后的末次 motion，返回逻辑坐标；缺少销毁或后续事件时失败"""
    destroyed = list(re.finditer(r"xdg_popup@\d+\.destroy\(", protocol))
    if not destroyed:
        raise AssertionError("no popup destruction was delivered")
    values = re.findall(r"wl_pointer@\d+\.motion, \(\d+, ([\d.-]+), ([\d.-]+)\)", protocol[destroyed[-1].end():])
    if not values:
        raise AssertionError("no pointer motion was delivered after popup destruction")
    return tuple(map(float, values[-1]))


def assert_motion(actual, expected):
    """【文字菜单回归】【坐标断言】比较 actual 与 expected，允许输出栅格量化但拒绝旧抓取坐标，无返回值"""
    if any(abs(first - second) > 1.5 for first, second in zip(actual, expected, strict=True)):
        raise AssertionError(f"pointer stayed at {actual} instead of {expected} after popup dismissal")


def screenshot(process, root, name):
    """【文字菜单回归】【画面记录】等待 process 处理输入，把 name 保存到 root，返回 RGB 画面；进程提前退出失败"""
    time.sleep(0.12)
    if process.poll() is not None:
        raise AssertionError(f"editor exited early: {process.returncode}")
    path = root / f"{name}.png"
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def run(binary, root, overlay, dismissal, pointer_backend):
    """【文字菜单回归】【最小场景】以 binary、overlay、dismissal、pointer_backend 检查关闭后输入，root 保存证据；返回实际坐标"""
    root.mkdir(parents=True, exist_ok=False)
    (root / "binary.json").write_text(json.dumps({"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                                                "bytes": binary.stat().st_size}, indent=2) + "\n")
    environment = session.environment()
    with session.input_context(), pointer_context(0.04, pointer_backend), tempfile.TemporaryDirectory(prefix="mark-shot-popup-pointer-") as temporary:
        private = Path(temporary)
        config = private / "config/mark-shot"
        config.mkdir(parents=True)
        (config / "config.toml").write_text(f'[general]\noverlay = "{overlay}"\n[notifications]\nenabled = false\n[history]\nenabled = false\n')
        environment.update(XDG_CONFIG_HOME=str(private / "config"), XDG_STATE_HOME=str(private / "state"),
                           XDG_DATA_HOME=str(private / "data"), WAYLAND_DEBUG="client")
        source = private / "source.png"
        Image.new("RGB", (1600, 900), (30, 34, 38)).save(source)
        with (root / "process.log").open("w") as log:
            process = subprocess.Popen([str(binary), "--fullscreen", "--fullscreen-default-tool", "text",
                                        "--default-color", "#FF4D4D", str(source)], env=environment, stderr=log)
            try:
                initial = screenshot(process, root, "initial")
                session.wait_single_thread(process.pid)
                origin = round(initial.width * 0.62), round(initial.height * 0.52)
                target = round(initial.width * 0.32), round(initial.height * 0.75)
                click(origin)
                session.key("a")
                screenshot(process, root, "typed")
                # 1. 【文字菜单回归】【真实抓取】在文字视口右键按下建立原生菜单，键盘取消或执行撤销
                click((origin[0] + 10, origin[1] + 12), "right")
                screenshot(process, root, "menu")
                if dismissal == "action":
                    session.key("Home")
                    session.key("Return")
                    session.key("ctrl+shift+z")
                else:
                    session.key("Escape")
                session.key("ctrl+Return")
                screenshot(process, root, "committed")
                # 2. 【文字菜单回归】【抓取之后】目标远离菜单与旧文字，正文必须进入该处的新草稿
                click(target)
                session.key("b")
                final = screenshot(process, root, "new-draft")
                protocol = (root / "process.log").read_text()
                if not re.search(r"xdg_popup@\d+\.grab\(", protocol) or not re.search(r"xdg_popup@\d+\.destroy\(", protocol):
                    raise AssertionError("the test did not create and destroy a native popup grab")
                actual = last_motion(protocol)
                observed = {"origin": origin, "target": target, "last_motion": actual,
                            "overlay": overlay, "dismissal": dismissal, "pointer_backend": pointer_backend}
                (root / "observed.json").write_text(json.dumps(observed, indent=2) + "\n")
                assert_motion(actual, target)
                region = final.crop((target[0] - 4, target[1] - 4, target[0] + 90, target[1] + 90))
                if not any(red > 100 and red > green * 1.7 and red > blue * 1.7 for red, green, blue in region.getdata()):
                    raise AssertionError("the post-menu draft did not receive text at the target")
                return observed
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=3)


def main():
    """【文字菜单回归】【入口】解析构建、证据与输入方式，保存指纹和实际结果，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--overlay", choices=("layer", "window"), default="layer")
    parser.add_argument("--dismissal", choices=("action", "cancel"), default="action")
    parser.add_argument("--pointer-backend", choices=("virtual", "x11"), default="virtual")
    options = parser.parse_args()
    binary = options.binary.resolve()
    result = {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "scenario": run(binary, options.artifacts.resolve(), options.overlay, options.dismissal, options.pointer_backend)}
    (options.artifacts / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
