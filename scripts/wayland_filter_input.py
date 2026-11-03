"""【滤镜测试】【真实输入】通过可见原生控件修改模式、原图强度与透明度，不直接修改应用状态"""

from contextlib import nullcontext
from pathlib import Path
import time
import tomllib

from wayland_editor_locator import color_popover_rectangle, filter_controls
from wayland_filter_checks import alive, capture, quiet
from wayland_filter_fixture import BORDER
from wayland_filter_reference import MODES
from wayland_test_keyboard import hold
import wayland_test_session as session


DEFAULTS = tomllib.loads((Path(__file__).resolve().parents[1] / "src/config/default.toml").read_text())


def key_tool(name):
    """【滤镜测试】【工具切换】发送 name 对应的实际配置快捷键；无返回值"""
    value = DEFAULTS["keys"][f"tool_{name}"]
    session.key(value[0] if isinstance(value, list) else value)


def click(point, count=1):
    """【滤镜测试】【指针点击】在私有桌面 point 单击 count 次；无返回值，等待输入进入编辑器"""
    session.control("click", "--logical", *map(round, point), "--count", count)
    time.sleep(0.05)


def draw(start, end, modifier=None):
    """【滤镜测试】【指针拖动】从 start 拖动至 end，modifier 可保持修饰键；无返回值，只操作隔离会话"""
    with hold(modifier) if modifier else nullcontext():
        session.control("drag", "--from", *map(round, start), "--to", *map(round, end), "--steps", 10)
    time.sleep(0.08)


def select(annotation):
    """【滤镜测试】【区域选择】切换选择工具并点击 annotation 中心；无返回值，不把控制点误当作正文"""
    sx, sy, ex, ey = annotation["logical"]
    key_tool("pick")
    click(((sx + ex) / 2, (sy + ey) / 2))


def choose(process, artifacts, layout, mode, name):
    """【滤镜测试】【模式确认】在 process 实际弹层中选择 mode；layout、artifacts、name 保存输入证据，无返回值"""
    before = quiet(process, artifacts / f"{name}-before.png", layout)
    click(filter_controls(before, BORDER)["mode"])
    time.sleep(0.2)
    capture(artifacts / f"{name}-popover.png")
    session.key("Home")
    for _ in range(MODES.index(mode)):
        session.key("Right")
    session.key("Return")
    alive(process)
    time.sleep(0.3)


def strength(process, artifacts, layout, value, name):
    """【滤镜测试】【强度粘贴】在 process 可见强度字段粘贴并确认 value；layout、artifacts、name 保存证据，无返回值"""
    image = quiet(process, artifacts / f"{name}-field-before.png", layout)
    click(filter_controls(image, BORDER)["strength"], 2)
    session.key("ctrl+a")
    session.paste_text(str(value))
    capture(artifacts / f"{name}-field-active.png")
    session.key("Return")
    alive(process)
    time.sleep(0.1)


def opacity(process, artifacts, layout, value, name):
    """【滤镜测试】【透明输入】点击 process 的真实 alpha 轨道，再通过可见 RGBA 字段精确确认 value；layout、artifacts、name 保存输入证据"""
    before = quiet(process, artifacts / f"{name}-color-before.png", layout)
    click(filter_controls(before, BORDER)["opacity"])
    time.sleep(0.3)
    opened = capture(artifacts / f"{name}-color-open.png")
    left, top, _right, _bottom = color_popover_rectangle(opened, BORDER)
    alpha_center = (left + 8 + value * 183 / 255, top + 150)
    click(alpha_center)
    # 1. 【滤镜测试】【精确透明度】分数缩放截图定位可能相差一个逻辑像素，用实际字段复制的 RGB 与目标 alpha 确认，不放宽导出断言
    click((left + 100, top + 175))
    session.control("copy", "opacity sentinel")
    session.key("ctrl+a")
    session.key("ctrl+c")
    text = session.read_clipboard(session.environment()).strip().removeprefix("#")
    if len(text) not in (6, 8) or any(character not in "0123456789abcdefABCDEF" for character in text):
        raise AssertionError(f"the visible color field did not copy an RGB or RGBA value: {text!r}")
    session.paste_text(f"#{text[:6]}{value:02X}")
    capture(artifacts / f"{name}-alpha-active.png")
    session.key("Return")
    alive(process)
    click((5, layout["safe_y"]))
    time.sleep(0.3)
