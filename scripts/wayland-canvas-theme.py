#!/usr/bin/env python3
"""【画布主题回归】【真实界面】私有Wayland验证图片留白、导航和透明原件的主题往返"""

import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import time

from PIL import Image, ImageChops
import wayland_text_format_session as session
from wayland_text_pointer import pointer_context, click
from wayland_text_glyph_capture import capture
from wayland_theme_transition import toggle


def stable(process, root, name):
    """【画布主题回归】【稳定帧】按process连续采样至两张完整画面一致，保存root/name证据并返回像素"""
    previous = capture(process, root, name + "-first")
    for index in range(12):
        current = capture(process, root, f"{name}-{index}")
        if ImageChops.difference(previous, current).getbbox() is None:
            return current
        previous = current
    raise AssertionError(f"canvas did not stabilize: {name}")


def roundtrip(process, root, config, name):
    """【画布主题回归】【主题往返】验证浅色留白、深色差异和完整恢复，返回变化像素数量，不改变原件"""
    light = stable(process, root, name + "-light")
    # 1. 【画布主题回归】【显示留白】右下角远离工具栏、选框和原图，不能用按钮变色冒充画布修复
    sample = (light.width - 15, light.height - 15)
    minimum = 90 if name == "navigation" else 150
    assert min(light.getpixel(sample)) >= minimum, f"light canvas remained dark: {light.getpixel(sample)}"
    toggle(config, "dark", session.key, session.screenshot)
    dark = stable(process, root, name + "-dark")
    assert max(dark.getpixel(sample)) < 80, f"dark canvas did not update: {dark.getpixel(sample)}"
    count = sum(sum(a) - sum(b) > 200 for a, b in zip(light.getdata(), dark.getdata()))
    assert count > light.width * light.height // 4, "only controls changed, not canvas background"
    toggle(config, "light", session.key, session.screenshot)
    restored = stable(process, root, name + "-restored")
    assert ImageChops.difference(light, restored).getbbox() is None, "canvas cache did not restore exactly"
    return {"changed_pixels": count, "light": light.getpixel(sample), "dark": dark.getpixel(sample)}


def run(binary, root):
    """【画布主题回归】【完整执行】在root隔离配置，用binary验证正常与缩放画布并比较透明PNG，返回报告"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"),
                       XDG_STATE_HOME=str(root / "state"), WAYLAND_DEBUG="client", LANG="en_US.UTF-8")
    config = root / "config/mark-shot/config.toml"
    config.parent.mkdir(parents=True)
    config.write_text('[general]\noverlay="window"\nto="c"\n[history]\nenabled=false\n[notifications]\nenabled=false\n[ui]\ntheme="light"\n')
    source = Image.new("RGBA", (320, 200), (0, 0, 0, 0))
    source.paste((204, 102, 51, 255), (20, 20, 300, 180))
    source_path = root / "source.png"
    source.save(source_path)
    before = dict(os.environ)
    os.environ.update(environment)
    report = {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "passed": False}
    try:
        with session.input_context(), pointer_context() as pointer, (root / "process.log").open("w") as log:
            process = subprocess.Popen([str(binary), "--fullscreen", str(source_path)], env=environment, stderr=log)
            try:
                initial = stable(process, root, "initial")
                session.wait_single_thread(process.pid)
                if session.backend() is session.sway:
                    session.sway.keyboard_ready()
                report["normal"] = roundtrip(process, root, config, "normal")
                # 2. 【画布主题回归】【导航状态】使用实际滚轮缩放，不调用应用内部渲染入口
                session.key("s")
                click((initial.width // 2, initial.height // 2), "left")
                for _ in range(3):
                    pointer.scroll(dy=10.0)
                    time.sleep(0.05)
                time.sleep(1.1)
                zoomed = stable(process, root, "zoomed")
                region = (30, 70, initial.width - 30, initial.height - 30)
                assert ImageChops.difference(initial.crop(region), zoomed.crop(region)).getbbox(), "wheel did not change source preview"
                report["navigation"] = roundtrip(process, root, config, "navigation")
                session.key("ctrl+c")
                assert process.wait(timeout=8) == 0
                data = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment, timeout=5)
                (root / "export.png").write_bytes(data)
                exported = Image.open(io.BytesIO(data)).convert("RGBA")
                assert exported.size == source.size and exported.tobytes() == source.tobytes(), "theme changed transparent original export"
                report["transparent_export_exact"] = True
                report["passed"] = True
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=3)
    finally:
        os.environ.clear()
        os.environ.update(before)
        (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    """【画布主题回归】【命令入口】读取binary与新root并执行隔离测试，输出结果，无参数、无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
