"""【箭头测试】【真实场景】验证头部与线型矩阵、骨架编辑、属性历史和手势取消"""

from contextlib import nullcontext
from pathlib import Path
import subprocess
import time
import tomllib

from wayland_editor_locator import line_controls
from wayland_test_keyboard import hold
import wayland_test_session as session
from wayland_arrow_checks import BORDER, HEADS, PATTERNS, alive, capture, check_alpha, check_shaft, curve_point, difference, idle, quiet, red_bounds, red_near, rotated


DEFAULTS = tomllib.loads((Path(__file__).resolve().parents[1] / "src/config/default.toml").read_text())


def key_tool(name):
    """【箭头测试】【工具切换】按 name 对应的真实快捷键切换工具；无返回值"""
    session.key(DEFAULTS["keys"][f"tool_{name}"])


def click(point, count=1):
    """【箭头测试】【点击动作】在私有桌面 point 单击 count 次并等待输入；无返回值"""
    session.control("click", "--logical", *map(round, point), "--count", count)
    time.sleep(0.05)


def draw(start, end, modifier=None):
    """【箭头测试】【拖动动作】在私有桌面从 start 拖到 end，modifier 可保持修饰键；无返回值"""
    with hold(modifier) if modifier else nullcontext():
        session.control("drag", "--from", *map(round, start), "--to", *map(round, end), "--steps", 10)
    time.sleep(0.08)


def choose(process, artifacts, layout, kind, index, name, arrow=True):
    """【箭头测试】【样式选择】在 process 的 kind 弹层选择 index；layout、artifacts、name 保存证据，arrow 决定属性控件排列"""
    before = quiet(process, artifacts / f"{name}-before.png", layout)
    click(line_controls(before, BORDER, arrow=arrow)[kind])
    time.sleep(0.2)
    capture(artifacts / f"{name}-popover.png")
    session.key("Home")
    for _ in range(index):
        session.key("Right")
    session.key("Return")
    alive(process)


def matrix(process, artifacts, layout):
    """【箭头测试】【完整样式矩阵】在 process 中绘制四头部乘三线型的直线和单控制点曲线；layout 限定位置，artifacts 保存证据"""
    for cell in layout["matrix"]:
        head, pattern = cell["head"], cell["pattern"]
        name = f"matrix-{head}-{pattern}"
        key_tool("arrow")
        choose(process, artifacts, layout, "head", head, f"{name}-head")
        choose(process, artifacts, layout, "pattern", pattern, f"{name}-pattern")
        before = quiet(process, artifacts / f"{name}-empty.png", layout)
        draw(cell["start"], cell["end"])
        draw(cell["curve_start"], cell["curve_end"])
        key_tool("pick")
        middle = ((cell["curve_start"][0] + cell["curve_end"][0]) / 2, cell["curve_start"][1])
        click(middle, 2)
        draw(middle, cell["control"])
        key_tool("arrow")
        after = quiet(process, artifacts / f"{name}-drawn.png", layout)
        if not difference(before, after, cell["region"]):
            raise RuntimeError(f"the {HEADS[head]} / {PATTERNS[pattern]} arrows did not render")
        curve_region = (cell["curve_start"][0] + 55, cell["curve_start"][1] + 7, cell["curve_end"][0] - 55, cell["curve_start"][1] + 16)
        red_bounds(after, curve_region)
        check_shaft(after, cell["start"], cell["end"], PATTERNS[pattern])
        check_alpha(after, cell["region"])


def panel_lifecycle(process, artifacts, layout):
    """【箭头测试】【弹层生命周期】验证 process 的两类弹层重点击关闭、Escape 和滚轮隔离；layout、artifacts 保存状态与空闲证据"""
    key_tool("arrow")
    for kind in ["head", "pattern"]:
        image = quiet(process, artifacts / f"{kind}-toggle-before.png", layout)
        position = line_controls(image, BORDER)[kind]
        click(position)
        time.sleep(0.2)
        opened = capture(artifacts / f"{kind}-toggle-open.png")
        if not difference(image, opened, (50, 100, image.width - 50, 330)):
            raise RuntimeError(f"the {kind} popover did not open")
        idle(process, artifacts, f"{kind}-popover")
        session.control("scroll", "--to", image.width - 70, layout["safe_y"] - 20, "--dy", -1, "--notches", 3)
        click(position)
        closed = quiet(process, artifacts / f"{kind}-toggle-closed.png", layout)
        if difference(image, closed, layout["content"]):
            raise RuntimeError(f"the {kind} toggle or scroll altered persistent annotations")
        if difference(image, closed, (50, 100, image.width - 50, 330)):
            raise RuntimeError(f"the {kind} toggle reopened its popover instead of closing it")
        click(position)
        time.sleep(0.2)
        session.key("Escape")
        alive(process)
        escaped = quiet(process, artifacts / f"{kind}-escape-closed.png", layout)
        if difference(image, escaped, (50, 100, image.width - 50, 330)):
            raise RuntimeError(f"Escape did not close only the {kind} popover")


def create_edit_curve(process, artifacts, layout):
    """【箭头测试】【曲线历史】在 process 中双击插入控制点并移出正文；layout 提供端点，artifacts 验证两次独立历史"""
    key_tool("arrow")
    choose(process, artifacts, layout, "head", 0, "edit-head")
    choose(process, artifacts, layout, "pattern", 0, "edit-pattern")
    draw(layout["edit_start"], layout["edit_end"])
    straight = quiet(process, artifacts / "edit-straight.png", layout)
    middle = ((layout["edit_start"][0] + layout["edit_end"][0]) / 2, layout["edit_start"][1])
    key_tool("pick")
    click(middle, 2)
    capture(artifacts / "edit-control-inserted.png")
    key_tool("arrow")
    collinear = quiet(process, artifacts / "edit-collinear-control.png", layout)
    key_tool("pick")
    click(middle)
    draw(middle, layout["edit_control"])
    key_tool("arrow")
    curved = quiet(process, artifacts / "edit-curved.png", layout)
    if not difference(straight, curved, layout["edit_region"]):
        raise RuntimeError("dragging the inserted control did not create a quadratic curve")
    session.key("ctrl+z")
    undone = quiet(process, artifacts / "edit-control-move-undo.png", layout)
    if difference(collinear, undone, layout["content"]):
        raise RuntimeError("one undo did not restore the collinear control")
    session.key("ctrl+z")
    removed = quiet(process, artifacts / "edit-control-insert-undo.png", layout)
    if difference(straight, removed, layout["content"]):
        raise RuntimeError("insertion undo did not restore the original arrow")
    session.key("ctrl+shift+z")
    session.key("ctrl+shift+z")
    redone = quiet(process, artifacts / "edit-control-redo.png", layout)
    if difference(curved, redone, layout["content"]):
        raise RuntimeError("curve insertion and movement did not redo exactly")


def check_drag(process, artifacts, layout, name, start, end, modifier=None):
    """【箭头测试】【已有拖动】在 process 中从 start 拖到 end，modifier 约束手势；layout、artifacts 验证单次历史并返回改变图像"""
    key_tool("arrow")
    before = quiet(process, artifacts / f"{name}-before.png", layout)
    key_tool("pick")
    click(layout["edit_start"])
    draw(start, end, modifier)
    key_tool("arrow")
    changed = quiet(process, artifacts / f"{name}-changed.png", layout)
    if not difference(before, changed, layout["content"]):
        raise RuntimeError(f"the {name} gesture did not modify the curve")
    session.key("ctrl+z")
    undone = quiet(process, artifacts / f"{name}-undo.png", layout)
    if difference(before, undone, layout["content"]):
        raise RuntimeError(f"the {name} gesture did not undo exactly")
    session.key("ctrl+shift+z")
    redone = quiet(process, artifacts / f"{name}-redo.png", layout)
    if difference(changed, redone, layout["content"]):
        raise RuntimeError(f"the {name} gesture did not redo exactly")
    session.key("ctrl+z")
    return changed


def controls(process, artifacts, layout):
    """【箭头测试】【端点外框旋转】验证 process 的端点、正文外控制点、Ctrl 缩放和 Shift 旋转；layout、artifacts 保存历史证据"""
    start, end = layout["edit_start"], layout["edit_end"]
    check_drag(process, artifacts, layout, "start-point", start, (start[0] + 20, start[1] - 25))
    check_drag(process, artifacts, layout, "end-point", end, (end[0] + 25, end[1] + 15))
    control = layout["edit_control"]
    check_drag(process, artifacts, layout, "external-control", control, (control[0] + 20, control[1] + 15))
    before = quiet(process, artifacts / "frame-reference.png", layout)
    left, top, right, bottom = red_bounds(before, layout["edit_region"])
    corner = (right + 10, top - 10)
    changed = check_drag(process, artifacts, layout, "proportional-frame", corner, (corner[0] + 25, corner[1] - 18), "Ctrl")
    bounds = red_bounds(changed, layout["edit_region"])
    ratio = (right - left) / (bottom - top)
    new_ratio = (bounds[2] - bounds[0]) / (bounds[3] - bounds[1])
    if abs(new_ratio / ratio - 1) > 0.1:
        raise RuntimeError(f"Ctrl frame resize lost its proportions: {ratio} -> {new_ratio}")
    center = ((left + right) / 2, (top + bottom) / 2)
    rotation = (center[0], min(top, layout["edit_control"][1]) - 34)
    target = rotated(rotation, center, 32)
    changed = check_drag(process, artifacts, layout, "snapped-rotation", rotation, target, "Shift")
    if not red_near(changed, rotated(end, center, 45)):
        raise RuntimeError("Shift rotation did not snap the endpoint to forty-five degrees")


def multiple_controls(process, artifacts, layout):
    """【箭头测试】【多控制点删除】在 process 的真实曲线上插入第二控制点，删除当前点而非整箭头；layout、artifacts 验证历史"""
    point = curve_point(layout["edit_start"], layout["edit_control"], layout["edit_end"], 0.75)
    key_tool("pick")
    click(layout["edit_start"])
    click(point, 2)
    capture(artifacts / "second-control-inserted.png")
    draw(point, layout["second_control"])
    key_tool("arrow")
    before = quiet(process, artifacts / "two-controls.png", layout)
    key_tool("pick")
    click(layout["edit_start"])
    click(layout["second_control"])
    session.key("Delete")
    key_tool("arrow")
    removed = quiet(process, artifacts / "second-control-deleted.png", layout)
    if not difference(before, removed, layout["edit_region"]):
        raise RuntimeError("Delete did not remove the selected control")
    if not red_near(removed, layout["edit_start"]) or not red_near(removed, layout["edit_end"]):
        raise RuntimeError("Delete removed the whole arrow instead of the selected control")
    session.key("ctrl+z")
    undone = quiet(process, artifacts / "second-control-delete-undo.png", layout)
    if difference(before, undone, layout["content"]):
        raise RuntimeError("control deletion did not undo exactly")
    session.key("ctrl+shift+z")
    redone = quiet(process, artifacts / "second-control-delete-redo.png", layout)
    if difference(removed, redone, layout["content"]):
        raise RuntimeError("control deletion did not redo exactly")


def edit_width(process, artifacts, layout, arrow, width):
    """【箭头测试】【独立笔宽输入】在 process 选中对象中粘贴 width 并确认，arrow 决定控件布局；layout、artifacts 保存单次历史证据"""
    name = "arrow" if arrow else "line"
    before = quiet(process, artifacts / f"{name}-width-before.png", layout)
    click(line_controls(before, BORDER, arrow=arrow)["width"], 2)
    session.key("ctrl+a")
    session.paste_text(str(width))
    session.key("Return")
    changed = quiet(process, artifacts / f"{name}-width-{width}.png", layout)
    if not difference(before, changed, layout["content"]):
        raise RuntimeError(f"pasting {name} width did not change the selected annotation")
    session.key("ctrl+z")
    undone = quiet(process, artifacts / f"{name}-width-undo.png", layout)
    if difference(before, undone, layout["content"]):
        raise RuntimeError(f"the {name} width did not undo in one transaction")
    session.key("ctrl+shift+z")
    redone = quiet(process, artifacts / f"{name}-width-redo.png", layout)
    if difference(changed, redone, layout["content"]):
        raise RuntimeError(f"the {name} width did not redo in one transaction")


def line_patterns(process, artifacts, layout):
    """【箭头测试】【直线独立样式】在 process 中创建三种普通线型并单独修改直线笔宽；layout、artifacts 保存像素与输入证据"""
    key_tool("line")
    for index, (start, end) in enumerate(layout["lines"]):
        choose(process, artifacts, layout, "pattern", index, f"line-pattern-{index}", arrow=False)
        draw(start, end)
        image = quiet(process, artifacts / f"line-pattern-{index}-drawn.png", layout)
        check_shaft(image, start, end, PATTERNS[index])
    key_tool("pick")
    start, end = layout["lines"][0]
    click(((start[0] + end[0]) / 2, start[1]))
    edit_width(process, artifacts, layout, False, 7)
    key_tool("arrow")


def existing_styles(process, artifacts, layout):
    """【箭头测试】【选中样式历史】修改 process 已有箭头并验证独立历史，再绘制新箭头确认默认值不变；layout、artifacts 保存证据"""
    key_tool("arrow")
    before = quiet(process, artifacts / "existing-style-before.png", layout)
    key_tool("pick")
    click(curve_point(layout["edit_start"], layout["edit_control"], layout["edit_end"], 0.5))
    choose(process, artifacts, layout, "head", 3, "existing-head")
    choose(process, artifacts, layout, "pattern", 2, "existing-pattern")
    key_tool("arrow")
    changed = quiet(process, artifacts / "existing-style-changed.png", layout)
    if not difference(before, changed, layout["edit_region"]):
        raise RuntimeError("changing selected arrow styles did not update the annotation")
    session.key("ctrl+z")
    session.key("ctrl+z")
    undone = quiet(process, artifacts / "existing-style-undo.png", layout)
    if difference(before, undone, layout["content"]):
        raise RuntimeError("head and line-style changes did not undo as two transactions")
    session.key("ctrl+shift+z")
    session.key("ctrl+shift+z")
    redone = quiet(process, artifacts / "existing-style-redo.png", layout)
    if difference(changed, redone, layout["content"]):
        raise RuntimeError("head and line-style changes did not redo exactly")
    start = (layout["edit_end"][0] + 65, layout["edit_start"][1] - 35)
    end = (start[0] + 150, start[1])
    draw(start, end)
    future = quiet(process, artifacts / "unchanged-future-defaults.png", layout)
    check_shaft(future, start, end, "solid")
    if not red_near(future, (end[0] - 25, end[1] + 6), radius=1) or red_near(future, (start[0] + 40, start[1] - 24), radius=2):
        raise RuntimeError("changing a selected head changed the subsequent filled single-head default")
    session.key("ctrl+z")
    restored = quiet(process, artifacts / "future-default-test-undo.png", layout)
    if difference(changed, restored, layout["content"]):
        raise RuntimeError("the future-default probe did not undo without changing the selected styles")


def cancel(process, artifacts, layout, existing=False):
    """【箭头测试】【未提交取消】在 process 中保持草稿或 existing 端点拖动并取消；layout、artifacts 验证旧像素精确恢复"""
    key_tool("arrow")
    name = "existing-cancel" if existing else "draft-cancel"
    before = quiet(process, artifacts / f"{name}-before.png", layout)
    if existing:
        key_tool("pick")
        click(layout["edit_start"])
    start = layout["edit_start"] if existing else layout["cancel_start"]
    end = (start[0] + 35, start[1] - 30) if existing else layout["cancel_end"]
    with subprocess.Popen(["niri-use", "--child", "click", "--logical", *map(str, start), "--hold", "3"], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE) as gesture:
        time.sleep(0.2)
        session.control("move", "--logical", *map(round, end))
        draft = capture(artifacts / f"{name}-active.png")
        if gesture.poll() is not None or not difference(before, draft, layout["content"]):
            raise RuntimeError("the pending arrow was not observable while the button stayed pressed")
        session.key("Escape")
        alive(process)
        gesture.wait(timeout=5)
        if gesture.returncode:
            raise RuntimeError(gesture.stderr.read().decode())
    key_tool("arrow")
    restored = quiet(process, artifacts / f"{name}-restored.png", layout)
    if difference(before, restored, layout["content"]):
        raise RuntimeError(f"{name} altered committed pixels")
