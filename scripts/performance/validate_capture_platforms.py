"""【截图回归】【平台验证】私有桌面验证共享槽位、X11曝光及双输出分数比例的实际标注像素。"""

import argparse
import hashlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path

from PIL import Image, ImageChops

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from wayland_headless_runtime import HeadlessRuntime
from wayland_pin_pointer import private_environment
from wayland_virtual_pointer import VirtualPointer


def check_test(binary, name, environment, root):
    """【截图回归】【原生接口】按 binary、name 和 environment 执行真实测试，将结果写入 root，失败直接终止。"""
    result = subprocess.run([str(binary), name, "--ignored", "--nocapture"],
                            env=environment, capture_output=True, text=True, timeout=30, check=False)
    (root / "shared-buffer.log").write_text(result.stdout + result.stderr)
    result.check_returncode()
    if "1 passed; 0 failed;" not in result.stdout:
        raise RuntimeError("the requested shared-buffer test did not execute exactly once")


def capture(binary, root, environment, mode):
    """【截图回归】【双屏绘图】按 binary、root、environment 和 mode 完成两屏矩形，返回实际截图路径。"""
    config = root / "app.toml"
    config.write_text('[desktop]\ntray_enabled=false\nshortcuts_enabled=false\n[history]\nenabled=false\n')
    environment = dict(environment, MARK_SHOT_CAPTURE_SCENE="1" if mode == "sparse" else "0")
    log = root / f"{mode}.log"
    with log.open("w") as output:
        process = subprocess.Popen([str(binary), "--config", str(config), "--fullscreen", "--all-outputs", "--speed"],
                                   env=environment, stdout=output, stderr=output)
        try:
            deadline = time.monotonic() + 15
            while "frames staged + flushed" not in log.read_text():
                if process.poll() is not None or time.monotonic() >= deadline:
                    raise RuntimeError(f"dual-output capture did not start: {log}")
                time.sleep(.03)
            with private_environment(environment), VirtualPointer(bind_output=False) as pointer:
                pointer.move_abs(1100, 160, 1760, 720)
                # 1. 【截图回归】【键盘就绪】无实体键盘的座席先处理新键盘与空按键焦点，再发送实际快捷键
                subprocess.run(["wtype", "-s", "300", "-k", "r", "-s", "100"], env=environment, check=True)
                for start, end in [((1100, 160), (1470, 370)), ((140, 160), (470, 370))]:
                    pointer.move_abs(*start, 1760, 720)
                    pointer.press()
                    time.sleep(.1)
                    pointer.move_abs(*end, 1760, 720)
                    time.sleep(.15)
                    pointer.release()
                    time.sleep(.2)
            time.sleep(.4)
            paths = {}
            for name in ["HEADLESS-1", "HEADLESS-2"]:
                path = root / f"{mode}-{name}.png"
                subprocess.run(["grim", "-o", name, str(path)], env=environment, check=True)
                paths[name] = path
            return paths
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)
            time.sleep(.2)


def compare(first, second):
    """【截图回归】【实际像素】按 first、second 比较工具栏外像素，返回标注数量与最大通道差异，未绘出标注直接失败。"""
    first = Image.open(first).convert("RGB")
    second = Image.open(second).convert("RGB")
    area = (0, 170, first.width, first.height)
    first, second = first.crop(area), second.crop(area)
    pixels = second.tobytes()
    red = sum(r > 150 and r > g * 1.4 and r > b * 1.4
              for r, g, b in zip(pixels[0::3], pixels[1::3], pixels[2::3]))
    maximum = max(high for _, high in ImageChops.difference(first, second).getextrema())
    if red < 1000 or maximum > 2:
        raise RuntimeError(f"fractional-output pixels differ: red={red}, maximum={maximum}")
    return {"annotation_pixels": red, "maximum_channel_difference": maximum}


def main():
    """【截图回归】【命令入口】读取 binary、test-binary、sway 和 root，运行真实平台接口与双屏对照，保存验证报告。"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--test-binary", type=Path, required=True)
    parser.add_argument("--sway", type=Path, default=Path("target/test-tools/sway/usr/bin/sway"))
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    root = options.root.resolve()
    root.mkdir(mode=0o700)
    test = options.test_binary.resolve()
    env = {key: value for key, value in os.environ.items() if key not in ("WAYLAND_DISPLAY", "WAYLAND_SOCKET", "NIRI_SOCKET")}
    # 2. 【截图回归】【私有曝光】测试脚本只接收自行创建的显示器编号，不连接实际桌面
    with (root / "x11-test.log").open("w") as log:
        subprocess.run(["xvfb-run", "-a", "-s", "-screen 0 1920x1080x24", "sh", "-c",
                        'export MARK_SHOT_TEST_X11_DISPLAY="$DISPLAY"; exec "$@"', "private-x11",
                        str(test), "partial_upload_and_exposure_keep_unchanged_pixels", "--ignored", "--nocapture"],
                       env=env, stdout=log, stderr=log, check=True, timeout=30)
    if "1 passed; 0 failed;" not in (root / "x11-test.log").read_text():
        raise RuntimeError("the requested X11 exposure test did not execute exactly once")
    with HeadlessRuntime(root / "wayland", options.sway) as environment:
        check_test(test, "partial_copy_preserves_active_source_and_rebuilds_reclaimed_spare",
                   dict(environment, MARK_SHOT_TEST_WAYLAND_DISPLAY=environment["WAYLAND_DISPLAY"]), root)
        first = capture(options.binary.resolve(), root, environment, "legacy")
        second = capture(options.binary.resolve(), root, environment, "sparse")
        report = {name: compare(first[name], second[name]) for name in first}
        report.update(scales=[1.25, 1.5], x11_exposure="passed", shared_buffer="passed",
                      binary_sha256=hashlib.sha256(options.binary.read_bytes()).hexdigest(),
                      test_binary_sha256=hashlib.sha256(test.read_bytes()).hexdigest())
        (root / "validation.json").write_text(json.dumps(report, indent=2))
        print(json.dumps(report), flush=True)


if __name__ == "__main__":
    main()
