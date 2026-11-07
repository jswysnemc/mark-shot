#!/usr/bin/env python3
"""【画布初始布局回归】【真实会话】验证图片入口等比放大、原始像素与透明度导出、空闲状态，不连接用户桌面"""

import argparse
import hashlib
import importlib.util
import io
import json
import subprocess
from pathlib import Path

from PIL import Image

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("canvas_layout_support", PROJECT / "scripts/wayland-annotation-collection.py")
GROUP = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GROUP)
FLOW, INPUT, session = GROUP.FLOW, GROUP.INPUT, GROUP.session


def original(root, alpha):
    """【画布初始布局回归】【原图构造】按alpha创建root下小型渐变图片并标记四角，返回路径和完整RGBA像素"""
    image = Image.new("RGBA", (384, 240))
    image.putdata([(40 + x % 90, 70 + y % 70, 220, alpha) for y in range(240) for x in range(384)])
    for point, color in zip([(0, 0), (383, 0), (0, 239), (383, 239)], [(255, 0, 0, alpha), (0, 255, 0, alpha), (0, 0, 255, alpha), (255, 255, 255, alpha)]):
        image.putpixel(point, color)
    path = root / "original.png"
    image.save(path)
    return path, image


def premultiplied(image):
    """【画布初始布局回归】【像素等价】按image返回预乘RGBA字节，用于检查现有原图存储语义，不忽略透明度或角点"""
    rgba = image.convert("RGBA").tobytes()
    result = bytearray(len(rgba))
    for index in range(0, len(rgba), 4):
        alpha = rgba[index + 3]
        for channel in range(3):
            result[index + channel] = (rgba[index + channel] * alpha + 127) // 255
        result[index + 3] = alpha
    return bytes(result)


def verify(binary, environment, root, alpha):
    """【画布初始布局回归】【图片流程】在environment运行binary打开alpha原图，root保存实际预览和导出，返回像素及资源证据"""
    root.mkdir()
    environment = dict(environment, XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    config = root / "config.toml"
    config.write_text('[general]\noverlay="window"\nto=""\n[notifications]\nenabled=false\n[history]\nenabled=false\n')
    path, image = original(root, alpha)
    with INPUT.launch(binary, environment, config, path, root, "image") as (process, log):
        FLOW.wait_input(process, log, (200, 200))
        session.key("Space")
        INPUT.move((650, 480))
        shown = FLOW.snapshot(root, "fitted-image")
        probes = [(round(shown.width * 0.12), round(shown.height * 0.3)), (round(shown.width * 0.86), round(shown.height * 0.5)), (round(shown.width * 0.14), round(shown.height * 0.73))]
        # 1. 【画布初始布局回归】【全窗适配】三处采样均位于旧居中小图之外，新的蓝色原图必须实际覆盖这些像素
        colors = [shown.getpixel(point) for point in probes]
        assert all(color[2] > color[0] + 30 and color[2] > color[1] + 30 for color in colors), f"small source image did not fill the Qt fitted viewport: {colors}"
        idle = FLOW.NAV.SIZE.idle(process, log)
        # 2. 【画布初始布局回归】【原图交付】通过生产复制入口导出，保留尺寸、四角和全部预乘像素，不以预览图替代原图
        session.key("ctrl+c")
        assert process.wait(timeout=5) == 0
        data = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment, timeout=3)
        (root / "export.png").write_bytes(data)
        exported = Image.open(io.BytesIO(data)).convert("RGBA")
        assert exported.size == image.size, f"image export changed source dimensions: {exported.size}"
        assert premultiplied(exported) == premultiplied(image), "image export changed original premultiplied pixels or alpha"
    result = {"alpha": alpha, "image_size": list(image.size), "preview_size": list(shown.size), "probes": probes, "colors": colors, "fit_visible": True, "premultiplied_pixels_equal": True, "export_sha256": hashlib.sha256(data).hexdigest(), "idle": idle}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def run(binary, root):
    """【画布初始布局回归】【私有矩阵】在已校验会话测试binary两种透明度，root保存独立证据，返回不可变指纹和结果"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(TMPDIR=str(root / "temporary"), MARK_SHOT_LANG="english", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/canvas-layout-test")
    Path(environment["TMPDIR"]).mkdir()
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    cases = []
    with session.input_context(), FLOW.NAV.SIZE.pointer_context(backend="x11" if session.backend() is session.niri else "virtual"):
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        for alpha in (255, 170):
            value = verify(binary, environment, root / f"alpha-{alpha}", alpha)
            cases.append(value)
            print(json.dumps(value), flush=True)
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == fingerprint
    result = {"binary_sha256": fingerprint, "compositor": session.backend().__name__, "cases": cases, "scope": "Initial fitted image layout and original image export only; interactive canvas navigation is not connected yet."}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    """【画布初始布局回归】【入口】解析不可变binary和新root，输出实际结果，失败保留现场，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
