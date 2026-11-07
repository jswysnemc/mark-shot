#!/usr/bin/env python3
"""【标注多选回归】【真实会话】私有Wayland验证框选、共同变换、属性、删除、历史和导出，不连接用户桌面"""

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path

from PIL import ImageChops

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("collection_rotation_support", PROJECT / "scripts/wayland-annotation-rotation.py")
ROTATION = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ROTATION)
TOOLS, FLOW, INPUT, session = ROTATION.TOOLS, ROTATION.FLOW, ROTATION.INPUT, ROTATION.session
REGION = (100, 70, 600, 435)
ORANGE, RED, BODY = (251, 146, 60), (239, 68, 68), (51, 65, 85)
PARK = (650, 480)


def snapshot(root, name):
    """【标注多选回归】【屏幕证据】隐藏指针对正文的影响并保存root/name，返回实际RGB图像"""
    INPUT.move(PARK)
    return FLOW.snapshot(root, name)


def same(root, name, expected):
    """【标注多选回归】【完整恢复】等待root/name在正文和控制框区域与expected逐像素相同，返回真实图像，超时失败"""
    INPUT.move(PARK)
    return TOOLS.presented(root, name, lambda image: ImageChops.difference(expected.crop(REGION), image.crop(REGION)).getbbox() is None, f"{name}: group body or controls were not restored exactly")


def controls(image):
    """【标注多选回归】【控制点测量】按image中的真实橙色像素返回联合框中心、旋转点和矩形，不读取编辑器内部几何"""
    pixels = FLOW.pixels(image, REGION, ORANGE)
    assert len(pixels) > 150, "group frame did not reach the compositor"
    first_y = min(y for _, y in pixels)
    circle = [(x, y) for x, y in pixels if y < first_y + 10]
    rotation = (sum(x for x, _ in circle) / len(circle), first_y + 5.5)
    left, right = min(x for x, _ in pixels) + 3.5, max(x for x, _ in pixels) - 3.5
    top, bottom = rotation[1] + 26, max(y for _, y in pixels) - 3.5
    return ((left + right) / 2, (top + bottom) / 2), rotation, (left, top, right, bottom)


def body_pixels(image):
    """【标注多选回归】【正文测量】从image返回蓝灰正文像素集合，排除橙色控制点、红色删除按钮和灰色背景"""
    return {(x, y) for y in range(REGION[1], REGION[3]) for x in range(REGION[0], REGION[2]) if image.getpixel((x, y))[2] > image.getpixel((x, y))[0] + 5 and image.getpixel((x, y))[2] > image.getpixel((x, y))[1] + 3}


def swatch_origin(image):
    """【标注多选回归】【属性定位】按image找到正文区域外实际蓝灰色块，返回Qt群组属性面板原点，定位失败直接终止"""
    remaining = set(FLOW.pixels(image, (0, 355, image.width, image.height), BODY))
    components = []
    while remaining:
        first = remaining.pop()
        points, pending = {first}, [first]
        while pending:
            x, y = pending.pop()
            for neighbour in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
                if neighbour in remaining:
                    remaining.remove(neighbour)
                    points.add(neighbour)
                    pending.append(neighbour)
        left, right = min(x for x, _ in points), max(x for x, _ in points)
        top, bottom = min(y for _, y in points), max(y for _, y in points)
        if 12 < right - left < 32 and 12 < bottom - top < 32 and len(points) / ((right - left + 1) * (bottom - top + 1)) > 0.7:
            components.append(((left + right) / 2, (top + bottom) / 2))
    assert components, "native group property color swatch was not visible"
    x, y = max(components, key=lambda point: point[1])
    return x - 407, y - 22


def create_group(mixed):
    """【标注多选回归】【真实创建】mixed决定矩形组或混合工具组，通过生产键盘和鼠标入口创建后执行相交框选，无返回值"""
    INPUT.drag((80, 50), (620, 460))
    session.key("r")
    INPUT.drag((180, 130), (280, 210))
    if mixed:
        session.key("d")
        INPUT.drag((335, 150), (450, 245))
        session.key("t")
        INPUT.click((200, 270))
        session.paste_text("Group")
        session.key("Escape")
        session.key("n")
        INPUT.click((450, 300))
    else:
        INPUT.drag((340, 250), (420, 310))
    session.key("v")
    INPUT.drag((130, 100), (510, 345))


def properties(root, selected):
    """【标注多选回归】【批量滑条】从selected实际色块定位宽度与透明度，root保存变化和撤销证据，返回属性面板位置"""
    x, y = swatch_origin(selected)
    # 1. 【标注多选回归】【统一宽度】点击原生宽度滑条八像素位置，实际正文覆盖必须增加，撤销恢复原始成员和控制框
    INPUT.click((x + 104 + 7 + 74 * 7 / 23, y + 22))
    widened = snapshot(root, "bulk-width")
    assert len(body_pixels(widened)) > len(body_pixels(selected)), "group width slider did not thicken the actual members"
    session.key("ctrl+z")
    same(root, "width-undo", selected)
    # 2. 【标注多选回归】【独立透明度】点击原生百分之五十位置，正文变化而联合框位置保持不变，撤销只恢复本次属性
    INPUT.click((x + 265 + 7 + 62 / 2, y + 22))
    faded = snapshot(root, "bulk-opacity")
    assert ImageChops.difference(selected.crop(REGION), faded.crop(REGION)).getbbox(), "group opacity slider changed no body pixels"
    assert controls(faded)[2] == controls(selected)[2], "opacity moved the group geometry"
    session.key("ctrl+z")
    same(root, "opacity-undo", selected)
    return [x, y]


def reedit_line(root, group, center):
    """【标注多选回归】【单项重新编辑】按center估计群组旋转后的线端，从root真实像素定位并编辑，恢复group联合选择后返回实际端点"""
    INPUT.click((130, 70))
    unselected = snapshot(root, "line-group-exit")
    requested = ROTATION.rotated((335 + 24, 150 + 14), center, 90)
    point = min(body_pixels(unselected), key=lambda point: (point[0] - requested[0]) ** 2 + (point[1] - requested[1]) ** 2)
    assert (point[0] - requested[0]) ** 2 + (point[1] - requested[1]) ** 2 < 144, "rotated line body was outside the production endpoint hit tolerance"
    INPUT.click(point)
    before = snapshot(root, "single-rotated-line")
    handle = FLOW.pixels(before, (requested[0] - 14, requested[1] - 14, requested[0] + 14, requested[1] + 14), ORANGE)
    assert len(handle) > 20, "reselected line did not expose its visible endpoint control"
    point = (sum(x for x, _ in handle) / len(handle), sum(y for _, y in handle) / len(handle))
    INPUT.drag(point, (point[0] + 14, point[1] - 8))
    after = snapshot(root, "single-line-endpoint-edited")
    assert ImageChops.difference(before.crop(REGION), after.crop(REGION)).getbbox(), "selected rotated line endpoint did not respond to the drag"
    session.key("ctrl+z")
    same(root, "single-line-undo", before)
    session.key("ctrl+y")
    same(root, "single-line-redo", after)
    session.key("ctrl+z")
    same(root, "single-line-restored", before)
    INPUT.drag((130, 70), (540, 425))
    same(root, "group-reselected-after-line", group)
    return list(point)


def verify(binary, environment, root, mixed):
    """【标注多选回归】【完整流程】按mixed在environment运行binary，root保存实际操作和导出，返回本例证据"""
    root.mkdir()
    environment = dict(environment, XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    config = root / "config.toml"
    config.write_text('[general]\noverlay="window"\nto=""\n[notifications]\nenabled=false\n[history]\nenabled=false\n[tools]\ncolor="#334155"\n[tools.stroke]\nwidth=3\n[tools.line]\nwidth=3\n')
    FLOW.empty_desktop(root, REGION)
    with INPUT.launch(binary, environment, config, None, root, "mixed" if mixed else "rectangles") as (process, log):
        FLOW.wait_input(process, log, (80, 80))
        create_group(mixed)
        selected = snapshot(root, "selected-group")
        original_center, original_rotation, original_bounds = controls(selected)
        assert len(body_pixels(selected)) > 150, "marquee selected no actual annotation bodies"
        panel_origin = properties(root, selected)
        session.key("v")
        same(root, "same-pick-tool", selected)
        # 1. 【标注多选回归】【移动与历史】群组空隙优先命中共同移动，右键不开始手势，左键一次移动可逐像素撤销重做
        INPUT.move(original_center)
        FLOW.NAV.button("right", True)
        FLOW.NAV.button("right", False)
        same(root, "right-inert", selected)
        INPUT.drag(original_center, (original_center[0] + 24, original_center[1] + 14))
        moved = snapshot(root, "moved-group")
        assert ImageChops.difference(selected.crop(REGION), moved.crop(REGION)).getbbox(), "group move had no visible effect"
        moved_center, handle, moved_bounds = controls(moved)
        assert all(abs(actual - expected - delta) <= 1 for actual, expected, delta in zip(moved_center, original_center, (24, 14))), "group did not move with the common frame"
        session.key("ctrl+z")
        same(root, "move-undo", selected)
        session.key("ctrl+y")
        same(root, "move-redo", moved)
        # 2. 【标注多选回归】【旋转与缩放】真实Shift增量旋转和共同角点缩放形成独立历史，未选中对象不进入新图层
        with ROTATION.modifier("Shift_L"):
            INPUT.drag(handle, ROTATION.rotated(handle, moved_center, 90))
        quarter = snapshot(root, "quarter-turn")
        assert ImageChops.difference(moved.crop(REGION), quarter.crop(REGION)).getbbox(), "group rotation had no visible effect"
        session.key("ctrl+z")
        same(root, "rotation-undo", moved)
        session.key("ctrl+y")
        same(root, "rotation-redo", quarter)
        _, _, bounds = controls(quarter)
        INPUT.drag((bounds[2], bounds[3]), (bounds[2] + 24, bounds[3] + 18))
        resized = snapshot(root, "resized-group")
        assert ImageChops.difference(quarter.crop(REGION), resized.crop(REGION)).getbbox(), "group resize had no visible effect"
        session.key("ctrl+z")
        same(root, "resize-undo", quarter)
        # 3. 【标注多选回归】【按住撤销】左键仍按下时撤销结束旧移动，后续释放不能重复提交
        center = controls(quarter)[0]
        INPUT.move(center)
        FLOW.NAV.button("left", True)
        INPUT.move((center[0] + 12, center[1] + 8))
        snapshot(root, "held-move")
        session.key("ctrl+z")
        FLOW.NAV.button("left", False)
        same(root, "held-undo", quarter)
        line_endpoint = reedit_line(root, quarter, moved_center) if mixed else None
        # 4. 【标注多选回归】【整组删除】实际红色按钮一次删除全部成员，撤销恢复正文与群组控制框，重做没有残留正文
        red = FLOW.pixels(quarter, REGION, RED)
        assert len(red) > 60, "group delete button was not visible"
        delete = (sum(x for x, _ in red) / len(red), sum(y for _, y in red) / len(red))
        INPUT.click(delete)
        deleted = snapshot(root, "group-deleted")
        assert not FLOW.pixels(deleted, REGION, ORANGE) and not body_pixels(deleted), "group delete retained a member or a control"
        session.key("ctrl+z")
        same(root, "delete-undo", quarter)
        session.key("ctrl+y")
        same(root, "delete-redo", deleted)
        session.key("ctrl+z")
        same(root, "delete-restored", quarter)
        session.key("ctrl+shift+l")
        dark = snapshot(root, "dark-group")
        assert len(FLOW.pixels(dark, REGION, ORANGE)) > 150 and len(FLOW.pixels(dark, REGION, RED)) > 60
        session.key("ctrl+shift+l")
        same(root, "light-restored", quarter)
        session.key("Space")
        idle = FLOW.NAV.SIZE.idle(process, log)
        session.key("Space")
        exported = ROTATION.export(environment, process, root, "export")
    result = {"mixed": mixed, "selected_count": 4 if mixed else 2, "original_center": original_center, "rotation_handle": original_rotation, "original_bounds": original_bounds, "moved_bounds": moved_bounds, "panel_origin": panel_origin, "reedited_line_endpoint": line_endpoint, "exact_history": True, "bulk_width": True, "bulk_opacity": True, "delete_all": True, "idle": idle, "export": exported}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def run(binary, root):
    """【标注多选回归】【私有矩阵】在已校验的当前私有会话运行binary，root保存独立配置和证据，返回不可变构建指纹及两类结果"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(TMPDIR=str(root / "temporary"), MARK_SHOT_LANG="english", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/annotation-collection-test")
    Path(environment["TMPDIR"]).mkdir()
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    with session.input_context(), FLOW.NAV.SIZE.pointer_context(backend="x11" if session.backend() is session.niri else "virtual"):
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        cases = []
        for mixed in (False, True):
            result = verify(binary, environment, root / ("mixed" if mixed else "rectangles"), mixed)
            cases.append(result)
            print(json.dumps(result), flush=True)
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == fingerprint
    result = {"binary_sha256": fingerprint, "compositor": session.backend().__name__, "cases": cases}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    """【标注多选回归】【入口】读取不可变binary与新root，输出实际验证结果，失败保留现场，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
