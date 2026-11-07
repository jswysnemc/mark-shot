#!/usr/bin/env python3
"""【普通标注旋转回归】【真实会话】在私有Wayland测试六类正文、吸附、取消、历史、缩放、文字及PNG导出"""

import argparse
from contextlib import contextmanager
import hashlib
import importlib.util
import io
import json
import math
from pathlib import Path
import subprocess
import time

from PIL import Image, ImageChops

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("rotation_tool_support", PROJECT / "scripts/wayland-annotation-selection-tools.py")
TOOLS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(TOOLS)
FLOW, INPUT, session = TOOLS.FLOW, TOOLS.INPUT, TOOLS.session
ORANGE = (251, 146, 60)
REGION = (120, 80, 570, 355)
PARK = (620, 400)


@contextmanager
def modifier(name):
    """【普通标注旋转回归】【实际修饰键】在已经校验的私有设备持有name，退出释放，无返回值，不连接用户键盘"""
    pointer, _, _, x11 = INPUT.CURRENT.get()
    if pointer is None:
        subprocess.run(["xdotool", "keydown", name], env=x11[0], check=True, timeout=3)
    else:
        session.sway.keyboard_ready()
        session.sway.KEYBOARD.get()["keyboard"].modifiers({"Shift_L": 2, "Control_L": 1}[name])
    time.sleep(0.05)
    try:
        yield
    finally:
        if pointer is None:
            subprocess.run(["xdotool", "keyup", name], env=x11[0], check=True, timeout=3)
        else:
            session.sway.KEYBOARD.get()["keyboard"].modifiers(0)
        time.sleep(0.05)


def snapshot(root, name):
    """【普通标注旋转回归】【静止截图】隐藏当前指针影响并记录root/name，返回真实RGB画面"""
    INPUT.move(PARK)
    return FLOW.snapshot(root, name)


def same(root, name, expected):
    """【普通标注旋转回归】【精确恢复】等待root/name与expected在正文及控制点区域完全一致，返回截图，超时失败"""
    return TOOLS.presented(root, name,
        lambda image: ImageChops.difference(expected.crop(REGION), image.crop(REGION)).getbbox() is None,
        f"{name}: body or selected control pixels were not restored exactly")


def unrotated_controls(image):
    """【普通标注旋转回归】【可见控制点】从image实际橙色像素返回轴心、顶部旋转点和八向控制点范围，不用内部标注状态"""
    pixels = FLOW.pixels(image, REGION, ORANGE)
    assert len(pixels) > 150, "ordinary annotation has no complete visible frame"
    min_y = min(y for _, y in pixels)
    circle = [(x, y) for x, y in pixels if y < min_y + 10]
    rotation = (sum(x for x, _ in circle) / len(circle), min_y + 5.5)
    top, bottom = rotation[1] + 26.0, max(y for _, y in pixels) - 3.5
    left, right = min(x for x, _ in pixels) + 3.5, max(x for x, _ in pixels) - 3.5
    center = ((left + right) / 2, (top + bottom) / 2)
    return center, rotation, (left, top, right, bottom)


def body_bounds(image):
    """【普通标注旋转回归】【实际正文范围】按image返回蓝灰正文像素边界，排除橙色控制点、红色按钮及灰色桌面，不读取内部几何"""
    colored = {(x, y) for y in range(105, REGION[3]) for x in range(REGION[0], REGION[2])
             if image.getpixel((x, y))[2] > image.getpixel((x, y))[0] + 5
             and image.getpixel((x, y))[2] > image.getpixel((x, y))[1] + 3}
    # 1. 【普通标注旋转回归】【杂色隔离】过滤橙色控制点重采样产生的独立像素，正文笔迹和字形必须有实际邻域覆盖
    found = [(x, y) for x, y in colored
             if sum((x+dx, y+dy) in colored for dx, dy in [(-1,-1), (0,-1), (1,-1), (-1,0), (1,0), (-1,1), (0,1), (1,1)]) >= 3]
    assert len(found) > 20, "annotation body has no visible colored pixels"
    return min(x for x, _ in found), min(y for _, y in found), max(x for x, _ in found)+1, max(y for _, y in found)+1


def rotated(point, center, degrees):
    """【普通标注旋转回归】【输入目标】按point、center与degrees返回鼠标目标，不作为实际绘制断言的预期图像"""
    sine, cosine = math.sin(math.radians(degrees)), math.cos(math.radians(degrees))
    x, y = point[0] - center[0], point[1] - center[1]
    return center[0] + x * cosine - y * sine, center[1] + x * sine + y * cosine


def export(environment, process, root, name):
    """【普通标注旋转回归】【最终图像】用environment请求process真实导出并退出，在root保存name原始字节，返回指纹、尺寸、正文像素"""
    session.key("ctrl+c")
    assert process.wait(timeout=5) == 0
    data = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment, timeout=3)
    (root / (name + ".png")).write_bytes(data)
    image = Image.open(io.BytesIO(data)).convert("RGB")
    assert len(image.getcolors(image.width * image.height)) > 3, "rotated export contained no annotation body"
    assert not FLOW.pixels(image, (0, 0, image.width, image.height), ORANGE), "selected controls leaked into PNG"
    return {"sha256": hashlib.sha256(data).hexdigest(), "size": list(image.size)}


def verify(binary, environment, root, case):
    """【普通标注旋转回归】【完整工具流程】在environment启动binary创建case，root保存各阶段实际画面，返回全部操作与导出证据"""
    name, key, start, end, hit, extra = case
    root.mkdir()
    environment = dict(environment, XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    config = root / "config.toml"
    config.write_text('[general]\noverlay="window"\nto=""\n[notifications]\nenabled=false\n[history]\nenabled=false\n[tools]\ncolor="#334155"\n' + extra)
    FLOW.empty_desktop(root, REGION)
    report = {"name": name}
    with INPUT.launch(binary, environment, config, None, root, name) as (process, log):
        FLOW.wait_input(process, log, (80, 80))
        INPUT.drag((80, 80), (600, 380))
        session.key(key)
        if name == "text":
            INPUT.click(start)
            session.paste_text("Rotation")
            session.key("Escape")
        else:
            INPUT.drag(start, end)
        session.key("v")
        INPUT.click(hit)
        before = snapshot(root, "original")
        center, handle, bounds = unrotated_controls(before)
        report.update(center=center, rotation_handle=handle)
        # 1. 【普通标注旋转回归】【鼠标职责】右键不开始旋转，左键旋转途中右键取消必须完整恢复
        INPUT.move(handle)
        FLOW.NAV.button("right", True)
        INPUT.move(rotated(handle, center, 90))
        FLOW.NAV.button("right", False)
        INPUT.move(PARK)
        same(root, "right-inert", before)
        INPUT.move(handle)
        FLOW.NAV.button("left", True)
        INPUT.move(rotated(handle, center, 90))
        FLOW.NAV.button("right", True)
        FLOW.NAV.button("right", False)
        FLOW.NAV.button("left", False)
        INPUT.move(PARK)
        same(root, "cancel-restored", before)
        # 2. 【普通标注旋转回归】【真实吸附】按住Shift执行90度后实际控制点移动到右方，历史往返必须逐像素恢复
        with modifier("Shift_L"):
            INPUT.drag(handle, rotated(handle, center, 90))
        quarter = snapshot(root, "quarter-turn")
        assert ImageChops.difference(before.crop(REGION), quarter.crop(REGION)).getbbox(), "rotation changed neither body nor controls"
        target = rotated(handle, center, 90)
        assert len(FLOW.pixels(quarter, (target[0]-8, target[1]-8, target[0]+8, target[1]+8), ORANGE)) > 30, "rotation handle did not follow actual pose"
        original_body, quarter_body = body_bounds(before), body_bounds(quarter)
        assert abs((original_body[2]-original_body[0])-(quarter_body[3]-quarter_body[1])) <= 4, "actual body width did not become rotated body height"
        assert abs((original_body[3]-original_body[1])-(quarter_body[2]-quarter_body[0])) <= 4, "actual body height did not become rotated body width"
        report.update(original_body=original_body, quarter_turn_body=quarter_body, actual_body_rotated=True)
        session.key("ctrl+z")
        INPUT.move(PARK)
        same(root, "undo-original", before)
        session.key("ctrl+y")
        INPUT.move(PARK)
        same(root, "redo-quarter", quarter)
        # 3. 【普通标注旋转回归】【绝对角度网格】90度姿态拖回约30度增量后必须吸附到45度，不能仅吸附本次增量
        with modifier("Shift_L"):
            INPUT.drag(target, rotated(handle, center, 30))
        diagonal = snapshot(root, "snapped-45")
        diagonal_target = rotated(handle, center, 45)
        assert len(FLOW.pixels(diagonal, (diagonal_target[0]-9, diagonal_target[1]-9, diagonal_target[0]+9, diagonal_target[1]+9), ORANGE)) > 30, "Shift did not snap the absolute pose to45 degrees"
        session.key("ctrl+z")
        INPUT.move(PARK)
        same(root, "undo-snap", quarter)
        # 4. 【普通标注旋转回归】【旋转缩放】可见右边对应原始横向，缩放后撤销必须恢复90度正文和控制点
        right = rotated((bounds[2], center[1]), center, 90)
        # 1. 【普通标注旋转回归】【原版收框】单行文字扩宽可保持画面不变，缩窄到实际换行后才能要求可见尺寸变化
        distance = -40 if name == "text" else 24
        INPUT.drag(right, (right[0], right[1] + distance))
        resized = snapshot(root, "rotated-resize")
        assert ImageChops.difference(quarter.crop(REGION), resized.crop(REGION)).getbbox(), "rotated resize had no effect"
        session.key("ctrl+z")
        INPUT.move(PARK)
        same(root, "undo-resize", quarter)
        # 5. 【普通标注旋转回归】【旋转移动】沿实际正文移动后保留姿态，撤销精确恢复，不累计方向变化
        INPUT.drag(center, (center[0]+24, center[1]+16))
        moved = snapshot(root, "rotated-move")
        assert ImageChops.difference(quarter.crop(REGION), moved.crop(REGION)).getbbox(), "rotated movement had no effect"
        session.key("ctrl+z")
        INPUT.move(PARK)
        same(root, "undo-move", quarter)
        if name == "text":
            # 6. 【普通标注旋转回归】【原版文字编辑】文本工具在旋转正文开始编辑，内联编辑器保持横向，提交后旋转正文可整体撤销
            session.key("t")
            INPUT.click(center)
            session.key("ctrl+a")
            session.paste_text("Edited")
            inline = snapshot(root, "inline-editing")
            inline_bounds = body_bounds(inline)
            assert inline_bounds[2]-inline_bounds[0] > inline_bounds[3]-inline_bounds[1], "inline text editor did not remain horizontal"
            session.key("ctrl+Return")
            session.key("v")
            INPUT.click(center)
            edited = snapshot(root, "edited-rotated")
            edited_bounds = body_bounds(edited)
            assert edited_bounds[3]-edited_bounds[1] > edited_bounds[2]-edited_bounds[0], "committed text did not preserve90 degree body rotation"
            assert ImageChops.difference(quarter.crop(REGION), edited.crop(REGION)).getbbox(), "rotated text commit changed no body pixels"
            session.key("ctrl+z")
            INPUT.move(PARK)
            same(root, "undo-text-edit", quarter)
            session.key("ctrl+y")
            INPUT.move(PARK)
            same(root, "redo-text-edit", edited)
            report["text_edit_pose_and_history"] = True
        # 7. 【普通标注旋转回归】【静止资源】旋转标注隐藏操作界面后不创建定时绘制，八次采样必须保持单线程和固定提交数
        session.key("Space")
        time.sleep(0.2)
        report["idle"] = FLOW.NAV.SIZE.idle(process, log)
        session.key("Space")
        report.update(right_button_inert=True, cancellation_exact=True, shift_90_and_45=True,
                      rotation_undo_redo_exact=True, resize_undo_exact=True, move_undo_exact=True,
                      export=export(environment, process, root, "rotated-export"))
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def run(binary, root):
    """【普通标注旋转回归】【六类矩阵】在当前私有会话验证固定binary，root为新证据目录，返回构建指纹及实际流程结果"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(TMPDIR=str(root / "temporary"), MARK_SHOT_LANG="english", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/annotation-rotation")
    Path(environment["TMPDIR"]).mkdir()
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    cases = []
    with session.input_context(), FLOW.NAV.SIZE.pointer_context(backend="x11" if session.backend() is session.niri else "virtual"):
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        for case in TOOLS.CASES:
            if case[0] not in {"rectangle", "ellipse", "pen", "highlighter", "marker", "text"}:
                continue
            result = verify(binary, environment, root / case[0], case)
            cases.append(result)
            print(json.dumps(result), flush=True)
    assert len(cases) == 6
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == fingerprint
    report = {"binary_sha256": fingerprint, "compositor": session.backend().__name__, "cases": cases}
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    """【普通标注旋转回归】【入口】解析不可变构建和新证据目录，输出六类实际验证结果，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
