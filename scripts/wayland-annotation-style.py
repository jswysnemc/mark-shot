#!/usr/bin/env python3
"""【标注样式回归】【原生输入】仅在校验后的私有Wayland创建线条，核对窗口笔宽、原图导出和空闲状态"""

import argparse
import hashlib
import importlib.util
import io
import json
import subprocess
import time
from pathlib import Path

from PIL import Image

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("style_native_navigation", PROJECT / "scripts/wayland-canvas-rotation.py")
SUPPORT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SUPPORT)
NAVIGATION = SUPPORT.NAVIGATION
FLOW, INPUT, TOOLS, session = SUPPORT.FLOW, SUPPORT.INPUT, SUPPORT.TOOLS, SUPPORT.session


def width(image, x, start, end):
    """【标注样式回归】【像素笔宽】按image、x和纵向start至end返回黑色笔迹的覆盖积分，不按日志推算实际宽度"""
    return sum(1.0 - image.getpixel((x, y))[0] / 255.0 for y in range(start, end))


def verify(binary, environment, root, size, raw_width):
    """【标注样式回归】【真实流程】按binary、environment、root、size与raw_width创建并导出线条，返回实际测量和空闲结果"""
    root.mkdir()
    environment = dict(environment, XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    config = root / "config.toml"
    config.write_text('[general]\noverlay="window"\nto=""\n[notifications]\nenabled=false\n[history]\nenabled=false\n[tools]\ncolor="#000000"\n[tools.line]\nwidth=' + str(raw_width) + '\n')
    original = Image.new("RGBA", size, (255, 255, 255, 255))
    path = root / "original.png"
    original.save(path)
    with INPUT.launch(binary, environment, config, path, root, "line") as (process, log):
        FLOW.wait_input(process, log, (200, 200))
        session.key("Space")
        baseline = NAVIGATION.snapshot(root, "initial")
        INPUT.move((320, 240))
        NAVIGATION.wheel_up(3)
        factor = 1.12 ** SUPPORT.received_steps(log)
        session.key("d")
        INPUT.drag((220, 240), (600, 240))
        INPUT.move(SUPPORT.PARK)
        fitted = min(baseline.width / size[0], baseline.height / size[1])
        expected = max(1.5, raw_width * fitted * factor)
        presented = TOOLS.presented(root, "line", lambda image: width(image, 400, 210, 270) >= expected * 0.8, "line stroke did not reach the private compositor")
        measured = width(presented, 400, 210, 270)
        # 1. 【标注样式回归】【逻辑截图】截图把一点五倍设备像素重新采样为逻辑像素，覆盖积分允许四分之一窗口像素误差
        assert abs(measured - expected) <= 0.25, f"window width {measured} != {expected}; original={size} raw={raw_width} zoom={factor}"
        time.sleep(1.05)
        idle = FLOW.NAV.SIZE.idle(process, log)
        session.key("ctrl+c")
        assert process.wait(timeout=5) == 0
        data = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment, timeout=3)
        (root / "export.png").write_bytes(data)
        exported = Image.open(io.BytesIO(data)).convert("RGBA")
        assert exported.size == size
        exported_width = width(exported, size[0] // 2, 0, size[1])
        expected_export = max(1.5, raw_width)
        assert abs(exported_width - expected_export) <= 0.08, f"original width {exported_width} != {expected_export}"
    result = {"size": size, "raw_width": raw_width, "actual_zoom": factor, "measured_window_width": measured, "expected_window_width": expected, "exported_image_width": exported_width, "export_sha256": hashlib.sha256(data).hexdigest(), "idle": idle}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def run(binary, root):
    """【标注样式回归】【私有矩阵】在校验后的私有会话以binary执行四种创建和导出组合，root保留现场，返回候选指纹"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(TMPDIR=str(root / "temporary"), MARK_SHOT_LANG="english", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/annotation-style-test")
    Path(environment["TMPDIR"]).mkdir()
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    cases = []
    with session.input_context(), FLOW.NAV.SIZE.pointer_context(backend="x11" if session.backend() is session.niri else "virtual"):
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        for size in ((384, 240), (1280, 960)):
            for raw_width in (1, 6):
                case = verify(binary, environment, root / f"image-{size[0]}-width-{raw_width}", size, raw_width)
                cases.append(case)
                print(json.dumps(case), flush=True)
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == fingerprint
    result = {"binary_sha256": fingerprint, "compositor": session.backend().__name__, "cases": cases, "scope": "Real native line creation after image zoom for small and large files, measured window minimum, original width export and idle samples. Other tools, full Qt pixel parity and cold startup are not covered by this script."}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    """【标注样式回归】【入口】解析binary和新root并执行独立矩阵，无返回值，失败保留全部现场"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
