"""【单实例原生回归】【协议与空闲】验证转交退出、有限输入、无额外表面及稳定空闲资源"""

import json
import io
from contextlib import ExitStack
from pathlib import Path
import re
import socket
import subprocess
import time

import wayland_text_format_session as session
import wayland_single_instance_session as runtime
from wayland_text_pointer import CURRENT
from wayland_virtual_pointer import transform_for_pointer
from PIL import Image


def forwarded(capture):
    """【单实例原生回归】【转交确认】按 capture 验证一秒内退出、无图片及无图形提交，返回实际耗时"""
    start = time.monotonic()
    assert capture.process.wait(timeout=2) == 0, capture.log.read_text()
    elapsed = time.monotonic() - start
    assert elapsed < 1.5, elapsed
    assert capture.output.stat().st_size == 0
    assert ".damage_buffer(" not in capture.log.read_text(), "forwarding process created another surface buffer"
    return elapsed


def idle(capture, threads=1):
    """【单实例原生回归】【空闲确认】按 capture 采样，threads 指定现有界面允许线程数，返回 CPU 与提交数稳定的实际资源"""
    if threads == 1:
        session.wait_single_thread(capture.process.pid)
    time.sleep(0.15)
    samples = []
    for _ in range(5):
        sample = session.resources(capture.process.pid)
        sample["commits"] = len(re.findall(r"wl_surface@\d+\.commit\(", capture.log.read_text()))
        sample["fds"] = len(list(Path(f"/proc/{capture.process.pid}/fd").iterdir()))
        samples.append(sample)
        time.sleep(0.08)
    assert all(sample["threads"] == threads for sample in samples), samples
    assert len({sample["cpu_ticks"] for sample in samples}) == 1, samples
    assert len({sample["commits"] for sample in samples}) == 1, samples
    assert len({sample["fds"] for sample in samples}) == 1, samples
    return samples


def packet(path, body, expected=True):
    """【单实例原生回归】【直接请求】向已验证私有 path 发送 body，expected 指定处理结果，返回完整 JSON 回复"""
    assert path == runtime.endpoint(), "request path is outside this private compositor"
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as stream:
        stream.settimeout(2)
        stream.connect(str(path))
        stream.sendall(body)
        data = bytearray()
        while b"\n" not in data:
            chunk = stream.recv(1024)
            assert chunk, "control endpoint closed without a complete reply"
            data.extend(chunk)
            assert len(data) <= 16 * 1024
    response = json.loads(data)
    assert response["handled"] is expected, response
    return response


def expired_partial(path):
    """【单实例原生回归】【半包回收】向私有 path 发送未完成请求，返回实际关闭时间，不产生业务请求"""
    assert path == runtime.endpoint()
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as stream:
        stream.settimeout(2)
        stream.connect(str(path))
        start = time.monotonic()
        stream.sendall(b'{"capture":')
        assert stream.recv(1024) == b""
        elapsed = time.monotonic() - start
    assert 0.8 < elapsed < 1.6, elapsed
    return elapsed


def bounded_clients(path, capture, copies=1):
    """【单实例原生回归】【连接上限】按 path、capture 验证八个半包和四个额外接入，copies 为事件源副本数，返回期限与描述符回收证据"""
    assert path == runtime.endpoint()
    descriptors = Path(f"/proc/{capture.process.pid}/fd")
    before = len(list(descriptors.iterdir()))
    started = time.monotonic()
    with ExitStack() as stack:
        clients = []
        for _ in range(8):
            stream = stack.enter_context(socket.socket(socket.AF_UNIX, socket.SOCK_STREAM))
            stream.settimeout(2)
            stream.connect(str(path))
            stream.sendall(b'{"capture":')
            clients.append(stream)
        deadline = time.monotonic() + 0.5
        while len(list(descriptors.iterdir())) != before + 8 * copies:
            assert time.monotonic() < deadline, "the client descriptor set did not reach its bounded size"
            time.sleep(0.01)
        for _ in range(4):
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as extra:
                extra.settimeout(0.5)
                extra.connect(str(path))
                assert extra.recv(1024) == b"", "the service retained more than eight clients"
        for stream in clients:
            assert stream.recv(1024) == b"", "incomplete request triggered business delivery"
    elapsed = time.monotonic() - started
    assert 0.8 < elapsed < 1.6, elapsed
    deadline = time.monotonic() + 0.5
    while len(list(descriptors.iterdir())) != before:
        assert time.monotonic() < deadline, "the service retained a client descriptor after expiry"
        time.sleep(0.01)
    return {"accepted_partial": 8, "excess_closed": 4, "expired_seconds": elapsed,
            "fds_before": before, "fds_after": len(list(descriptors.iterdir()))}


def finish(capture, expected):
    """【单实例原生回归】【全屏参数】按 capture 确认延迟替换后的实际 PNG 与 expected 原始缓冲逐字节一致，返回尺寸"""
    session.key("Return")
    assert capture.process.wait(timeout=5) == 0
    with Image.open(io.BytesIO(capture.output.read_bytes())) as image:
        actual = image.convert("RGBA")
        assert actual.size == expected.size, (actual.size, expected.size)
        assert actual.tobytes() == expected.tobytes(), "fullscreen replacement did not deliver the complete original frame"
        return {"size": list(actual.size), "original_pixels_equal": True}


def select_region(capture):
    """【单实例原生回归】【手动范围】通过已验证私有指针建立选区，capture 确认按钮事件实际到达，无返回值"""
    pointer, logical, _, x11 = CURRENT.get()
    def move(point):
        """【单实例原生回归】【指针移动】按 point 移动当前私有设备，无返回值"""
        if x11:
            coordinates = [round(point[index] * x11[1][["width", "height"][index]] / logical[["width", "height"][index]]) for index in range(2)]
            subprocess.run(["xdotool", "mousemove", "--sync", *map(str, coordinates)], env=x11[0], check=True, timeout=3)
        else:
            pointer.move_abs(*transform_for_pointer(*point, logical["width"], logical["height"], logical.get("pointer_transform", logical["transform"])))
        time.sleep(0.08)
    offset = len(capture.log.read_text())
    move((60, 70))
    deadline = time.monotonic() + 2
    while True:
        positions = re.findall(r"wl_pointer@\d+\.(?:motion|enter), \([^,\n]+(?:, wl_surface@\d+)?, (-?[\d.]+), (-?[\d.]+)\)", capture.log.read_text()[offset:])
        if positions and all(abs(float(value) - expected) < 1 for value, expected in zip(positions[-1], (60, 70))):
            break
        assert time.monotonic() < deadline, "selection start did not reach the active editor"
        time.sleep(0.01)
    offset = len(capture.log.read_text())
    if x11:
        subprocess.run(["xdotool", "mousedown", "1"], env=x11[0], check=True, timeout=3)
    else:
        pointer.press()
    try:
        deadline = time.monotonic() + 2
        while not re.search(r"wl_pointer@\d+\.button, \([^\n]+, 272, 1\)", capture.log.read_text()[offset:]):
            assert time.monotonic() < deadline, "selection press did not reach the active editor"
            time.sleep(0.01)
        time.sleep(0.06)
        move((300, 250))
    finally:
        if x11:
            subprocess.run(["xdotool", "mouseup", "1"], env=x11[0], check=True, timeout=3)
        else:
            pointer.release()
    time.sleep(0.15)
