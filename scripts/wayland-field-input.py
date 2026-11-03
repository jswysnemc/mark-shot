#!/usr/bin/env python3
"""【Wayland 测试】【字段回归】验证属性笔宽的真实剪贴板粘贴、撤销、重做与默认值保存"""

import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import tomllib

from PIL import Image, ImageChops

from wayland_editor_locator import marker_controls
import wayland_test_session as session


BORDER = (96, 113, 127)


def capture(path):
    """【字段测试】【界面截图】把隔离桌面写入 path；返回可比较的 RGB 图像"""
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def ready(process, path):
    """【字段测试】【启动等待】等待 process 绘制源图，证据保存到 path；返回首帧图像，失败时抛出异常"""
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(f"the editor exited before its first frame: {process.returncode}")
        image = capture(path)
        if image.getpixel((image.width // 2, image.height // 2)) == (30, 34, 38):
            time.sleep(0.25)
            return capture(path)
        time.sleep(0.04)
    raise RuntimeError("the numeric field test image did not appear")


def run(binary, artifacts):
    """【字段测试】【完整回归】使用 binary 检查笔宽编辑与默认值保存；将截图、状态和日志写入 artifacts"""
    environment = session.environment()
    artifacts.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="mark-shot-field-test-") as directory:
        temporary = Path(directory)
        config = temporary / "config/mark-shot"
        state = temporary / "state/mark-shot"
        config.mkdir(parents=True)
        state.mkdir(parents=True)
        (config / "config.toml").write_text('[general]\nto = "s"\n[notifications]\nenabled = false\n[theme]\nborder = "#60717FFF"\nborder_on_light = "#60717FFF"\n')
        (state / "tools.toml").write_text('magnifier = false\nmarker_shape = "square"\nmarker_filled = false\n')
        environment.update(XDG_CONFIG_HOME=str(temporary / "config"), XDG_DATA_HOME=str(temporary / "data"), XDG_STATE_HOME=str(temporary / "state"))
        source_path = temporary / "source.png"
        Image.new("RGBA", (1600, 900), (30, 34, 38, 255)).save(source_path)
        with (artifacts / "process.log").open("w") as log:
            process = subprocess.Popen([str(binary), "--fullscreen", "--fullscreen-default-tool", "marker", "--default-color", "#FF3B30", "--to", "s", "--output", str(temporary / "export"), str(source_path)], env=environment, stderr=log)
            try:
                image = ready(process, artifacts / "baseline.png")
                start = (round(image.width * 0.7), round(image.height * 0.65))
                end = (start[0] + 70, start[1] + 55)
                region = (start[0] - 12, start[1] - 12, end[0] + 12, end[1] + 12)
                defaults = tomllib.loads((Path(__file__).resolve().parents[1] / "src/config/default.toml").read_text())

                # 1. 绘制并选中空心方形，输入经过真实属性面板与应用剪贴板实现
                session.control("drag", "--from", *start, "--to", *end, "--steps", 4)
                session.key(defaults["keys"]["tool_pick"])
                session.control("click", "--logical", round((start[0] + end[0]) / 2), start[1] + 1)
                selected = capture(artifacts / "selected.png")
                controls = marker_controls(selected, BORDER)
                session.control("click", "--logical", *controls["width"], "--count", 2)
                session.key("ctrl+a")
                session.paste_text("9")
                capture(artifacts / "pasted.png")
                session.key("Return")
                if process.poll() is not None:
                    raise RuntimeError("confirming a numeric field triggered global export")
                modified = capture(artifacts / "committed.png")
                if ImageChops.difference(selected.crop(region), modified.crop(region)).getbbox() is None:
                    raise RuntimeError("pasting a stroke width did not change the selected annotation")

                # 2. 一次确认形成一个属性历史项，撤销与重做恢复准确笔宽
                session.key("ctrl+z")
                restored = capture(artifacts / "undo.png")
                if ImageChops.difference(selected.crop(region), restored.crop(region)).getbbox():
                    raise RuntimeError("one undo did not restore the original stroke width")
                session.key("ctrl+shift+z")
                redone = capture(artifacts / "redo.png")
                if ImageChops.difference(modified.crop(region), redone.crop(region)).getbbox():
                    raise RuntimeError("one redo did not restore the pasted stroke width")

                # 3. 全局导出后验证独立笔宽默认值，普通编辑不能创建识别模型目录
                session.key("Return")
                process.wait(timeout=5)
                if process.returncode:
                    raise RuntimeError(f"export failed with status {process.returncode}")
                saved = tomllib.loads((state / "tools.toml").read_text())
                if saved["stroke_widths"]["marker"] != 9:
                    raise RuntimeError(f"the pasted stroke width did not persist: {saved}")
                if (temporary / "data/mark-shot/models").exists():
                    raise RuntimeError("numeric field editing created the OCR model directory")
                shutil.copyfile(state / "tools.toml", artifacts / "tools.toml")
                (artifacts / "result.json").write_text(json.dumps({"clipboard_input": "9", "saved_width": 9, "undo": True, "redo": True}, indent=2) + "\n")
                print("Wayland numeric field paste, commit, undo, redo and persistence checks passed")
            finally:
                if process.poll() is None:
                    session.key("Escape")
                    try:
                        process.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        process.terminate()
                        process.wait(timeout=3)


def main():
    """【字段测试】【入口】解析构建产物及证据目录，执行回归；失败时返回非零退出码"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, default=Path("target/wayland-field-input"))
    options = parser.parse_args()
    run(options.binary.resolve(), options.artifacts.resolve())


if __name__ == "__main__":
    main()
