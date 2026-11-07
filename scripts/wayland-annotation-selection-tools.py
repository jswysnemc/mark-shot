#!/usr/bin/env python3
"""【标注选框回归】【工具矩阵】在私有 Wayland 会话检查实际工具的统一选框、删除与历史恢复"""

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import time

from PIL import ImageChops

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "annotation_selection_workflow", PROJECT / "scripts/wayland-annotation-selection.py"
)
FLOW = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(FLOW)
INPUT, session = FLOW.INPUT, FLOW.session

CASES = [
    ("rectangle", "r", (180, 180), (280, 260), (230, 180), ""),
    ("ellipse", "c", (180, 180), (280, 260), (230, 180), ""),
    ("pen", "p", (180, 180), (280, 260), (230, 220), ""),
    ("highlighter", "h", (180, 180), (280, 260), (230, 220), ""),
    ("marker", "k", (180, 180), (280, 260), (230, 220), ""),
    ("line", "d", (180, 180), (280, 260), (230, 220), ""),
    ("arrow", "a", (180, 180), (280, 260), (230, 220), ""),
    ("numbered-arrow", "shift+n", (180, 180), (280, 260), (230, 220), ""),
    ("filter", "m", (180, 180), (280, 260), (230, 220), ""),
    ("number", "n", (230, 220), None, (230, 220), ""),
    ("magnifier", "z", (220, 220), (400, 220), (220, 220), ""),
    ("length-ruler", "u", (180, 180), (300, 240), (230, 190), ""),
    ("angle-ruler", "u", (180, 180), (300, 260), (180, 180), '[tools.ruler]\nmode="angle"\n'),
    ("text", "t", (180, 180), None, (200, 190), ""),
]


def presented(root, name, predicate, description):
    """【工具选框回归】【实际提交】等待 root/name 画面满足 predicate，返回图像，超时使用 description 报告失败，判据不作放宽"""
    deadline = time.monotonic() + 3.0
    while True:
        image = FLOW.snapshot(root, name)
        if predicate(image):
            return image
        assert time.monotonic() < deadline, description


def verify(binary, environment, root, case):
    """【工具选框回归】【单项验证】用 binary 和 environment 创建 case 标注，root 保存真实图像，返回删除与撤销证据"""
    name, key, start, end, hit, extra = case
    root.mkdir()
    environment = dict(environment, XDG_CONFIG_HOME=str(root / "config"),
                       XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    config = root / "config.toml"
    config.write_text(
        '[general]\noverlay="window"\nto=""\n[notifications]\nenabled=false\n'
        '[history]\nenabled=false\n[tools]\ncolor="#334155"\n' + extra
    )
    region = (100, 80, 630, 360)
    # 1. 【工具选框回归】【原图比例】滤镜、序号和双框工具沿用原图像素尺寸，1.5倍输出的控制点逻辑面积更小
    minimum_orange = 80 if name in {"filter", "number", "magnifier"} else 150
    background = FLOW.empty_desktop(root, region)
    with INPUT.launch(binary, environment, config, None, root, name) as (process, log):
        FLOW.wait_input(process, log, (80, 80))
        INPUT.drag((80, 80), (600, 380))
        session.key(key)
        # 1. 【工具选框回归】【真实创建】所有样本经工具输入入口创建，文字先提交正文，角度尺依次确认三个实际定点
        if name == "text":
            INPUT.click(start)
            session.paste_text("Selection")
            session.key("Escape")
        elif name == "angle-ruler":
            for point in [start, (260, 180), end]:
                INPUT.click(point)
        elif end is None:
            INPUT.click(start)
        else:
            INPUT.drag(start, end)
        session.key("v")
        INPUT.click(hit)
        INPUT.move((620, 400))
        selected = presented(root, "selected", lambda image:
            len(FLOW.pixels(image, region, (251, 146, 60))) > minimum_orange
            and len(FLOW.pixels(image, region, (239, 68, 68))) > 60,
            f"{name}: selected controls did not reach the compositor")
        orange = FLOW.pixels(selected, region, (251, 146, 60))
        red = FLOW.pixels(selected, region, (239, 68, 68))
        assert len(orange) > minimum_orange, f"{name}: no shared orange controls: {len(orange)}"
        assert len(red) > 60, f"{name}: no red delete button: {len(red)}"
        point = (sum(x for x, _ in red) / len(red), sum(y for _, y in red) / len(red))
        INPUT.move(point)
        before = FLOW.snapshot(root, "before-delete")
        # 2. 【工具选框回归】【整项删除】实际按钮只删除所属完整标注，释放后撤销必须恢复原始正文和选中外框
        INPUT.click(point)
        deleted = presented(root, "deleted", lambda image:
            not FLOW.pixels(image, region, (251, 146, 60))
            and not FLOW.pixels(image, region, (239, 68, 68)),
            f"{name}: selected annotation controls remained after deletion")
        assert not FLOW.pixels(deleted, region, (251, 146, 60)), f"{name}: orange controls remain"
        assert not FLOW.pixels(deleted, region, (239, 68, 68)), f"{name}: delete button remains"
        session.key("ctrl+z")
        restored = presented(root, "undo", lambda image:
            ImageChops.difference(before.crop(region), image.crop(region)).getbbox() is None,
            f"{name}: undo changed annotation or selection pixels")
        assert ImageChops.difference(before.crop(region), restored.crop(region)).getbbox() is None, (
            f"{name}: undo changed annotation or selection pixels"
        )
        session.key("Escape")
        assert process.wait(timeout=5) == 0
    result = {"name": name, "background_rgb": background,
              "orange_pixels": len(orange), "delete_pixels": len(red),
              "delete_point": point, "whole_annotation_deleted": True, "exact_undo": True}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def run(binary, root, selected_cases=None):
    """【工具选框回归】【矩阵执行】在当前私有会话检查固定 binary，root 保存隔离配置，selected_cases 可限定工具列表，返回指纹和结果"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(
        XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"),
        XDG_STATE_HOME=str(root / "state"), TMPDIR=str(root / "temporary"),
        MARK_SHOT_LANG="english", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/annotation-selection-tools",
    )
    Path(environment["TMPDIR"]).mkdir()
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    cases = []
    with session.input_context(), FLOW.NAV.SIZE.pointer_context(
        backend="x11" if session.backend() is session.niri else "virtual"
    ):
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        for case in CASES:
            if selected_cases is not None and case[0] not in selected_cases:
                continue
            result = verify(binary, environment, root / case[0], case)
            cases.append(result)
            print(json.dumps(result), flush=True)
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == fingerprint
    result = {"binary_sha256": fingerprint, "compositor": session.backend().__name__, "cases": cases}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    """【工具选框回归】【入口】读取不可变构建和新证据目录，打印矩阵结果，失败保留本次现场，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--case", action="append", choices=[case[0] for case in CASES])
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve(), options.case), indent=2))


if __name__ == "__main__":
    main()
