#!/usr/bin/env python3
"""【文字存储回归】【真实会话】私有Wayland验证连续缩放、隐藏高度历史、编辑收框与原图导出"""

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path

from PIL import ImageChops

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("text_storage_rotation_support", PROJECT / "scripts/wayland-annotation-rotation.py")
ROTATION = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ROTATION)
FLOW, INPUT, session = ROTATION.FLOW, ROTATION.INPUT, ROTATION.session
REGION = ROTATION.REGION


def controls(image):
    """【文字存储回归】【控制点坐标】按image实际方块边缘和旋转圆心返回逻辑控制点，像素中心加半像素还原输入坐标"""
    pixels = FLOW.pixels(image, REGION, ROTATION.ORANGE)
    assert len(pixels) > 150, "text has no complete orange frame"
    minimum_y = min(y for _, y in pixels)
    circle = [(x, y) for x, y in pixels if y < minimum_y + 10]
    rotation = (sum(x for x, _ in circle) / len(circle) + 0.5, sum(y for _, y in circle) / len(circle) + 0.5)
    bounds = (min(x for x, _ in pixels) + 4, rotation[1] + 26,
              max(x for x, _ in pixels) - 3, max(y for _, y in pixels) - 3)
    center = ((bounds[0] + bounds[2]) / 2, (bounds[1] + bounds[3]) / 2)
    return center, rotation, bounds


def dimensions(image):
    """【文字存储回归】【可见测量】按image实际橙色控制点返回选框宽高，不读取标注内部状态"""
    _, _, bounds = controls(image)
    return bounds[2] - bounds[0], bounds[3] - bounds[1]


def resize(image, edge, delta):
    """【文字存储回归】【真实拖动】按image实际选框的edge拖动delta，返回值为空，不直接设置标注宽高"""
    center, _, bounds = controls(image)
    if edge == "right":
        start = (bounds[2], center[1])
        end = (bounds[2] + delta, center[1])
    else:
        start = (center[0], bounds[3])
        end = (center[0], bounds[3] + delta)
    INPUT.drag(start, end)
    INPUT.move(ROTATION.PARK)


def same(root, name, expected):
    """【文字存储回归】【历史恢复】等待root/name与expected正文及控制点逐像素一致，返回真实截图，超时失败"""
    return ROTATION.same(root, name, expected)


def verify(binary, environment, root, scenario):
    """【文字存储回归】【单项流程】在environment用binary验证scenario，root保存截图、日志和PNG，返回验证记录"""
    root.mkdir()
    environment = dict(environment, XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    config = root / "config.toml"
    config.write_text('[general]\noverlay="window"\nto=""\n[notifications]\nenabled=false\n[history]\nenabled=false\n[tools]\ncolor="#334155"\n[tools.font]\nsize=20.0\nbold=true\n')
    FLOW.empty_desktop(root, REGION)
    result = {"scenario": scenario}
    with INPUT.launch(binary, environment, config, None, root, "text") as (process, log):
        FLOW.wait_input(process, log, (80, 80))
        INPUT.drag((80, 80), (600, 380))
        session.key("t")
        INPUT.click((180, 180))
        session.paste_text("Rotation")
        session.key("Escape")
        session.key("v")
        INPUT.click((200, 190))
        original = ROTATION.snapshot(root, "original")
        original_size = dimensions(original)
        assert all(abs(actual - expected) < 1.0 for actual, expected in zip(original_size, (126, 45))), f"unexpected source text size: {original_size}"
        # 1. 【文字存储回归】【连续隐藏尺寸】扩宽和改高不扩大自然框，但每次存储矩形改变均保留独立历史
        repetitions = 1 if scenario == "widen-shrink" else 2
        edge = "bottom" if scenario == "height-twice" else "right"
        current = original
        for step in range(repetitions):
            resize(current, edge, 24)
            current = same(root, f"hidden-{step + 1}", original)
        resize(current, "right", -40)
        narrowed = ROTATION.snapshot(root, "narrowed")
        actual_size = dimensions(narrowed)
        expected_width = {"widen-shrink": 103, "widen-twice": 122, "height-twice": 86}[scenario]
        assert all(abs(actual - expected) < 1.0 for actual, expected in zip(actual_size, (expected_width, 82))), f"{scenario}: visible size {actual_size} differs from production storage mapping"
        # 2. 【文字存储回归】【全部历史】缩窄后撤销每次隐藏操作，第三或第四次才删除最初正文，不允许漏记高度变化
        for step in range(repetitions + 1):
            session.key("ctrl+z")
            INPUT.move(ROTATION.PARK)
            same(root, f"undo-{step + 1}", original)
        session.key("ctrl+z")
        deleted = ROTATION.snapshot(root, "undo-created")
        assert not FLOW.pixels(deleted, REGION, ROTATION.ORANGE), "creation history was not reached after every storage change"
        for _ in range(repetitions + 2):
            session.key("ctrl+y")
        INPUT.move(ROTATION.PARK)
        same(root, "redo-all", narrowed)
        # 3. 【文字存储回归】【旋转提交】先转九十度再开始横向编辑，提交仍保留姿态，撤销恢复旋转前的已换行字形
        center, handle, _ = controls(narrowed)
        with ROTATION.modifier("Shift_L"):
            INPUT.drag(handle, ROTATION.rotated(handle, center, 90))
        quarter = ROTATION.snapshot(root, "quarter-turn")
        session.key("t")
        INPUT.click(center)
        session.key("ctrl+a")
        session.paste_text("Edited")
        session.key("ctrl+Return")
        session.key("v")
        INPUT.click(center)
        edited = ROTATION.snapshot(root, "edited")
        assert ImageChops.difference(quarter.crop(REGION), edited.crop(REGION)).getbbox(), "editing changed no body pixels"
        session.key("ctrl+z")
        INPUT.move(ROTATION.PARK)
        same(root, "undo-edit", quarter)
        session.key("ctrl+y")
        INPUT.move(ROTATION.PARK)
        same(root, "redo-edit", edited)
        session.key("Space")
        result["idle"] = FLOW.NAV.SIZE.idle(process, log)
        session.key("Space")
        result.update(original_size=original_size, narrowed_size=actual_size, hidden_operations=repetitions,
                      exact_hidden_history=True, exact_edit_history=True,
                      export=ROTATION.export(environment, process, root, "text-storage-export"))
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def run(binary, root):
    """【文字存储回归】【矩阵运行】在已校验私有会话验证不可变binary，root为新目录，返回构建指纹及三项真实结果"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(TMPDIR=str(root / "temporary"), MARK_SHOT_LANG="english", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/text-storage")
    Path(environment["TMPDIR"]).mkdir()
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    with session.input_context(), FLOW.NAV.SIZE.pointer_context(backend="x11" if session.backend() is session.niri else "virtual"):
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        cases = [verify(binary, environment, root / name, name) for name in ("widen-shrink", "widen-twice", "height-twice")]
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == fingerprint
    result = {"binary_sha256": fingerprint, "compositor": session.backend().__name__, "cases": cases}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    """【文字存储回归】【入口】解析binary及新root，输出实际验证结果，无返回值，失败保留证据"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
