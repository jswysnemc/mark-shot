#!/usr/bin/env python3
"""【放大镜多选回归】【真实会话】在私有Wayland检查边缘双框、群组改宽、旋转、历史与实际导出"""

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path

from PIL import ImageChops

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("magnifier_collection_support", PROJECT / "scripts/wayland-annotation-collection.py")
GROUP = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GROUP)
ROTATION, TOOLS, FLOW, INPUT, session = GROUP.ROTATION, GROUP.TOOLS, GROUP.FLOW, GROUP.INPUT, GROUP.session
REGION = (5, 75, 600, 395)
GROUP.REGION = REGION


def changed(before, after, description):
    """【放大镜多选回归】【实际变化】按before、after和description检查正文或控制框确实变化，无返回值"""
    assert ImageChops.difference(before.crop(REGION), after.crop(REGION)).getbbox(), description


def verify(binary, environment, root, shape):
    """【放大镜多选回归】【完整流程】在environment启动binary，按shape创建边缘双框，root保存证据，返回原生结果"""
    root.mkdir()
    environment = dict(environment, XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    config = root / "config.toml"
    config.write_text(
        '[general]\noverlay="window"\nto=""\n[notifications]\nenabled=false\n'
        '[history]\nenabled=false\n[tools]\ncolor="#334155"\n[tools.stroke]\nwidth=3\n'
        f'[tools.magnifier]\nwidth=3\nzoom=2.75\nshape="{shape}"\n'
    )
    FLOW.empty_desktop(root, REGION)
    with INPUT.launch(binary, environment, config, None, root, shape) as (process, log):
        FLOW.wait_input(process, log, (80, 80))
        INPUT.drag((0, 50), (620, 460))
        session.key("z")
        INPUT.drag((12, 200), (150, 200))
        session.key("r")
        INPUT.drag((270, 170), (350, 230))
        session.key("v")
        INPUT.drag((8, 80), (400, 330))
        selected = GROUP.snapshot(root, "edge-selected")
        center, rotate, bounds = GROUP.controls(selected)
        assert len(GROUP.body_pixels(selected)) > 150, "edge magnifier and rectangle bodies were not visible"
        # 1. 【放大镜多选回归】【边缘旋转】共同吸附旋转四十五度后改宽，实际双框靠近原图边缘，历史恢复全部正文和控制框
        with ROTATION.modifier("Shift_L"):
            INPUT.drag(rotate, ROTATION.rotated(rotate, center, 45))
        rotated = GROUP.snapshot(root, "edge-rotated")
        changed(selected, rotated, "edge magnifier group did not rotate")
        session.key("ctrl+z")
        GROUP.same(root, "rotation-undo", selected)
        session.key("ctrl+y")
        GROUP.same(root, "rotation-redo", rotated)
        panel_x, panel_y = GROUP.swatch_origin(rotated)
        INPUT.move((panel_x + 104 + 7 + round(74 * 2 / 23), panel_y + 22))
        FLOW.NAV.button("left", True)
        try:
            INPUT.move((panel_x + 104 + 7 + 74, panel_y + 22))
        finally:
            FLOW.NAV.button("left", False)
        widened = GROUP.snapshot(root, "rotated-width-24")
        changed(rotated, widened, "width slider changed no rotated magnifier pixels")
        handle = FLOW.pixels(widened, (panel_x + 104, panel_y + 14, panel_x + 192, panel_y + 32), (255, 255, 255))
        assert handle and abs(sum(x for x, _ in handle) / len(handle) - (panel_x + 104 + 7 + 74)) <= 2, "actual width handle did not reach the maximum value"
        session.key("ctrl+z")
        GROUP.same(root, "width-undo", rotated)
        session.key("ctrl+y")
        GROUP.same(root, "width-redo", widened)
        # 2. 【放大镜多选回归】【缓存往返】隐藏控件等待空闲，再恢复并共同移动和非等比缩放，局部重绘不得留下双框旧像素
        session.key("Space")
        idle = FLOW.NAV.SIZE.idle(process, log)
        session.key("Space")
        GROUP.same(root, "idle-restored", widened)
        center, _, bounds = GROUP.controls(widened)
        INPUT.drag(center, (center[0] + 12, center[1] + 8))
        moved = GROUP.snapshot(root, "widened-moved")
        changed(widened, moved, "widened magnifier group did not move")
        session.key("ctrl+z")
        GROUP.same(root, "move-undo", widened)
        INPUT.drag((bounds[2], bounds[3]), (bounds[2] + 18, bounds[3] + 12))
        resized = GROUP.snapshot(root, "widened-nonuniform-resize")
        changed(widened, resized, "widened magnifier group did not resize")
        session.key("ctrl+z")
        GROUP.same(root, "resize-undo", widened)
        session.key("ctrl+y")
        GROUP.same(root, "resize-redo", resized)
        session.key("ctrl+z")
        GROUP.same(root, "resize-restored", widened)
        # 3. 【放大镜多选回归】【整组删除】一次删除完整双框和矩形，撤销重做无残留正文，主题往返保持相同像素
        red = FLOW.pixels(widened, (0, 0, widened.width, widened.height), GROUP.RED)
        assert len(red) > 60, "edge group delete button was not visible"
        INPUT.click((sum(x for x, _ in red) / len(red), sum(y for _, y in red) / len(red)))
        deleted = GROUP.snapshot(root, "edge-deleted")
        assert not FLOW.pixels(deleted, REGION, GROUP.ORANGE) and not GROUP.body_pixels(deleted), "edge group delete retained a body or control"
        session.key("ctrl+z")
        GROUP.same(root, "delete-undo", widened)
        session.key("ctrl+y")
        GROUP.same(root, "delete-redo", deleted)
        session.key("ctrl+z")
        GROUP.same(root, "delete-restored", widened)
        session.key("ctrl+shift+l")
        dark = GROUP.snapshot(root, "edge-dark")
        assert len(FLOW.pixels(dark, REGION, GROUP.ORANGE)) > 150
        session.key("ctrl+shift+l")
        GROUP.same(root, "light-restored", widened)
        exported = ROTATION.export(environment, process, root, "edge-export")
    result = {"shape": shape, "selected_count": 2, "edge_source": [12, 200], "bulk_width": 24, "rotation_delta": 45, "exact_history": True, "cache_restore": True, "delete_all": True, "idle": idle, "export": exported}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def run(binary, root):
    """【放大镜多选回归】【私有矩阵】按binary在已校验私有会话测试两种shape，root保存独立证据，返回构建指纹和结果"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(TMPDIR=str(root / "temporary"), MARK_SHOT_LANG="english", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/collection-magnifier-test")
    Path(environment["TMPDIR"]).mkdir()
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    cases = []
    with session.input_context(), FLOW.NAV.SIZE.pointer_context(backend="x11" if session.backend() is session.niri else "virtual"):
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        for shape in ("rectangle", "circle"):
            result = verify(binary, environment, root / shape, shape)
            cases.append(result)
            print(json.dumps(result), flush=True)
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == fingerprint
    result = {"binary_sha256": fingerprint, "compositor": session.backend().__name__, "cases": cases}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    """【放大镜多选回归】【入口】解析不可变binary和新root，打印真实矩阵结果，失败保留现场，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
