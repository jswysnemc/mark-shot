#!/usr/bin/env python3
"""【Wayland 测试】【激光回归】验证消散、工具切换、空闲占用和图片导出，不操作真实桌面"""

import argparse
import io
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

from PIL import Image, ImageChops, ImageDraw

import wayland_test_session as session


def capture(path):
    """【激光测试】【界面截图】将隔离桌面保存到 path；返回 RGB 像素图像"""
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def changed(original, current, region):
    """【激光测试】【区域差异】比较 original、current 在 region 内的像素；返回非空差异区域或空值"""
    return ImageChops.difference(original.crop(region), current.crop(region)).getbbox()


def difference_energy(original, current, region):
    """【激光测试】【消散强度】计算 original、current 在 region 内的 RGB 差异总量；返回非负整数"""
    histogram = ImageChops.difference(original.crop(region), current.crop(region)).histogram()
    return sum(index % 256 * count for index, count in enumerate(histogram))


def drag(start, end, steps=12):
    """【激光测试】【拖动手势】在子会话内从 start 拖动到 end；steps 指定输入采样次数，无返回值"""
    session.control("drag", "--from", *start, "--to", *end, "--steps", steps)


def wait_ready(process, output):
    """【激光测试】【启动等待】等待 process 显示源图并截图到 output；返回界面像素，超时抛出异常"""
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError("the editor exited before showing the source image")
        current = capture(output)
        if current.getpixel((current.width // 2, current.height // 2)) == (30, 34, 38):
            time.sleep(0.25)
            return capture(output)
        time.sleep(0.05)
    raise RuntimeError("the source image did not appear")


def check_pixels(encoded, expected, label):
    """【激光测试】【导出校验】解码 encoded 并逐像素比较 expected；label 标识输出渠道，无返回值"""
    with Image.open(io.BytesIO(encoded)) as image:
        image = image.convert("RGBA")
        if image.size != expected.size or image.tobytes() != expected.tobytes():
            raise RuntimeError(f"{label} includes a transient trace or changed source pixels")


def cancel_draft(process, original, start, end, region, artifacts):
    """【激光测试】【切换取消】在 process 的长手势中切换画笔；验证 region 恢复 original，不留下完成笔迹"""
    arguments = ["niri-use", "--child", "drag", "--from", *map(str, start), "--to", *map(str, end), "--steps", "180"]
    with subprocess.Popen(arguments, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE) as gesture:
        deadline = time.monotonic() + 6
        observed = False
        while time.monotonic() < deadline and gesture.poll() is None:
            current = capture(artifacts / "draft.png")
            if changed(original, current, region):
                observed = True
                break
            time.sleep(0.02)
        if not observed or gesture.poll() is not None:
            gesture.wait(timeout=10)
            raise RuntimeError("the long gesture ended before its draft could be observed")
        session.key("P")
        canceled = capture(artifacts / "draft-canceled.png")
        if changed(original, canceled, region):
            gesture.wait(timeout=10)
            raise RuntimeError("switching tools did not clear the uncommitted laser draft")
        gesture.wait(timeout=10)
        if gesture.returncode:
            raise RuntimeError(gesture.stderr.read().decode())
    if process.poll() is not None:
        raise RuntimeError("switching tools unexpectedly closed the editor")
    session.key("G")


def run(binary, artifacts):
    """【激光测试】【完整回归】使用 binary 在隔离会话验证临时笔迹；将图像、日志和资源结果写入 artifacts"""
    environment = session.environment()
    artifacts.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="mark-shot-laser-test-") as directory:
        temporary = Path(directory)
        config = temporary / "config/mark-shot"
        state = temporary / "state/mark-shot"
        config.mkdir(parents=True)
        state.mkdir(parents=True)
        (config / "config.toml").write_text('[general]\nto = "cs"\n[notifications]\nenabled = false\n[tools.laser]\nwidth = 10.0\n')
        (state / "tools.toml").write_text("magnifier = false\n")
        environment.update(XDG_CONFIG_HOME=str(temporary / "config"), XDG_DATA_HOME=str(temporary / "data"), XDG_STATE_HOME=str(temporary / "state"))

        # 1. 保留源图透明边缘，验证屏幕预览缩放不会改变最终 PNG
        source = Image.new("RGBA", (1600, 900), (30, 34, 38, 255))
        painter = ImageDraw.Draw(source)
        painter.rectangle((0, 0, 1599, 7), fill=(0, 0, 0, 0))
        painter.rectangle((20, 20, 42, 42), fill=(255, 0, 0, 128))
        source_path = temporary / "source.png"
        source.save(source_path)
        export_directory = temporary / "export"
        exported = artifacts / "saved-image.png"
        with (artifacts / "process.log").open("w") as stderr:
            process = subprocess.Popen([str(binary), "--fullscreen", "--fullscreen-default-tool", "laser", "--default-color", "#FF3B30", "--to", "cs", "--output", str(export_directory), str(source_path)], env=environment, stderr=stderr)
            try:
                baseline = wait_ready(process, artifacts / "baseline.png")
                scale = min((baseline.width - 64) / source.width, (baseline.height - 96) / source.height, 1)
                width, height = source.width * scale, source.height * scale
                left, top = (baseline.width - width) / 2, (baseline.height - height) / 2
                start = (round(left + width * 0.24), round(top + height * 0.42))
                end = (round(left + width * 0.67), round(top + height * 0.48))
                region = (start[0] - 24, start[1] - 24, end[0] + 24, end[1] + 24)

                # 2. 完成笔迹在工具切换后继续消散，全部消失后不再占用刷新定时器
                drag(start, end)
                release = time.monotonic()
                active = capture(artifacts / "active.png")
                if not changed(baseline, active, region):
                    raise RuntimeError("laser dragging did not produce a visible trace")
                session.key("P")
                switched = capture(artifacts / "switched.png")
                if not changed(baseline, switched, region):
                    raise RuntimeError("switching tools prematurely removed a committed trace")
                time.sleep(max(0, release + 0.95 - time.monotonic()))
                fading = capture(artifacts / "fading.png")
                initial_energy = difference_energy(baseline, active, region)
                fading_energy = difference_energy(baseline, fading, region)
                if not 0 < fading_energy < initial_energy:
                    raise RuntimeError("the committed trace does not fade gradually")
                time.sleep(max(0, release + 2.15 - time.monotonic()))
                expired = capture(artifacts / "expired.png")
                if changed(baseline, expired, region):
                    raise RuntimeError("the expired trace left a glow or an unrefreshed area")
                before = session.resources(process.pid)
                time.sleep(1)
                after = session.resources(process.pid)
                if after["cpu_ticks"] != before["cpu_ticks"]:
                    raise RuntimeError(f"the idle editor still consumes CPU ticks: {before} -> {after}")
                (artifacts / "resources.json").write_text(json.dumps({"before": before, "after": after}, indent=2) + "\n")
                session.key("G")
                time.sleep(0.15)
                baseline = capture(artifacts / "before-cancel.png")
                cancel_draft(process, baseline, start, end, region, artifacts)

                # 3. 激光不进入撤销记录、图片保存或图片剪贴板，即时导出保留全部源像素
                time.sleep(0.15)
                drag(start, end)
                session.key("ctrl+z")
                unrecorded = capture(artifacts / "after-undo.png")
                if not changed(baseline, unrecorded, region):
                    raise RuntimeError("undo incorrectly removed a transient laser trace")
                session.key("Return")
                process.wait(timeout=5)
                if process.returncode:
                    raise RuntimeError(f"the editor exited with status {process.returncode}")
                saved_images = list(export_directory.rglob("*.png"))
                if len(saved_images) != 1:
                    raise RuntimeError(f"expected one saved PNG, received {len(saved_images)}")
                shutil.copyfile(saved_images[0], exported)
                check_pixels(exported.read_bytes(), source, "saved PNG")
                offered = subprocess.check_output(["wl-paste", "--list-types"], env=environment, text=True)
                if "image/png" not in offered:
                    raise RuntimeError(f"the image clipboard type is missing: {offered}")
                clipboard = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment)
                (artifacts / "clipboard.png").write_bytes(clipboard)
                check_pixels(clipboard, source, "clipboard PNG")
                if (temporary / "data/mark-shot/models").exists():
                    raise RuntimeError("ordinary annotation initialized the OCR model directory")
                print("Wayland laser fade, cancellation, idle and export checks passed")
            finally:
                log = state / "mark-shot.log"
                if log.exists():
                    shutil.copyfile(log, artifacts / "application.log")
                if process.poll() is None:
                    session.key("Escape")
                    try:
                        process.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        process.terminate()
                        process.wait(timeout=3)


def main():
    """【激光测试】【入口】解析构建产物与证据目录，执行回归；失败时返回非零退出码"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, default=Path("target/wayland-laser"))
    options = parser.parse_args()
    run(options.binary.resolve(), options.artifacts.resolve())


if __name__ == "__main__":
    main()
