"""【普通控制回归】【完整操作】复用实际页面与原始像素期望，控制位置始终来自可见画面"""

import re
import time
from pathlib import Path

from PIL import ImageChops


def paused_resources(process):
    """【普通控制回归】【静止资源】读取 process 的实际线程、描述符和CPU计数，返回两个静止样本，异常增长即失败"""
    def sample():
        """【普通控制回归】【进程样本】返回本次进程的实际内核计数，无参数，不连接外部桌面"""
        root = Path(f"/proc/{process.pid}")
        fields = (root / "stat").read_text().split(")", 1)[1].split()
        return {"cpu_ticks": int(fields[11]) + int(fields[12]), "threads": len(list((root / "task").iterdir())), "fds": len(list((root / "fd").iterdir())), "rss_pages": int(fields[21])}

    before = sample()
    time.sleep(0.8)
    after = sample()
    assert process.poll() is None
    assert after["cpu_ticks"] - before["cpu_ticks"] <= 1, "paused ordinary controls keep consuming CPU"
    assert (before["threads"], before["fds"]) == (after["threads"], after["fds"]), "paused ordinary controls keep allocating resources"
    return {"before": before, "after": after}


def controls(support, environment, root, name):
    """【普通控制回归】【控件等待】按 support、environment、root 和 name 等待真实可见按钮，返回坐标，不使用协议配置代替像素"""
    def ready():
        """【普通控制回归】【可见检测】取得本次私有画面，仍处于捕获避让时返回空值，无参数"""
        try:
            return support.controls(support.frame(environment, root, name))
        except AssertionError:
            return None

    return support.wait(ready, "the ordinary scroll controls did not become visible")


def pause(support, environment, root, name, buttons):
    """【普通控制回归】【实际暂停】按可见图标判断 buttons 是否已经暂停，再点击运行状态的按钮；support、environment和root保存现场，返回原状态"""
    image = support.frame(environment, root, name)
    x, y = buttons["pause"]
    pixels = {(xx, yy) for yy in range(round(y) - 11, round(y) + 12) for xx in range(round(x) - 11, round(x) + 12) if max(image.getpixel((xx, yy))[:3]) < 170}
    assert len(pixels) > 30, "the actual pause or resume SVG was not visible"
    top = min(yy for _xx, yy in pixels)
    bottom = max(yy for _xx, yy in pixels)
    assert bottom - top >= 10, "the control SVG was clipped"
    rows = [{xx for xx, yy in pixels if yy == row} for row in range(top, bottom + 1)]
    spans = [max(row) - min(row) + 1 if row else 0 for row in rows]
    # 1. 【普通控制回归】【轮廓判定】平行线顶部宽度与中部接近，恢复三角形顶部明显收窄，不能依赖分数采样后的连通数量
    paused = max(spans[:max(2, len(spans) // 4)]) < max(spans) * 0.7
    if not paused:
        support.INPUT.click(buttons["pause"])
    return paused


def axis_roundtrip(support, environment, root, trace, log):
    """【普通控制回归】【区域往返】按 support、environment、root 暂停并以真实指针移出再移回，trace和log核对页面与选区身份，返回两个区域"""
    buttons = controls(support, environment, root, "drag-ready")
    pause(support, environment, root, "drag-pause", buttons)
    support.paused_idle(log)
    origin = support.origin(trace)
    positions = []
    for index, delta in enumerate([20, -20]):
        buttons = controls(support, environment, root, f"drag-{index}-before")
        x, y = buttons["axis"]
        count = log.read_text().count("【滚动截图】【区域调整】")
        support.INPUT.drag((x, y), (x, y + delta))
        support.wait(lambda before=count: log.read_text().count("【滚动截图】【区域调整】") > before, "the axis handle did not move the original region")
        line = [line for line in log.read_text().splitlines() if "【滚动截图】【区域调整】" in line][-1]
        positions.append(list(map(float, re.findall(r"(?:left|top|right|bottom): ([\d.]+)", line))))
    assert positions[0] == [40.0, 60.0, 320.0, 340.0], "axis dragging accumulated the moving panel origin"
    assert positions[1] == list(map(float, support.REGION)), "axis dragging did not restore the original region"
    assert support.origin(trace) == origin, "axis dragging reached the underlying page"
    buttons = controls(support, environment, root, "drag-restored")
    support.INPUT.click(buttons["pause"])
    return positions


def verify(support, binary, environment, root, trace, horizontal):
    """【普通控制回归】【主流程】以 support 和 binary 在 environment 执行横纵操作，root保存证据，trace记录实际页面位移"""
    root.mkdir()
    support.INPUT.click((160, 160))
    support.session.key("Home")
    time.sleep(0.15)
    first = support.crop(support.frame(environment, root, "source-first", True))
    parts = [first]
    step = round(48 * first.width / (support.REGION[2] - support.REGION[0]))
    with support.capture(binary, environment, root, "scroll") as (process, log):
        buttons = controls(support, environment, root, "panel-first")
        assert "get_layer_surface" not in log.read_text(), "the layerless entry bound a Layer Shell surface"
        assert "get_popup" in log.read_text(), "the control panel did not use its standard constrained role"
        assert not re.search(r"xdg_popup@\d+\.grab\(", log.read_text()), "scroll controls requested an exclusive popup grab"
        # 1. 【普通控制回归】【按钮职责】非左键到达真实控件后，不得暂停、交付、退出或打开另存为
        for name in ["axis", "pause", "annotate", "save", "copy", "cancel"]:
            before = len(re.findall(r"wl_pointer@\d+\.button,", log.read_text()))
            support.INPUT.click(buttons[name], "right")
            support.wait(lambda previous=before: len(re.findall(r"wl_pointer@\d+\.button,", log.read_text())) >= previous + 2, "right button did not reach the ordinary control panel")
            assert process.poll() is None
            assert not any(node.get("app_id") == "mark-shot-save-as" for node in support.tree(environment))
        if horizontal:
            support.INPUT.click(buttons["axis"])
            drag = None
        else:
            drag = axis_roundtrip(support, environment, root, trace, log)
        for index in range(8):
            support.scroll(trace, 0 if horizontal else 1, horizontal)
            current = support.crop(support.frame(environment, root, f"source-{index}", True))
            parts.append(current.crop((current.width - step, 0, current.width, current.height) if horizontal else (0, current.height - step, current.width, current.height)))
        if not horizontal:
            # 2. 【普通控制回归】【底层键盘】点击页面后方向键必须仍到达页面，普通控制窗口不得持有独占键盘
            support.INPUT.click((160, 160))
            before = support.origin(trace)
            support.session.key("Down")
            support.wait(lambda: support.origin(trace)[1] == before[1] + 48, "the ordinary controls kept the page keyboard focus")
            time.sleep(0.2)
            current = support.crop(support.frame(environment, root, "source-keyboard", True))
            parts.append(current.crop((0, current.height - step, current.width, current.height)))
        buttons = controls(support, environment, root, "panel-grown")
        already_paused = pause(support, environment, root, "before-pause", buttons)
        idle = support.paused_idle(log)
        resources = paused_resources(process)
        buttons = controls(support, environment, root, "panel-paused")
        support.INPUT.click(buttons["save"])
        support.wait(lambda: any(node.get("app_id") == "mark-shot-save-as" for node in support.tree(environment)), "ordinary scroll save did not open")
        support.command(environment, '[app_id="mark-shot-save-as"] focus')
        support.session.key("Escape")
        support.wait(lambda: "【滚动截图】【保存取消】" in log.read_text(), "ordinary scroll controls did not restore after canceled save")
        assert process.poll() is None and not list((root / "saved").glob("*.png"))
        buttons = controls(support, environment, root, "panel-save-canceled")
        if not horizontal:
            # 3. 【普通控制回归】【面板浏览】鼠标实际位置以按钮行推算到预览内部，滚轮必须仅改变细节预览
            support.INPUT.move((buttons["save"][0] - 70, buttons["save"][1] - 70))
            before = support.frame(environment, root, "preview-end")
            origin = support.origin(trace)
            support.SUPPORT.SUPPORT.HIT.wheel(1, 2)
            support.changed_frame(environment, root, "preview-browsed", before)
            assert support.origin(trace) == origin, "preview scrolling also changed the underlying page"
            support.INPUT.click(buttons["pause"], "right")
            original = support.frame(environment, root, "panel-light")
            support.session.key("ctrl+shift+l")
            support.changed_frame(environment, root, "panel-dark", original)
            support.session.key("ctrl+shift+l")
            support.changed_frame(environment, root, "panel-light-restored", original, True)
            buttons = controls(support, environment, root, "panel-annotate-ready")
            support.INPUT.click(buttons["annotate"])
            support.wait(lambda: "image loaded + preview mapped" in log.read_text() or 'set_title(Some("mark-shot"))' in log.read_text()[log.read_text().rfind("【滚动截图】【标注交接】"):], "ordinary scroll annotation did not map the full image editor")
            time.sleep(0.4)
            support.frame(environment, root, "annotated-editor")
            support.session.key("ctrl+c")
        else:
            support.INPUT.click(buttons["copy"])
        expected = support.joined(parts, horizontal)
        expected.save(root / "expected.png")
        exported = support.exported(environment, process, root, expected)
    return {"horizontal": horizontal, "six_controls": True, "right_button_isolated": True, "constrained_popup_without_grab": True, "axis_drag_roundtrip": drag, "already_paused_by_avoidance": already_paused, "paused_counts": idle, "paused_resources": resources, "save_cancel_restored": True, "preview_and_theme": not horizontal, "export": exported}


def save_cancel(support, binary, environment, root, trace):
    """【普通控制回归】【保存取消】按参数验证普通控制窗口实际保存一次和取消恢复页面，返回完整原图证据"""
    root.mkdir()
    support.INPUT.click((160, 160))
    support.session.key("Home")
    time.sleep(0.15)
    before = support.frame(environment, root, "before", True)
    expected = support.crop(before)
    with support.capture(binary, environment, root, "save") as (process, _log):
        buttons = controls(support, environment, root, "panel-save")
        support.INPUT.click(buttons["save"])
        support.wait(lambda: any(node.get("app_id") == "mark-shot-save-as" for node in support.tree(environment)), "ordinary scroll save dialog did not map")
        support.command(environment, '[app_id="mark-shot-save-as"] focus')
        support.session.key("Return")
        assert process.wait(timeout=5) == 0
        files = list((root / "saved").glob("*.png"))
        assert len(files) == 1
        image = support.Image.open(files[0]).convert("RGB")
        assert image.size == expected.size and ImageChops.difference(image, expected).getbbox() is None
    with support.capture(binary, environment, root, "cancel") as (process, _log):
        buttons = controls(support, environment, root, "panel-cancel")
        support.INPUT.click(buttons["cancel"])
        assert process.wait(timeout=5) == 0
    after = support.frame(environment, root, "restored", True)
    assert ImageChops.difference(before, after).getbbox() is None
    assert len(list((root / "saved").glob("*.png"))) == 1
    return {"saved_once": True, "save_all_pixels_equal": True, "cancel_restores_page": True, "origin": support.origin(trace)}
