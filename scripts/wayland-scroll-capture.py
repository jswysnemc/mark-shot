#!/usr/bin/env python3
"""【长截图回归】【原生流程】仅在私有Sway检查实际底层滚动、六类操作和完整原图交接"""

import argparse
import hashlib
import importlib.util
import io
import json
import os
import re
import subprocess
import time
from contextlib import contextmanager
from pathlib import Path

from PIL import Image, ImageChops
from wayland_scroll_raw_capture import capture as raw_capture
from wayland_scroll_markers import check as check_markers

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("scroll_priority_support", PROJECT / "scripts/wayland-priority-ui.py")
SUPPORT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SUPPORT)
INPUT, FLOW, session = SUPPORT.INPUT, SUPPORT.FLOW, SUPPORT.session
SWAYMSG = PROJECT / "target/test-tools/sway/usr/bin/swaymsg"
REGION = (40, 40, 320, 320)


def wait(predicate, message):
    """【长截图回归】【条件等待】限时检查predicate，超时以message报告真实失败，返回条件结果"""
    deadline = time.monotonic() + 4
    while True:
        value = predicate()
        if value:
            return value
        assert time.monotonic() < deadline, message
        time.sleep(0.04)


def command(environment, value):
    """【长截图回归】【私有窗口管理】按environment在已校验的Sway执行value，返回JSON结果"""
    session.environment()
    return json.loads(subprocess.check_output([str(SWAYMSG), "-r", value], env=environment))


def tree(environment):
    """【长截图回归】【窗口查询】返回environment私有Sway的真实窗口树，不查询用户桌面"""
    root = json.loads(subprocess.check_output([str(SWAYMSG), "-r", "-t", "get_tree"], env=environment))
    result, pending = [], [root]
    while pending:
        node = pending.pop()
        result.append(node)
        pending.extend(node.get("nodes", []) + node.get("floating_nodes", []))
    return result


def frame(environment, root, name, physical=False):
    """【长截图回归】【实际画面】按environment采集root/name，physical控制原始设备像素，返回RGB图像"""
    path = root / f"{name}.png"
    image = raw_capture()
    if not physical:
        metadata = json.loads((Path(environment["XDG_RUNTIME_DIR"]) / "runtime.json").read_text())
        logical = metadata["outputs"]["HEADLESS-1"]["logical"]
        # 1. 【长截图回归】【输入坐标】控制栏观察图按逻辑大小取最近像素，完整原图比较始终保留原始缓冲
        image = image.resize((logical["width"], logical["height"]), Image.Resampling.NEAREST)
    image.save(path)
    return image


def changed_frame(environment, root, name, reference, restored=False):
    """【长截图回归】【主题呈现】等待实际画面相对reference改变或按restored完整恢复，返回新画面，超时保留现场"""
    def ready():
        """【长截图回归】【像素条件】采集本次私有画面并核对完整像素，无参数，尚未呈现时返回空值"""
        image = frame(environment, root, name)
        equal = ImageChops.difference(reference, image).getbbox() is None
        return image if equal == restored else None

    return wait(ready, "scroll panel theme did not present the requested pixels")


def crop(image, region=REGION):
    """【长截图回归】【原图裁切】按region将逻辑选区映射到image的真实设备像素，返回选区图像"""
    metadata = json.loads((Path(os.environ["XDG_RUNTIME_DIR"]) / "runtime.json").read_text())
    scale = metadata["outputs"]["HEADLESS-1"]["logical"]["scale"]
    return image.crop(tuple(round(value * scale) for value in region))


def controls(image):
    """【长截图回归】【可见按钮】从image实际保存按钮颜色定位控制栏，返回六个按钮的真实中心"""
    remaining = {(x, y) for y in range(image.height) for x in range(image.width) if image.getpixel((x, y)) in ((16, 185, 129), (52, 211, 153))}
    groups = []
    while remaining:
        pending = [remaining.pop()]
        pixels = []
        while pending:
            point = pending.pop()
            pixels.append(point)
            for adjacent in [(point[0] - 1, point[1]), (point[0] + 1, point[1]), (point[0], point[1] - 1), (point[0], point[1] + 1)]:
                if adjacent in remaining:
                    remaining.remove(adjacent)
                    pending.append(adjacent)
        groups.append(pixels)
    pixels = max(groups, key=len, default=[])
    assert len(pixels) > 200, "the six-button scroll panel has no visible primary save control"
    x = (min(point[0] for point in pixels) + max(point[0] for point in pixels)) / 2
    y = (min(point[1] for point in pixels) + max(point[1] for point in pixels)) / 2
    return {"axis": (x - 145, y), "pause": (x - 94, y), "annotate": (x - 47, y), "save": (x, y), "copy": (x + 47, y), "cancel": (x + 94, y)}


@contextmanager
def page(binary, environment, root):
    """【长截图回归】【底层页面】在environment启动binary并记录root的实际位置，退出仅回收本次进程"""
    trace = root / "page-position.txt"
    with (root / "page.log").open("w") as log:
        process = subprocess.Popen([str(binary), str(trace)], env=environment, stdout=subprocess.DEVNULL, stderr=log)
        try:
            wait(lambda: next((node for node in tree(environment) if node.get("app_id") == "mark-shot-scroll-test-page"), None), "the native scroll page did not map")
            command(environment, '[app_id="mark-shot-scroll-test-page"] fullscreen enable')
            time.sleep(0.2)
            yield process, trace
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)


@contextmanager
def capture(binary, environment, root, name, region=REGION):
    """【长截图回归】【捕获进程】在environment用binary通过真实选区启动长截图，root/name记录原生协议"""
    config = root / f"{name}.toml"
    config.write_text('[general]\noverlay="window"\nto="c"\n[notifications]\nenabled=false\n[history]\nenabled=false\n')
    path = root / f"{name}.log"
    with path.open("w") as log:
        process = subprocess.Popen([str(binary), "-S", "--config", str(config), "--output", str(root / "saved")], env=dict(environment, WAYLAND_DEBUG="client"), stdout=subprocess.DEVNULL, stderr=log)
        try:
            FLOW.wait_input(process, path, (region[0], region[1]))
            INPUT.drag(region[:2], region[2:])
            wait(lambda: "【滚动截图】【会话开始】" in path.read_text() or len(re.findall(r"capture\(\)", path.read_text())) >= 2, "selecting a region did not start scroll capture")
            time.sleep(0.25)
            yield process, path
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)


def origin(trace):
    """【长截图回归】【页面位置】读取trace实际接收的滚动坐标，尚未移动时返回零点"""
    try:
        return tuple(map(int, trace.read_text().split()))
    except FileNotFoundError:
        return (0, 0)


def scroll(trace, axis, horizontal=False, direction=1):
    """【长截图回归】【底层滚轮】向真实页面投递轴位移，核对trace对应轴移动四十八逻辑像素，无返回值"""
    before = origin(trace)
    pointer = INPUT.CURRENT.get()[0]
    INPUT.move((160, 160))
    pointer.scroll(dx=10.0 * direction if horizontal else 0.0, dy=0.0 if horizontal else 10.0 * direction)
    expected = list(before)
    expected[axis] += 48 * direction
    wait(lambda: origin(trace) == tuple(expected), "the live page did not receive the scroll event")
    time.sleep(0.15)


def joined(parts, horizontal):
    """【长截图回归】【独立期望图】将parts中真实页面新增条带按horizontal连接，返回完整期望原图"""
    width = sum(part.width for part in parts) if horizontal else parts[0].width
    height = parts[0].height if horizontal else sum(part.height for part in parts)
    result = Image.new("RGB", (width, height))
    cursor = 0
    for part in parts:
        result.paste(part, (cursor, 0) if horizontal else (0, cursor))
        cursor += part.width if horizontal else part.height
    return result


def exported(environment, process, root, expected):
    """【长截图回归】【原图交付】等待process正常退出并核对剪贴板的expected全部像素，root保留完整PNG"""
    assert process.wait(timeout=5) == 0
    data = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment, timeout=3)
    (root / "export.png").write_bytes(data)
    image = Image.open(io.BytesIO(data)).convert("RGB")
    assert image.size == expected.size, f"scroll image dimensions differ: actual={image.size}, expected={expected.size}"
    assert ImageChops.difference(image, expected).getbbox() is None, "scroll export contains missing rows, duplicated rows or overlay pixels"
    return {"size": list(image.size), "sha256": hashlib.sha256(data).hexdigest(), "all_pixels_equal": True}


def paused_idle(log):
    """【长截图回归】【暂停静止】先等待log中在途帧提交完成，再核对捕获请求和缓冲提交均停止，返回稳定计数"""
    def counters():
        """【长截图回归】【协议计数】返回log内捕获请求与缓冲提交数量，无参数"""
        text = log.read_text()
        return (len(re.findall(r"(?:\.capture\(\)|\.capture_output\()", text)), len(re.findall(r"wl_surface@\d+\.attach\(", text)))

    deadline = time.monotonic() + 4
    previous = counters()
    unchanged = time.monotonic()
    while time.monotonic() - unchanged < 0.3:
        current = counters()
        if current != previous:
            previous, unchanged = current, time.monotonic()
        assert time.monotonic() < deadline, "the paused capture did not settle its in-flight frame"
        time.sleep(0.04)
    time.sleep(0.7)
    assert counters() == previous, "paused capture keeps requesting frames or submitting unchanged buffers"
    return previous


def verify(binary, environment, root, trace, horizontal):
    """【长截图回归】【完整操作】以binary执行方向、暂停、保存取消、预览浏览和标注或复制，返回原生证据"""
    root.mkdir()
    INPUT.click((160, 160))
    session.key("Home")
    time.sleep(0.15)
    first = crop(frame(environment, root, "source-first", True))
    parts = [first]
    scale = first.width / (REGION[2] - REGION[0])
    step = round(48 * scale)
    with capture(binary, environment, root, "scroll") as (process, log):
        shown = frame(environment, root, "panel-first")
        buttons = controls(shown)
        assert "set_keyboard_interactivity(2)" in log.read_text(), "scroll layer did not release exclusive keyboard focus"
        for button in ["pause", "copy", "save", "cancel"]:
            previous = len(re.findall(r"wl_pointer@\d+\.button,", log.read_text()))
            INPUT.click(buttons[button], "right")
            wait(lambda count=previous: len(re.findall(r"wl_pointer@\d+\.button,", log.read_text())) >= count + 2, "the right button did not reach the intended panel control")
            assert process.poll() is None
        if horizontal:
            INPUT.click(buttons["axis"])
        for index in range(20 if horizontal else 8):
            scroll(trace, 0 if horizontal else 1, horizontal)
            current = crop(frame(environment, root, f"source-{index}", True))
            parts.append(current.crop((current.width - step, 0, current.width, current.height) if horizontal else (0, current.height - step, current.width, current.height)))
        grown = frame(environment, root, "panel-grown")
        buttons = controls(grown)
        markers = check_markers(grown, buttons)
        axis = 0 if horizontal else 1
        for _ in range(2):
            scroll(trace, axis, horizontal, -1)
        def marker_position(name, expected):
            """【长截图回归】【异步位置】等待本次捕获完成并满足expected，返回实际双框，不重复投递滚动"""
            image = frame(environment, root, name)
            current = check_markers(image, controls(image))
            return current if expected(current["yellow"][axis]) else None

        wait(lambda: marker_position("panel-reversed", lambda value: value < markers["yellow"][axis]),
             "capture marker did not follow reverse scrolling")
        for _ in range(2):
            scroll(trace, axis, horizontal)
        wait(lambda: marker_position("panel-returned", lambda value: abs(value - markers["yellow"][axis]) <= 1),
             "capture marker did not return to the original source position")
        if not horizontal:
            # 1. 【长截图回归】【底层键盘】鼠标回到页面后方向键必须继续到达页面，不能留在独占捕获层
            INPUT.click((160, 160))
            before = origin(trace)
            session.key("Down")
            wait(lambda: origin(trace)[1] == before[1] + 48, "the page lost keyboard focus during scroll capture")
            capture_offset = log.stat().st_size
            # 1. 【长截图回归】【异步捕获】确认页面移动后等待两个实际捕获完成，避免固定延时在降频阶段提前暂停
            wait(lambda: len(re.findall(r'(?:ext_image_copy_capture_frame_v1|zwlr_screencopy_frame_v1)@\d+\.ready',
                                        log.read_bytes()[capture_offset:].decode(errors="replace"))) >= 2,
                 "the moved page did not reach the capture session before pause")
            current = crop(frame(environment, root, "source-keyboard", True))
            parts.append(current.crop((0, current.height - step, current.width, current.height)))
        INPUT.click(buttons["pause"])
        INPUT.move(buttons["pause"])
        time.sleep(0.2)
        idle = paused_idle(log)
        buttons = controls(frame(environment, root, "panel-paused"))
        INPUT.click(buttons["save"])
        wait(lambda: any(node.get("app_id") == "mark-shot-save-as" for node in tree(environment)), "scroll save did not open the native file dialog")
        command(environment, '[app_id="mark-shot-save-as"] focus')
        session.key("Escape")
        wait(lambda: not any(node.get("app_id") == "mark-shot-save-as" for node in tree(environment)), "save cancellation did not restore scroll capture")
        assert process.poll() is None and not list((root / "saved").glob("*.png"))
        buttons = controls(frame(environment, root, "panel-save-canceled"))
        if horizontal:
            INPUT.click(buttons["copy"])
        else:
            # 2. 【长截图回归】【预览浏览】真实滚轮只改变面板细节，之后Enter必须进入完整原图编辑器
            INPUT.move((400, 150))
            before = frame(environment, root, "preview-end")
            before_markers = check_markers(before, controls(before))
            SUPPORT.SUPPORT.HIT.wheel(1, 2)
            after = frame(environment, root, "preview-browsed")
            assert ImageChops.difference(before, after).getbbox() is not None, "preview wheel did not browse the long image"
            browsed = check_markers(after, controls(after))
            # 黄色尾边可能由重叠的青色框遮挡，只比较未被遮挡的捕获起点
            assert browsed["yellow"][1] == before_markers["yellow"][1], "browsing moved the capture marker"
            assert browsed["cyan"] != before_markers["cyan"], "browsing did not move the cyan viewport marker"
            # 3. 【长截图回归】【主题轮换】真实面板必须响应双主题切换并恢复同一主题像素
            INPUT.move(buttons["pause"])
            original = frame(environment, root, "panel-light")
            session.key("ctrl+shift+l")
            dark = changed_frame(environment, root, "panel-dark", original)
            assert ImageChops.difference(original, dark).getbbox() is not None, "scroll panel did not switch theme"
            session.key("ctrl+shift+l")
            light = changed_frame(environment, root, "panel-light-restored", original, True)
            assert ImageChops.difference(original, light).getbbox() is None, "scroll panel theme roundtrip changed its pixels"
            INPUT.click(buttons["pause"], "right")
            session.key("Return")
            wait(lambda: "image loaded + preview mapped" in log.read_text() or 'set_app_id(Some("mark-shot-editor"))' in log.read_text()[log.read_text().find("【滚动截图】【标注交接】"):], "Enter did not open the stitched image editor")
            time.sleep(0.4)
            frame(environment, root, "annotated-editor")
            session.key("ctrl+c")
        expected = joined(parts, horizontal)
        expected.save(root / "expected.png")
        result = exported(environment, process, root, expected)
    return {"horizontal": horizontal, "markers": markers, "six_controls": True, "right_button_isolated": True, "paused_commits_stable": True, "paused_capture_and_commit_counts": idle, "save_cancel_restored": True, "export": result}


def large_region(binary, environment, root, trace):
    """【长截图回归】【捕获污染】验证预览无避让空间时隐藏、静止暂停和继续捕获，返回全部像素比较证据"""
    root.mkdir()
    INPUT.click((160, 160))
    session.key("Home")
    time.sleep(0.15)
    baseline = frame(environment, root, "page-original", True)
    metadata = json.loads((Path(environment["XDG_RUNTIME_DIR"]) / "runtime.json").read_text())
    logical = metadata["outputs"]["HEADLESS-1"]["logical"]
    region = (8, 8, logical["width"] - 8, logical["height"] - 8)
    first = crop(baseline, region)
    step = round(48 * first.width / (region[2] - region[0]))
    parts = [first]
    with capture(binary, environment, root, "large", region) as (process, _log):
        scroll(trace, 1)
        current = crop(frame(environment, root, "capturing-hidden", True), region)
        parts.append(current.crop((0, current.height - step, current.width, current.height)))
        time.sleep(1.3)
        buttons = controls(frame(environment, root, "idle-panel"))
        INPUT.click(buttons["pause"])
        time.sleep(0.05)
        # 1. 【长截图回归】【继续捕获】按钮必须在再次采集之前消失，不能把暂停面板当成新页面内容
        scroll(trace, 1)
        current = crop(frame(environment, root, "resumed-hidden", True), region)
        parts.append(current.crop((0, current.height - step, current.width, current.height)))
        time.sleep(1.3)
        buttons = controls(frame(environment, root, "idle-panel-restored"))
        INPUT.click(buttons["copy"])
        expected = joined(parts, False)
        expected.save(root / "expected.png")
        result = exported(environment, process, root, expected)
    return {"overlapping_preview_hidden": True, "idle_paused": True, "resumed": True, "export": result}


def save_and_cancel(binary, environment, root, trace):
    """【长截图回归】【最终操作】验证实际保存和明确取消不执行预设复制，返回真实文件与恢复页面证据"""
    root.mkdir()
    INPUT.click((160, 160))
    session.key("Home")
    time.sleep(0.15)
    baseline = frame(environment, root, "before", True)
    expected = crop(baseline)
    with capture(binary, environment, root, "save") as (process, _log):
        buttons = controls(frame(environment, root, "save-panel"))
        INPUT.click(buttons["save"])
        wait(lambda: any(node.get("app_id") == "mark-shot-save-as" for node in tree(environment)), "scroll save dialog did not map")
        command(environment, '[app_id="mark-shot-save-as"] focus')
        session.key("Return")
        assert process.wait(timeout=5) == 0
        files = list((root / "saved").glob("*.png"))
        assert len(files) == 1, "scroll save must write one file"
        actual = Image.open(files[0]).convert("RGB")
        assert actual.size == expected.size and ImageChops.difference(actual, expected).getbbox() is None, "scroll save changed the original pixels"
    with capture(binary, environment, root, "cancel") as (process, _log):
        buttons = controls(frame(environment, root, "cancel-panel"))
        INPUT.click(buttons["cancel"])
        assert process.wait(timeout=5) == 0
        restored = frame(environment, root, "canceled-page", True)
        assert ImageChops.difference(restored, baseline).getbbox() is None, "cancel did not release the scroll surface and restore the live page"
        assert len(list((root / "saved").glob("*.png"))) == 1, "cancel saved an extra image"
    return {"save_all_pixels_equal": True, "saved_once": True, "cancel_restores_page": True, "page_origin": origin(trace)}


def idle_resume(binary, environment, root, trace):
    """【长截图回归】【静止恢复】以binary验证静止降频后连续滚动不丢失内容，root保存实际原图和独立期望图"""
    root.mkdir()
    INPUT.click((160, 160))
    session.key("Home")
    time.sleep(0.15)
    first = crop(frame(environment, root, "source-first", True))
    parts = [first]
    step = round(48 * first.width / (REGION[2] - REGION[0]))
    with capture(binary, environment, root, "scroll") as (process, log):
        # 1. 【长截图回归】【降频边界】先超过静止期限，再从底层页面连续滚动，不能用面板输入提前唤醒捕获
        time.sleep(1.5)
        for index in range(8):
            scroll(trace, 1)
            current = crop(frame(environment, root, f"source-{index}", True))
            parts.append(current.crop((0, current.height - step, current.width, current.height)))
        buttons = controls(frame(environment, root, "panel-grown"))
        INPUT.click(buttons["pause"])
        paused_idle(log)
        buttons = controls(frame(environment, root, "panel-paused"))
        INPUT.click(buttons["copy"])
        expected = joined(parts, False)
        expected.save(root / "expected.png")
        result = exported(environment, process, root, expected)
    return {"idle_seconds": 1.5, "page_origin": origin(trace), "export": result}


def run(binary, page_binary, root):
    """【长截图回归】【私有入口】在已验证会话使用binary与page_binary执行矩阵，root保存不可变指纹和结果"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"), MARK_SHOT_LANG="english", MARK_SHOT_LOG="info", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/scroll-test")
    previous = dict(os.environ)
    os.environ.update(environment)
    try:
        with session.input_context(), FLOW.NAV.SIZE.pointer_context():
            session.sway.keyboard_ready()
            with page(page_binary, environment, root) as (_process, trace):
                report = {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "idle_resume": idle_resume(binary, environment, root / "idle-resume", trace), "cases": [verify(binary, environment, root / name, trace, horizontal) for name, horizontal in [("vertical", False), ("horizontal", True)]], "large_region": large_region(binary, environment, root / "large", trace), "save_cancel": save_and_cancel(binary, environment, root / "save-cancel", trace)}
    finally:
        os.environ.clear()
        os.environ.update(previous)
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    """【长截图回归】【命令入口】解析独立binary、page与新root后执行私有测试，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--page", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.page.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
