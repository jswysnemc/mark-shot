"""【区域识别原生回归】【交互断言】验证原始轴事件、取消、正常拖选和候选像素恢复"""

import math
import os
import re
import signal
import time
from pathlib import Path


def wheel_events(client, offset):
    """【区域识别原生回归】【单刻度确认】按 client 和 offset 返回本次协议轴，必须只收到一次完整向上滚轮"""
    text = client.read_text()[offset:]
    events = re.findall(r"wl_pointer@\d+\.axis, \([^\n]+", text)
    assert len(events) == 1, events
    assert re.search(r", 0, -[\d.]+\)", events[0]), events
    return events


def pointer_position(client):
    """【区域识别原生回归】【实际端点】按 client 返回最近实际指针坐标，还原协议二百五十六分之一精度"""
    positions = re.findall(r"wl_pointer@\d+\.(?:motion|enter), \([^,\n]+(?:, wl_surface@\d+)?, (-?[\d.]+), (-?[\d.]+)\)", client.read_text())
    assert positions, "client did not receive a pointer position"
    return tuple(round(float(value) * 256) / 256 for value in positions[-1])


def coalesced_return(native, binary, environment, root, point, child_logical):
    """【区域识别原生回归】【合并返回】按给定私有会话暂停本次编辑器并积累移出返回事件，返回取消交付证据"""
    with native.launch(binary, environment, root, "coalesced-return", True, True) as (process, client, output, directory):
        native.move(point)
        native.wait_hover(process, directory, "hover", child_logical)
        native.press(True)
        try:
            assert re.search(r"wl_pointer@\d+\.button, \([^\n]+, 272, 1\)", client.read_text()), "editor did not receive the left press"
            # 1. 【区域识别原生回归】【真实排队】只暂停本次子进程，使中途位移与最终位置在同次协议读取中到达
            os.kill(process.pid, signal.SIGSTOP)
            try:
                deadline = time.monotonic() + 1
                while not re.search(r"State:\s+T", Path(f"/proc/{process.pid}/status").read_text()):
                    assert time.monotonic() < deadline, "owned editor did not stop"
                    time.sleep(0.005)
                native.move((point[0] + 12, point[1] + 10))
                native.move(point)
            finally:
                os.kill(process.pid, signal.SIGCONT)
            native.wait_hover(process, directory, "returned", None)
        finally:
            native.press(False)
        assert process.poll() is None, "coalesced returning drag adopted the old candidate"
        native.click(point, "right")
        assert process.wait(timeout=5) == 0 and output.stat().st_size == 0
        return {"name": "coalesced-return", "queued_excursion_not_adopted": True, "no_output": True}


def run(native, binary, environment, root, baseline, logical, point, child_logical, empty):
    """【区域识别原生回归】【补充场景】使用 native 调度和给定冻结画面参数验证交互，root 保存证据，返回场景结果"""
    reports = []
    for name, image, windows in [("manual-drag", True, True), ("disabled-manual-drag", False, False)]:
        with native.launch(binary, environment, root, name, image, windows) as (process, client, output, directory):
            start = tuple(round(value / 6) * 6 for value in point)
            end = (start[0] + 120, start[1] + 72)
            native.move(start)
            native.wait_hover(process, directory, "hover", child_logical if image else None)
            received_start = pointer_position(client)
            native.press(True)
            try:
                native.move(end)
                native.wait_hover(process, directory, "manual", None)
                received_end = pointer_position(client)
            finally:
                native.press(False)
            scale = logical["scale"]
            # 1. 【区域识别原生回归】【像素覆盖】期望范围来自实际协议端点，私有 X11 转换可能产生分数位移
            crop = tuple(math.floor(value * scale) for value in received_start) + tuple(math.ceil(value * scale) for value in received_end)
            size = native.finish(process, output, baseline.crop(crop))
            reports.append({"name": name, "png_size": size, "pixels_equal": True, "physical_crop": crop,
                            "received_start": received_start, "received_end": received_end})
    with native.launch(binary, environment, root, "return-drag", True, True) as (process, client, output, directory):
        native.move(point)
        native.wait_hover(process, directory, "hover", child_logical)
        native.press(True)
        try:
            native.move((point[0] + 12, point[1] + 10))
            native.wait_hover(process, directory, "dismissed", None)
            native.move(point)
            native.wait_hover(process, directory, "returned", None)
        finally:
            native.press(False)
        assert process.poll() is None, "returning drag adopted the old candidate"
        native.click(point, "right")
        assert process.wait(timeout=5) == 0 and output.stat().st_size == 0
        reports.append({"name": "return-drag", "old_candidate_not_adopted": True})
    with native.launch(binary, environment, root, "hover-away", True, False) as (process, client, output, directory):
        native.move(point)
        native.wait_hover(process, directory, "hover", child_logical)
        native.move((2, 2))
        native.wait_hover(process, directory, "cleared", None)
        restored = native.snapshot(directory, "restored")
        area = tuple(round(value) + (-4 if index < 2 else 4) for index, value in enumerate(child_logical))
        assert restored.crop(area).tobytes() == empty.crop(area).tobytes(), "moving away left stale candidate pixels"
        samples = native.idle(process, client)
        native.click((2, 2), "right")
        assert process.wait(timeout=5) == 0 and output.stat().st_size == 0
        reports.append({"name": "hover-away", "candidate_area_restored": True, "idle": samples})
    for action in ("escape", "right"):
        with native.launch(binary, environment, root, f"cancel-{action}", True, True) as (process, client, output, directory):
            native.move(point)
            native.wait_hover(process, directory, "hover", child_logical)
            if action == "escape":
                native.session.key("Escape")
            else:
                native.click(point, "right")
            assert process.wait(timeout=5) == 0 and output.stat().st_size == 0
            assert not Path(f"/proc/{process.pid}").exists(), "cancelled editor still exists"
            reports.append({"name": f"cancel-{action}", "no_output": True, "process_reaped": True})
    reports.append(coalesced_return(native, binary, environment, root, point, child_logical))
    return reports
