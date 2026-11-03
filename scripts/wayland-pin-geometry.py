#!/usr/bin/env python3
"""【贴图回归】【真实几何】验证边缘缩放、预览抓取、可见裁切与不同分数比例双输出移动"""

import argparse
import hashlib
import io
import json
from pathlib import Path
import subprocess
import time

from PIL import Image, ImageDraw
from wayland_headless_runtime import HeadlessRuntime
from wayland_pin_pointer import GlobalPointer, private_environment
from wayland_test_session import resources, wait_single_thread


def screenshot(environment, artifacts, name):
    """【贴图回归】【屏幕取样】将 environment 布局的逻辑截图写入 artifacts/name；返回 RGB 图像"""
    path = artifacts / f"{name}.png"
    subprocess.run(["grim", "-s", "1", str(path)], env=environment, check=True, timeout=3)
    return Image.open(path).convert("RGB")


def image_rect(image):
    """【贴图回归】【原图定位】从 image 中原图红绿区域返回实际可见矩形；不读取应用几何日志"""
    points = []
    pixels = image.get_flattened_data() if hasattr(image, "get_flattened_data") else image.getdata()
    for index, (red, green, blue) in enumerate(pixels):
        if (red > 195 and 40 < green < 75 and 50 < blue < 85) or (green > 170 and 30 < red < 75 and 60 < blue < 100):
            points.append((index % image.width, index // image.width))
    if not points:
        raise AssertionError("the pinned image is not visible")
    # 1. 【贴图回归】【边框补偿】纯色原图位于两像素内边框之内，只在确实有边框的方向补齐
    left, top = min(x for x, _ in points), min(y for _, y in points)
    right, bottom = max(x for x, _ in points) + 1, max(y for _, y in points) + 1
    return left - (2 if left > 2 else 0), top - (2 if top > 2 else 0), right + 2, bottom + 2


def check_rect(actual, expected, tolerance=2):
    """【贴图回归】【独立断言】actual 四条可见边必须接近 expected，tolerance 仅覆盖分数像素边界"""
    if any(abs(a - b) > tolerance for a, b in zip(actual, expected)):
        raise AssertionError(f"visible geometry mismatch: {actual} != {expected}")


def key(environment, value):
    """【贴图回归】【原生键盘】在 environment 中用 wtype 发送 value，不连接 X11 或真实桌面"""
    modifiers = value.split("+")
    # 1. 【贴图回归】【设备就绪】headless 座席没有常驻键盘，先等待能力通知和客户端绑定，再发送真实按下
    command = ["wtype", "-s", "100"]
    for modifier in modifiers[:-1]:
        command += ["-M", modifier]
    command += ["-k", modifiers[-1]]
    for modifier in reversed(modifiers[:-1]):
        command += ["-m", modifier]
    command += ["-s", "50"]
    subprocess.run(command, env=environment, check=True, timeout=3)
    time.sleep(0.06)


def same_source(environment, expected):
    """【贴图回归】【原图复制】逐像素核对 environment 剪贴板与 expected，不允许预览倍率进入 PNG"""
    key(environment, "ctrl+c")
    data = subprocess.check_output(["wl-paste", "--no-newline", "--type", "image/png"], env=environment, timeout=3)
    actual = Image.open(io.BytesIO(data)).convert("RGBA")
    if actual.size != expected.size or actual.tobytes() != expected.tobytes():
        raise AssertionError("resizing or moving changed source export pixels")


class PinProcess:
    """【贴图回归】【窗口生命周期】启动独立贴图，用实际退出结果清理本次进程"""

    def __init__(self, binary, source, config, environment, artifacts, name, position="300,220"):
        """【贴图回归】【进程构造】保存 binary、source、config 与 environment，日志写入 artifacts/name"""
        self.log = (artifacts / f"{name}.log").open("w")
        self.process = subprocess.Popen([str(binary), "--pin", "--position", position, str(source), "--config", str(config)],
                                        env=environment, stdout=subprocess.DEVNULL, stderr=self.log)

    def __enter__(self):
        """【贴图回归】【窗口等待】等待本实例启动任务结束，返回实际进程句柄"""
        wait_single_thread(self.process.pid)
        time.sleep(0.2)
        return self.process

    def __exit__(self, *_arguments):
        """【贴图回归】【进程释放】只结束本实例仍在运行的贴图，不用旧 PID 记录终止进程"""
        if self.process.poll() is None:
            self.process.terminate()
            self.process.wait(timeout=3)
        self.log.close()


def drag(pointer, start, end, steps=12):
    """【贴图回归】【累计拖动】用 pointer 从 start 移到 end，预先独立移动帧防止沿旧坐标按下"""
    pointer.move(start)
    time.sleep(0.08)
    pointer.press()
    time.sleep(0.1)
    for index in range(1, steps + 1):
        pointer.move((start[0] + (end[0] - start[0]) * index / steps,
                      start[1] + (end[1] - start[1]) * index / steps))
        time.sleep(0.02)
    time.sleep(0.2)


def edge_checks(binary, source_path, source, config, environment, artifacts):
    """【贴图回归】【八方向操作】逐项独立启动 binary，按 Qt 固定边公式核对预览与释放后的矩形，保存原图证据"""
    scenarios = [
        ("left", (302, 280), (182, 280), (180, 190, 540, 370)),
        ("right", (538, 280), (658, 280), (300, 190, 660, 370)),
        ("top", (420, 222), (420, 162), (240, 160, 600, 340)),
        ("bottom", (420, 338), (420, 398), (240, 220, 600, 400)),
        ("top-left", (302, 222), (182, 162), (180, 160, 540, 340)),
        ("top-right", (538, 222), (658, 162), (300, 160, 660, 340)),
        ("bottom-left", (302, 338), (182, 398), (180, 220, 540, 400)),
        ("bottom-right", (538, 338), (658, 398), (300, 220, 660, 400)),
    ]
    for name, start, end, expected in scenarios:
        with PinProcess(binary, source_path, config, environment, artifacts, name) as process, GlobalPointer() as pointer:
            check_rect(image_rect(screenshot(environment, artifacts, f"{name}-initial")), (300, 220, 540, 340))
            drag(pointer, start, end)
            check_rect(image_rect(screenshot(environment, artifacts, f"{name}-preview")), expected)
            pointer.release()
            time.sleep(0.2)
            check_rect(image_rect(screenshot(environment, artifacts, f"{name}-released")), expected)
            pointer.move(((expected[0] + expected[2]) / 2, (expected[1] + expected[3]) / 2))
            pointer.click()
            same_source(environment, source)
            key(environment, "Escape")
            process.wait(timeout=3)
            if process.returncode:
                raise AssertionError(f"{name} exited with {process.returncode}")
    return [item[0] for item in scenarios]


def clipping_and_cross_output(binary, source_path, source, config, environment, artifacts):
    """【贴图回归】【裁切与跨屏】核对 binary 超屏不压缩、嵌边移动、跨屏抓取延续与原生菜单；证据写入 artifacts"""
    with PinProcess(binary, source_path, config, environment, artifacts, "cross-output") as process, GlobalPointer() as pointer:
        drag(pointer, (420, 280), (40, 280))
        pointer.release()
        time.sleep(0.2)
        clipped = screenshot(environment, artifacts, "clipped-left")
        check_rect(image_rect(clipped), (0, 220, 160, 340))
        if clipped.getpixel((20, 280)) != (210, 55, 65) or clipped.getpixel((60, 280)) != (50, 190, 80):
            raise AssertionError("clipping squeezed the complete source instead of keeping its pixel scale")
        # 1. 【贴图回归】【嵌边恢复】屏幕左侧被裁切的边只移动，不误用它调整完整图片大小
        drag(pointer, (1, 280), (201, 280))
        pointer.release()
        time.sleep(0.2)
        check_rect(image_rect(screenshot(environment, artifacts, "embedded-edge-recovered")), (120, 220, 360, 340))

        # 2. 【贴图回归】【连续跨屏】同一次左键抓取跨越两个输出，继续移动后再释放，不重建输入表面
        drag(pointer, (240, 280), (1150, 280))
        check_rect(image_rect(screenshot(environment, artifacts, "cross-output-held")), (1030, 220, 1270, 340))
        pointer.move((1250, 340))
        time.sleep(0.2)
        check_rect(image_rect(screenshot(environment, artifacts, "cross-output-continued")), (1130, 280, 1370, 400))
        pointer.release()
        time.sleep(0.2)
        check_rect(image_rect(screenshot(environment, artifacts, "cross-output-released")), (1130, 280, 1370, 400))
        pointer.move((1250, 340))
        pointer.click()
        same_source(environment, source)
        pointer.click(button=0x111)
        screenshot(environment, artifacts, "second-output-menu")
        key(environment, "Down")
        key(environment, "Down")
        key(environment, "Return")
        time.sleep(0.2)
        check_rect(image_rect(screenshot(environment, artifacts, "second-output-rotation")), (1190, 220, 1310, 460))
        pointer.move((1250, 340))
        pointer.click()
        same_source(environment, source.transpose(Image.Transpose.ROTATE_270))

        # 3. 【贴图回归】【滚轮锚点】在非中心位置缩放，完整图像的锚点不得移动
        pointer.move((1220, 280))
        pointer.scroll(dy=-30)
        time.sleep(0.2)
        zoomed = image_rect(screenshot(environment, artifacts, "second-output-wheel-anchor"))
        expected_width, expected_height = round(120 * 1.08 ** 2), round(240 * 1.08 ** 2)
        left, top = 1220 - round(expected_width / 4), 280 - round(expected_height / 4)
        check_rect(zoomed, (left, top, left + expected_width, top + expected_height))
        same_source(environment, source.transpose(Image.Transpose.ROTATE_270))

        # 4. 【贴图回归】【跨屏返回】回到第一输出后仍能完成原图复制，预览表面和工作线程均已释放
        center = ((zoomed[0] + zoomed[2]) / 2, (zoomed[1] + zoomed[3]) / 2)
        drag(pointer, center, (500, 340))
        pointer.release()
        time.sleep(0.2)
        returned = image_rect(screenshot(environment, artifacts, "cross-output-returned"))
        if returned[0] >= 960 or returned[2] - returned[0] != zoomed[2] - zoomed[0]:
            raise AssertionError("returning across outputs changed logical image size")
        pointer.move((500, 340))
        pointer.click()
        same_source(environment, source.transpose(Image.Transpose.ROTATE_270))

        # 5. 【贴图回归】【普通窗口】关闭置顶后使用 xdg 系统缩放，客户端继续约束图片比例
        pointer.click(button=0x111)
        for _ in range(6):
            key(environment, "Down")
        key(environment, "Return")
        time.sleep(0.25)
        ordinary = image_rect(screenshot(environment, artifacts, "ordinary-window"))
        width, height = ordinary[2] - ordinary[0], ordinary[3] - ordinary[1]
        drag(pointer, (ordinary[2] - 2, ordinary[3] - 2),
             (ordinary[2] - 2 + width / 2, ordinary[3] - 2 + height / 2))
        pointer.release()
        time.sleep(0.2)
        resized = image_rect(screenshot(environment, artifacts, "ordinary-native-resized"))
        check_rect(resized, (ordinary[0], ordinary[1], ordinary[0] + round(width * 1.5),
                             ordinary[1] + round(height * 1.5)), tolerance=3)
        pointer.move(((resized[0] + resized[2]) / 2, (resized[1] + resized[3]) / 2))
        pointer.click()
        same_source(environment, source.transpose(Image.Transpose.ROTATE_270))
        time.sleep(0.4)
        center = ((resized[0] + resized[2]) / 2, (resized[1] + resized[3]) / 2)
        drag(pointer, center, (center[0] + 100, center[1] - 80))
        pointer.release()
        time.sleep(0.2)
        moved = image_rect(screenshot(environment, artifacts, "ordinary-native-moved"))
        check_rect(moved, (resized[0] + 100, resized[1] - 80, resized[2] + 100, resized[3] - 80), tolerance=3)
        pointer.move((center[0] + 100, center[1] - 80))
        pointer.click()
        same_source(environment, source.transpose(Image.Transpose.ROTATE_270))

        before = resources(process.pid)
        time.sleep(1)
        after = resources(process.pid)
        if after["threads"] != 1 or after["cpu_ticks"] != before["cpu_ticks"]:
            raise AssertionError(f"dragging kept idle workers or frame polling: {before} -> {after}")
        key(environment, "Escape")
        process.wait(timeout=3)
        return after


def main():
    """【贴图回归】【验证入口】解析私有工具与新证据目录，执行真实输入并记录固定构建指纹"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--sway", type=Path, default=Path("target/test-tools/sway/usr/bin/sway"))
    parser.add_argument("--artifacts", type=Path, default=Path("target/wayland-pin-geometry"))
    options = parser.parse_args()
    artifacts = options.artifacts.resolve()
    artifacts.mkdir(parents=True, exist_ok=False)
    binary = options.binary.resolve()
    source = Image.new("RGBA", (240, 120), (210, 55, 65, 255))
    ImageDraw.Draw(source).rectangle((120, 0, 239, 119), fill=(50, 190, 80, 255))
    ImageDraw.Draw(source).rectangle((30, 30, 45, 45), fill=(60, 90, 120, 128))
    source_path = artifacts / "source.png"
    source.save(source_path)
    config = artifacts / "config.toml"
    config.write_text('[history]\nenabled = false\n[notifications]\nenabled = false\n[log]\nlevel = "info"\n')
    with HeadlessRuntime(artifacts / "runtime", options.sway) as environment, private_environment(environment):
        edges = edge_checks(binary, source_path, source, config, environment, artifacts)
        idle = clipping_and_cross_output(binary, source_path, source, config, environment, artifacts)
        models = Path(environment["XDG_DATA_HOME"]) / "mark-shot/models"
        if models.exists():
            raise AssertionError("ordinary pin geometry initialized OCR models")
        result = {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "edges": edges,
                  "held_preview_and_release": True, "clipping_preserves_scale": True,
                  "embedded_edge_recovers": True, "continuous_cross_output": True,
                  "cross_output_menu_rotation": True, "wheel_cursor_anchor": True,
                  "ordinary_native_resize": True,
                  "ordinary_native_move": True,
                  "source_pixels": True, "ordinary_geometry_has_no_ocr": True, "idle": idle}
        (artifacts / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print("Wayland eight-edge resize, clipping, continuous cross-output and source checks passed")


if __name__ == "__main__":
    main()
