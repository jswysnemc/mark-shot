#!/usr/bin/env python3
"""【上传回归】【真实界面】私有 Wayland 中验证按钮、左右键、URL 复制、失败恢复、取消和静止资源"""

import argparse
from email.parser import BytesParser
from email.policy import default
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import importlib.util
import io
import json
import math
from pathlib import Path
import queue
import subprocess
import threading
import time
import tomllib

from PIL import Image, ImageChops
import wayland_text_format_session as session
from wayland_text_pointer import click, pointer_context, CURRENT
from wayland_virtual_pointer import BTN_LEFT, transform_for_pointer

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("upload_export_support", PROJECT / "scripts/wayland-export-image.py")
EXPORT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(EXPORT)


class UploadServer(ThreadingHTTPServer):
    """【上传回归】【私有服务】管理本机随机端口请求记录及有限等待，所有图片保存到本次 root"""

    daemon_threads = True

    def __init__(self, root):
        """【上传回归】【服务构造】按 root 初始化本机服务和所属接收队列，无返回值"""
        super().__init__(("127.0.0.1", 0), UploadHandler)
        self.root = root
        self.records = queue.Queue()
        self.release = threading.Event()


class UploadHandler(BaseHTTPRequestHandler):
    """【上传回归】【HTTP接收】验证实际 multipart PNG，并按路径返回成功、失败或等待"""

    def log_message(self, *_arguments):
        """【上传回归】【静默请求】忽略服务器通用日志，_arguments 不进入终端，无返回值"""

    def do_POST(self):
        """【上传回归】【完整请求】接收并解析本次 POST，记录真实 PNG 和请求头，返回受控服务响应"""
        length = int(self.headers["Content-Length"])
        if not 0 < length < 1024 * 1024:
            raise AssertionError("unexpected local upload body size")
        body = self.rfile.read(length)
        message = BytesParser(policy=default).parsebytes(b"Content-Type: " + self.headers["Content-Type"].encode() + b"\r\nMIME-Version: 1.0\r\n\r\n" + body)
        fields, png = {}, None
        for part in message.iter_parts():
            name = part.get_param("name", header="content-disposition")
            if part.get_filename():
                if name != "image" or part.get_content_type() != "image/png":
                    raise AssertionError("original upload image field differs")
                png = part.get_payload(decode=True)
            else:
                fields[name] = part.get_payload(decode=True).decode()
        if png is None or fields != {"album": "local-test"} or self.headers["Authorization"] != "Bearer isolated-token":
            raise AssertionError("upload PNG, fields or auth did not reach actual server")
        name = self.path.strip("/")
        destination = self.server.root / f"{name}-received.png"
        destination.write_bytes(png)
        self.server.records.put({"path": self.path, "file": str(destination), "bytes": len(png), "fields": fields})
        if name in ("cancel", "timeout"):
            self.server.release.wait(8)
        payload = json.dumps({"url": "https://images.example/native.png" if name == "success" else ""}).encode()
        try:
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)
        except (BrokenPipeError, ConnectionResetError):
            pass


def configure(path, endpoint, timeout=60000):
    """【上传回归】【独立配置】把 endpoint、timeout 写入本次 path，保持原版外框与默认浅色，所有凭据均为测试值"""
    path.write_text('[general]\noverlay="window"\nto=""\n[notifications]\nenabled=false\n[keys]\nupload="F8"\n' + EXPORT.FRAME +
                    f'\n[upload]\ntimeout_ms={timeout}\n[upload.env]\nMARK_SHOT_UPLOAD_URL="{endpoint}"\nMARK_SHOT_UPLOAD_API_KEY="isolated-token"\nMARK_SHOT_UPLOAD_FIELD_album="local-test"\n')


def geometry():
    """【上传回归】【真实输出】读取当前私有输出，返回实际渲染逻辑尺寸，不从应用日志估算坐标"""
    metadata = json.loads((Path(session.environment()["XDG_RUNTIME_DIR"]) / "runtime.json").read_text())
    output = next(iter(metadata["outputs"].values()))
    if session.backend() is session.sway:
        return math.ceil(1280 / output["logical"]["scale"]), math.ceil(800 / output["logical"]["scale"])
    mode = output["modes"][output["current_mode"]]
    return math.ceil(mode["width"] / output["logical"]["scale"]), math.ceil(mode["height"] / output["logical"]["scale"])


def upload_point(bounds):
    """【上传回归】【按钮位置】按原版默认工具栏规则返回上传按钮中心，bounds 为实际逻辑窗口大小"""
    text = (PROJECT / "src/ui/toolbar/items.rs").read_text().split("pub const FULLSCREEN_ITEMS:", 1)[1].split("];", 1)[0]
    items = [line.strip() for line in text.splitlines() if line.strip() == "S," or line.strip().startswith("I(")]
    along, across, longest, point = 12, 0, 12, None
    limit = bounds[0] - 28
    for item in items:
        length = 1 if item == "S," else 30
        if along + length > limit and along > 12:
            longest = max(longest, along - 2)
            along, across = 0, across + 32
        if "Integration::Upload" in item:
            point = (6 + along + length / 2, 6 + across + 15)
        along += length + 2
    width = max(longest, along - 2) + 12
    if point is None:
        raise AssertionError("upload toolbar entry is missing")
    return (bounds[0] - width) / 2 + point[0], 12 + point[1]


def frame(root, name):
    """【上传回归】【原始截图】保存 root/name 的完整当前画面，返回 RGB 图像"""
    path = root / f"{name}.png"
    time.sleep(0.12)
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def theme_frame(root, name, config, mode, probe, started):
    """【上传回归】【主题完成】等待 config 中 mode 和 probe 实际画面同时更新，root/name 保存原图，返回画面与观察耗时；不重复发送快捷键"""
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        image = frame(root, name)
        color = image.getpixel(probe)
        persisted = tomllib.loads(config.read_text()).get("ui", {}).get("theme")
        expected = (236, 238, 240) if mode == "light" else (15, 16, 22)
        if persisted == mode and color == expected:
            return image, round((time.monotonic() - started) * 1000, 3)
    raise AssertionError(f"theme action did not finish with both persisted {mode} and original pixel {expected}: persisted={persisted}, color={color}")


def move(point):
    """【上传回归】【持久指针】把 point 交给本次经过校验的设备，无返回值"""
    pointer, logical, _, _ = CURRENT.get()
    coordinates = transform_for_pointer(point[0], point[1], logical["width"], logical["height"], logical.get("pointer_transform", logical["transform"]))
    pointer.move_abs(*coordinates)
    time.sleep(0.06)


def annotate(bounds):
    """【上传回归】【实际标注】在 bounds 中央小原图创建矩形，输入使用真实工具快捷键与拖动，无返回值"""
    session.key("R")
    pointer = CURRENT.get()[0]
    move((bounds[0] / 2 - 12, bounds[1] / 2 - 7))
    pointer.press(BTN_LEFT)
    try:
        move((bounds[0] / 2 + 10, bounds[1] / 2 + 6))
    finally:
        pointer.release(BTN_LEFT)
    session.key("S")
    move((bounds[0] - 50, bounds[1] - 40))


def wait_log(path, value, pid):
    """【上传回归】【完成事实】等待 path 中 pid、value 对应实际任务记录，无结果或超时直接失败，不接受其他截图进程日志"""
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if any(f"pid={pid} " in line and value in line for line in path.read_text().splitlines()):
            return
        time.sleep(0.04)
    raise AssertionError(f"upload task did not reach {value}: {path}")


def idle(process, log):
    """【上传回归】【静止资源】检查 process 在无输入一秒内的线程、CPU 和 log 帧提交，返回实际采样"""
    session.wait_single_thread(process.pid)
    first = session.resources(process.pid)
    damage = log.read_text().count(".damage_buffer(")
    time.sleep(1)
    second = session.resources(process.pid)
    if second["threads"] != 1 or second["cpu_ticks"] != first["cpu_ticks"] or damage != log.read_text().count(".damage_buffer("):
        raise AssertionError(f"idle upload editor keeps polling: {first} -> {second}")
    return {"before": first, "after": second, "damage": damage}


def run(binary, root):
    """【上传回归】【完整流程】用 binary 在私有 Wayland 执行上传与恢复，root 保存实际图片、输入、资源和二进制指纹"""
    environment = session.environment()
    root.mkdir(parents=True, exist_ok=False)
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"),
                       XDG_STATE_HOME=str(root / "state"), TMPDIR=str(root / "temporary"), MARK_SHOT_LANG="english", MARK_SHOT_LOG="info")
    for key in list(environment):
        if key.startswith("MARK_SHOT_UPLOAD_"):
            environment.pop(key)
    config = root / "config/mark-shot/config.toml"
    config.parent.mkdir(parents=True)
    Path(environment["TMPDIR"]).mkdir()
    server = UploadServer(root)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    endpoint = f"http://127.0.0.1:{server.server_port}"
    report = {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "compositor": session.backend().__name__}
    try:
        with session.input_context(), pointer_context():
            if session.backend() is session.sway:
                session.sway.keyboard_ready()
            bounds = geometry()
            configure(config, endpoint + "/success")
            process, log = EXPORT.launch(binary, environment, root, "success", "")
            try:
                EXPORT.ready(process, environment)
                before = frame(root, "before-right")
                probe = (round(upload_point(bounds)[0]), 16)
                light_pixel = before.getpixel(probe)
                started = time.monotonic()
                session.key("ctrl+shift+l")
                dark, dark_ms = theme_frame(root, "dark-theme", config, "dark", probe, started)
                dark_pixel = dark.getpixel(probe)
                if sum(light_pixel) < 600 or sum(dark_pixel) > 200:
                    raise AssertionError(f"default light or actual dark theme failed: {light_pixel} -> {dark_pixel}")
                started = time.monotonic()
                session.key("ctrl+shift+l")
                before, light_ms = theme_frame(root, "restored-light-theme", config, "light", probe, started)
                persisted = tomllib.loads(config.read_text())
                if persisted["ui"]["theme"] != "light" or persisted["upload"]["env"]["MARK_SHOT_UPLOAD_URL"] != endpoint + "/success":
                    raise AssertionError("theme switch did not persist or removed upload configuration")
                report["theme"] = {"light_pixel": light_pixel, "dark_pixel": dark_pixel, "persisted": "light", "observed_dark_ms": dark_ms, "observed_light_ms": light_ms}
                click(upload_point(bounds), "right")
                right = frame(root, "after-right")
                if server.records.qsize() or process.poll() is not None:
                    raise AssertionError("right button started upload or closed screenshot")
                if ImageChops.difference(before, right).getbbox() is None:
                    raise AssertionError("right pointer did not reach enabled upload button")
                click(upload_point(bounds))
                receipt = server.records.get(timeout=5)
                EXPORT.finish(process)
                actual = subprocess.check_output(["wl-paste", "--no-newline", "--type", "text/plain"], env=environment, timeout=3).decode()
                if actual != "https://images.example/native.png":
                    raise AssertionError(f"uploaded URL was not copied persistently: {actual}")
                report["success"] = dict(receipt, pixels=EXPORT.equal(Path(receipt["file"]).read_bytes(), (EXPORT.FIXTURES / "case-17-expected.png").read_bytes(), "uploaded original framed PNG"))
                if list((root / "success").glob("*.png")) or EXPORT.history(environment):
                    raise AssertionError("upload also saved screenshot or added duplicate history")
            finally:
                EXPORT.close(process, log)
            # 1. 【上传回归】【失败恢复】真实图床响应失败后原图和标注保持可编辑，保存与已交付图片逐像素相同
            for name in ("failure", "cancel", "timeout"):
                server.release.clear()
                configure(config, endpoint + "/" + name, 1000 if name == "timeout" else 60000)
                process, log = EXPORT.launch(binary, environment, root, name, "")
                try:
                    EXPORT.ready(process, environment)
                    annotate(bounds)
                    before = frame(root, name + "-before")
                    session.key("F8")
                    receipt = server.records.get(timeout=5)
                    if name == "cancel":
                        session.key("Escape")
                    expected_log = "【图片上传】【取消完成】" if name == "cancel" else "【图片上传】【任务失败】Upload timed out" if name == "timeout" else "【图片上传】【任务失败】Upload failed"
                    wait_log(root / "state/mark-shot/mark-shot.log", expected_log, process.pid)
                    server.release.set()
                    time.sleep(4.5)
                    if process.poll() is not None:
                        raise AssertionError(f"{name} closed screenshot instead of restoring editor")
                    restored = frame(root, name + "-restored")
                    if ImageChops.difference(before, restored).getbbox() is not None:
                        raise AssertionError(f"{name} did not restore unchanged editor pixels after feedback expired")
                    report[name] = dict(receipt, idle=idle(process, root / f"{name}.log"))
                    session.key("ctrl+s")
                    EXPORT.finish(process)
                    saved = list((root / name).glob("*.png"))
                    if len(saved) != 1:
                        raise AssertionError(f"{name} did not retain actual editable screenshot")
                    report[name]["saved_pixels"] = EXPORT.equal(saved[0].read_bytes(), Path(receipt["file"]).read_bytes(), f"same annotated PNG after {name}")
                    if Image.open(io.BytesIO(Path(receipt["file"]).read_bytes())).convert("RGBA").tobytes() == Image.open(EXPORT.FIXTURES / "case-17-expected.png").convert("RGBA").tobytes():
                        raise AssertionError("upload did not include actual rectangle annotation")
                finally:
                    EXPORT.close(process, log)
    finally:
        server.release.set()
        server.shutdown()
        server.server_close()
        thread.join(timeout=3)
    if list(Path(environment["TMPDIR"]).glob("*.png")) or (root / "data/mark-shot/models").exists():
        raise AssertionError("native upload leaked temporary PNG or initialized OCR models")
    (root / "result.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return report


def main():
    """【上传回归】【命令入口】解析 binary、root 并执行已验证私有会话中的实际上传流程，返回完整报告"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve()), ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
