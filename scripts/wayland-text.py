#!/usr/bin/env python3
"""【文字回归】【真实输入】在私有 Wayland 会话验证文字粘贴、独立背景透明度与原图导出"""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import tomllib

from PIL import Image, ImageChops
import wayland_test_session as session
from wayland_text_checks import transparent_text_is_absent


SOURCE = (30, 34, 38, 255)
TEXT = "Mark Shot\n文字标注"


def capture(process, artifacts, name):
    """【文字回归】【截图证据】等待并保存 process 的 name 截图到 artifacts；返回逻辑 RGB 图，提前退出抛出异常"""
    time.sleep(0.18)
    if process.poll() is not None:
        raise AssertionError(f"the text editor exited unexpectedly: {process.returncode}")
    path = artifacts / f"{name}.png"
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def scenario(binary, artifacts, alpha):
    """【文字回归】【单场景】用 binary 创建 alpha 透明度文字，artifacts 保存输入与导出证据；返回像素统计和资源数据"""
    artifacts.mkdir(parents=True, exist_ok=True)
    environment = session.environment()
    with tempfile.TemporaryDirectory(prefix="mark-shot-text-") as directory:
        root = Path(directory)
        config, state = root / "config/mark-shot", root / "state/mark-shot"
        config.mkdir(parents=True)
        state.mkdir(parents=True)
        (config / "config.toml").write_text('[general]\nto = "s"\n[notifications]\nenabled = false\n')
        (state / "tools.toml").write_text('magnifier = false\nfont_size = 20.0\nfont_size_unit = "pt"\ntext_family = "Sans Serif"\ntext_background = "#10203060"\nbold = true\nitalic = false\noutline = false\n')
        environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
        source = root / "source.png"
        Image.new("RGBA", (1600, 900), SOURCE).save(source)
        with (artifacts / "process.log").open("w") as log:
            process = subprocess.Popen([str(binary), "--fullscreen", "--fullscreen-default-tool", "text",
                                        "--default-color", f"#FF4D4D{alpha:02X}", "--to", "s",
                                        "--output", str(root / "export"), str(source)], env=environment, stderr=log)
            try:
                deadline = time.monotonic() + 8
                while True:
                    initial = capture(process, artifacts, "initial")
                    if initial.getpixel((initial.width // 2, initial.height // 2)) == SOURCE[:3]:
                        break
                    if time.monotonic() >= deadline:
                        raise AssertionError("the source image did not appear")
                session.wait_single_thread(process.pid)
                point = (round(initial.width * 0.58), round(initial.height * 0.52))
                session.control("click", "--logical", *point)
                session.paste_text(TEXT)
                capture(process, artifacts, "pasted")

                # 1. 【文字回归】【换行职责】Enter 插入新行而不调用全局导出，删除新行后正文恢复
                session.key("Return")
                capture(process, artifacts, "newline")
                session.key("BackSpace")
                session.key("ctrl+a")
                session.key("ctrl+c")
                session.wait_clipboard(environment, TEXT)
                capture(process, artifacts, "selected-text")

                # 2. 【文字回归】【原图输出】点击外部结束现有编辑，导出只包含正文与背景，不含光标或文字选择
                session.control("click", "--logical", point[0] - 100, point[1] + 140)
                capture(process, artifacts, "committed")
                idle_before = session.wait_single_thread(process.pid)
                time.sleep(1)
                idle_after = session.resources(process.pid)
                session.key("Return")
                process.wait(timeout=6)
                if process.returncode:
                    raise AssertionError(f"text export failed: {process.returncode}")
                files = list((root / "export").glob("*.png"))
                if len(files) != 1:
                    raise AssertionError(f"the original image export is ambiguous: {files}")
                shutil.copyfile(files[0], artifacts / "export.png")
                shutil.copyfile(state / "tools.toml", artifacts / "tools.toml")
                with Image.open(files[0]) as image:
                    result = image.convert("RGBA")
                if result.size != (1600, 900):
                    raise AssertionError(f"the text export was downsampled: {result.size}")
                background = Image.new("RGB", result.size, SOURCE[:3])
                rgb = result.convert("RGB")
                bounds = ImageChops.difference(background, rgb).getbbox()
                if not bounds:
                    raise AssertionError("text and its independent background were not exported")
                left, top, right, bottom = bounds
                if not (148 <= right - left <= 149 and 82 <= bottom - top <= 83):
                    raise AssertionError(f"20 pt text did not have the original 148x82 box: {bounds}")
                sample = rgb.getpixel((left + 2, (top + bottom) // 2))
                if max(abs(first - second) for first, second in zip(sample, (25, 33, 42))) > 1:
                    raise AssertionError(f"background alpha did not remain independent: {sample}")
                peak_red = max(pixel[0] for pixel in rgb.crop(bounds).get_flattened_data())
                if alpha == 0 and not transparent_text_is_absent(rgb, bounds, SOURCE[:3]):
                    raise AssertionError(f"fully transparent text leaked glyph pixels: {peak_red}")
                if alpha == 128 and not (139 <= peak_red <= 142):
                    raise AssertionError(f"glyph opacity was missing or multiplied twice: {peak_red}")
                if result.getchannel("A").getextrema() != (255, 255):
                    raise AssertionError("export damaged the original opaque image alpha")
                if (root / "data/mark-shot/models").exists():
                    raise AssertionError("ordinary text annotation created OCR models")
                saved = tomllib.loads((state / "tools.toml").read_text())
                if saved["font_size_unit"] != "pt" or saved["font_size"] != 20:
                    raise AssertionError(f"the original point-size default was not saved: {saved}")
                if saved["text_background"] != "#10203060":
                    raise AssertionError("text opacity overwrote the background default")
                if idle_after["threads"] != 1 or idle_after["cpu_ticks"] != idle_before["cpu_ticks"]:
                    raise AssertionError(f"idle text editing was not event driven: {idle_before}, {idle_after}")
                return {"alpha": alpha, "text": TEXT, "copy_exact": True, "enter_inserts_newline": True,
                        "export_size": list(result.size), "pixel_bounds": list(bounds),
                        "background_sample": list(sample), "peak_red": peak_red,
                        "idle_before": idle_before, "idle_after": idle_after, "ocr_models_created": False}
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=3)


def main():
    """【文字回归】【命令入口】读取 binary 与 artifacts 参数，执行透明与半透明两场景；原始失败证据保留，成功时记录构建指纹"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, required=True)
    options = parser.parse_args()
    binary, artifacts = options.binary.resolve(), options.artifacts.resolve()
    result = {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "scenarios": [scenario(binary, artifacts / f"alpha-{alpha}", alpha) for alpha in (0, 128)]}
    if result["scenarios"][0]["pixel_bounds"] != result["scenarios"][1]["pixel_bounds"]:
        raise AssertionError("changing text opacity changed the independent background geometry")
    (artifacts / "result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print("Wayland text paste, newline, Unicode copy, independent alpha and original-resolution export checks passed")


if __name__ == "__main__":
    main()
