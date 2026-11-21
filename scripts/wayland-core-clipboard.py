#!/usr/bin/env python3
"""【核心剪贴板回归】【真实会话】隐藏数据控制协议，验证窗口复制、关闭后持有、输出 EOF 和接管退出"""

import argparse
from contextlib import ExitStack, contextmanager
import json
import hashlib
import io
import os
from pathlib import Path
import select
import socket
import subprocess
import threading
import tempfile
import time

from PIL import Image

from wayland_layer_shell_filter import Relay
from wayland_sway_keyboard import Keyboard
from wayland_test_runtime import TestRuntime


@contextmanager
def connection(environment):
    """【核心剪贴板回归】【协议隔离】按 environment 连接本次私有合成器，返回客户端及转发器，退出回收自有连接"""
    client, endpoint = socket.socketpair()
    upstream = socket.socket(socket.AF_UNIX)
    upstream.connect(str(Path(environment["XDG_RUNTIME_DIR"]) / environment["WAYLAND_DISPLAY"]))
    relay = Relay()
    relay.add(endpoint, upstream, ["ext_data_control_manager_v1", "zwlr_data_control_manager_v1"])
    worker = threading.Thread(target=relay.run)
    worker.start()
    try:
        yield client, relay
    finally:
        client.close()
        relay.stop()
        worker.join(timeout=5)
        assert not worker.is_alive(), "clipboard relay did not stop"
        relay.close()


def wait(predicate, description):
    """【核心剪贴板回归】【有界等待】按 predicate 等待真实状态，返回成立结果，八秒后报告 description"""
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        result = predicate()
        if result:
            return result
        time.sleep(0.04)
    raise AssertionError(description)


def windows(environment, pid=None, app_id="mark-shot"):
    """【核心剪贴板回归】【窗口检查】按 environment 查询唯一测试应用窗口，pid 仅供调用方关联；代理连接的凭据属于转发器"""
    values = json.loads(subprocess.check_output(["niri", "msg", "-j", "windows"], env=environment))
    return [window for window in values if window.get("app_id") == app_id]


def read_output(process):
    """【核心剪贴板回归】【管道结束】读取 process 的输出直到 EOF，返回原始 PNG，服务继续持有管道时明确失败"""
    deadline = time.monotonic() + 5
    result = bytearray()
    while time.monotonic() < deadline:
        if not select.select([process.stdout], [], [], max(0, deadline - time.monotonic()))[0]:
            break
        data = os.read(process.stdout.fileno(), 65536)
        if not data:
            return bytes(result)
        result.extend(data)
    raise AssertionError("clipboard owner kept stdout open after closing its window")


def run(binary, root):
    """【核心剪贴板回归】【完整流程】按 binary 和 root 创建私有桌面，验证连续两次截图复制，返回可审查结果"""
    processes = []
    logs = []
    with tempfile.TemporaryDirectory(prefix="ms-clip-") as runtime_root, TestRuntime(Path(runtime_root), scale=1.0) as environment:
        keyboard = Keyboard(environment)
        config = root / "config.toml"
        config.write_text('[general]\nto=""\noverlay="window"\n[desktop]\ntray_enabled=false\nshortcuts_enabled=false\n[history]\nenabled=false\n[notifications]\nenabled=false\n')
        connections = ExitStack()
        try:
            reports = []
            for index in range(3):
                client, relay = connections.enter_context(connection(environment))
                output = (root / f"copy-{index}.log").open("w")
                logs.append(output)
                private = dict(environment, WAYLAND_SOCKET=str(client.fileno()), WAYLAND_DEBUG="client")
                private.pop("MARK_SHOT_TEST_X11_DISPLAY", None)
                arguments = [str(binary), "--config", str(config), "-n", "-t", "co"]
                if index == 2:
                    arguments.append("-f")
                process = subprocess.Popen(arguments,
                                           stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=output,
                                           env=private, pass_fds=(client.fileno(),))
                processes.append(process)
                if index < 2:
                    wait(lambda: windows(environment, process.pid), "screenshot window did not appear")
                    wait(lambda: "wl_keyboard@" in (root / f"copy-{index}.log").read_text()
                         and ".enter," in (root / f"copy-{index}.log").read_text(), "screenshot did not receive keyboard focus")
                    keyboard.key("Ctrl+A")
                    keyboard.key("Ctrl+C")
                wait(lambda: not windows(environment, process.pid), "screenshot window remained visible after copy")
                exported = read_output(process)
                assert exported.startswith(b"\x89PNG\r\n\x1a\n"), "capture did not output PNG"
                types = subprocess.check_output(["wl-paste", "--list-types"], env=environment, timeout=5).decode().splitlines()
                assert "image/png" in types and "x-kde-force-image-copy" in types, types
                actual = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment, timeout=5)
                assert actual == exported, "clipboard image differed from stdout"
                assert process.poll() is None, "clipboard owner exited before takeover"
                if index:
                    assert processes[index - 1].wait(timeout=5) == 0, "previous clipboard owner did not exit"
                reports.append({"png_bytes": len(actual), "stdout_eof": True, "window_closed": True,
                                "clipboard_survived": True, "protocol": relay.report()})
                if index == 2:
                    subprocess.run(["wl-copy", "--clear"], env=environment, check=True, timeout=5)
                    assert process.wait(timeout=5) == 0, "clipboard owner did not exit after clear"
            auxiliary = auxiliary_windows(binary, root, environment, keyboard, connections, processes, logs)
            return {"copies": reports, "takeover_exited": True, "auxiliary": auxiliary}
        finally:
            keyboard.close()
            for process in processes:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=5)
                process.stdout.close()
            for log in logs:
                log.close()
            connections.close()


def auxiliary_windows(binary, root, environment, keyboard, connections, processes, logs):
    """【核心剪贴板回归】【其他窗口】按测试资源验证贴图大图背压及 OCR 文字复制，返回关闭交接结果"""
    # 1. 【核心剪贴板回归】【贴图传输】不可压缩图像超过管道容量，关闭窗口后再模拟接收方背压和来源取消
    image = Image.frombytes("RGB", (512, 512), hashlib.shake_256(b"core-clipboard-pin").digest(512 * 512 * 3))
    path = root / "pin.png"
    image.save(path)
    config = root / "auxiliary.toml"
    config.write_text('[desktop]\ntray_enabled=false\nshortcuts_enabled=false\n[history]\nenabled=false\n[notifications]\nenabled=false\n[pinned]\nalways_on_top=false\n')
    client, _relay = connections.enter_context(connection(environment))
    output = (root / "pin.log").open("w")
    logs.append(output)
    private = dict(environment, WAYLAND_SOCKET=str(client.fileno()), WAYLAND_DEBUG="client")
    process = subprocess.Popen([str(binary), "--config", str(config), "--pin-image", str(path)],
                               stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=output,
                               env=private, pass_fds=(client.fileno(),))
    processes.append(process)
    wait(lambda: windows(environment, app_id="mark-shot-pin"), "pin window did not appear")
    time.sleep(0.2)
    keyboard.key("Ctrl+C")
    keyboard.key("Escape")
    wait(lambda: not windows(environment, app_id="mark-shot-pin"), "pin did not close")
    read_output(process)
    actual = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment, timeout=5)
    assert Image.open(io.BytesIO(actual)).convert("RGB").tobytes() == image.tobytes(), "pin clipboard pixels changed"
    receiver = subprocess.Popen(["wl-paste", "--type", "image/png"], env=environment, stdout=subprocess.PIPE, bufsize=0)
    try:
        assert receiver.stdout.read(1) == b"\x89", "large transfer did not start"
        subprocess.run(["wl-copy", "--clear"], env=environment, check=True, timeout=5)
        data = b"\x89" + receiver.communicate(timeout=5)[0]
        assert data == actual, "cancelled source truncated an active transfer"
        assert process.wait(timeout=5) == 0, "pin owner remained after final transfer completed"
    finally:
        if receiver.poll() is None:
            receiver.kill()
            receiver.wait()
        receiver.stdout.close()
    # 2. 【核心剪贴板回归】【结果文字】使用真实 OCR 结果传输帧，不启动识别模型，验证独立结果窗口的正文复制
    text = "独立结果窗口 clipboard text"
    header = json.dumps({"text": text, "lines": []}).encode()
    png = path.read_bytes()
    frame = b"MSOCR001" + len(header).to_bytes(4, "little") + len(png).to_bytes(4, "little") + header + png
    client, _relay = connections.enter_context(connection(environment))
    output = (root / "ocr-result.log").open("w")
    logs.append(output)
    private = dict(environment, WAYLAND_SOCKET=str(client.fileno()), WAYLAND_DEBUG="client")
    process = subprocess.Popen([str(binary), "--config", str(config), "--ocr-result"],
                               stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=output,
                               env=private, pass_fds=(client.fileno(),))
    processes.append(process)
    process.stdin.write(frame)
    process.stdin.close()
    wait(lambda: windows(environment, app_id="mark-shot-ocr-result"), "OCR result window did not appear")
    time.sleep(0.2)
    keyboard.key("Ctrl+A")
    keyboard.key("Ctrl+C")
    received = subprocess.check_output(["wl-paste", "--no-newline", "--type", "text/plain;charset=utf-8"], env=environment, timeout=5)
    assert received.decode() == text, "OCR result clipboard text differed"
    keyboard.key("Escape")
    wait(lambda: not windows(environment, app_id="mark-shot-ocr-result"), "OCR result window did not close")
    read_output(process)
    assert process.poll() is None, "OCR clipboard owner exited on window close"
    assert subprocess.check_output(["wl-paste", "--no-newline"], env=environment, timeout=5).decode() == text
    subprocess.run(["wl-copy", "--clear"], env=environment, check=True, timeout=5)
    assert process.wait(timeout=5) == 0, "OCR clipboard owner did not exit after clear"
    return {"pin_pixels_equal": True, "cancel_during_transfer_completed": True, "ocr_text_equal": True, "window_close_preserved_text": True}


def main():
    """【核心剪贴板回归】【命令入口】读取程序与证据目录并执行回归，无参数，无返回值"""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    root = options.root.resolve()
    root.mkdir(parents=True, exist_ok=False)
    result = run(options.binary.resolve(), root)
    (root / "result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(result, ensure_ascii=False))


if __name__ == "__main__":
    main()
