#!/usr/bin/env python3
"""【窗口捕获回归】【私有 niri】验证精确窗口、透明度、焦点、指针与剪贴板边界"""

import argparse
from contextlib import contextmanager
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import time

from PIL import Image, ImageChops


def digest(path):
    """【窗口捕获回归】【文件指纹】按 path 计算固定构建 SHA-256，返回十六进制字符串"""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def environment():
    """【窗口捕获回归】【会话校验】检查当前环境与运行器身份一致，无参数；返回仅指向私有会话的环境"""
    result = dict(os.environ)
    root = Path(result["XDG_RUNTIME_DIR"]).resolve()
    state = json.loads((root / "runtime.json").read_text())
    assert result["NIRI_SOCKET"] == state["niri_socket"]
    assert result["WAYLAND_DISPLAY"] == state["wayland_display"]
    assert result["DISPLAY"] == state["x11_display"]
    assert Path(state["niri_socket"]).parent == root
    assert Path(f'/proc/{state["niri_pid"]}/cmdline').read_bytes().split(b"\0")[0].endswith(b"niri")
    compositor_environment = Path(f'/proc/{state["niri_pid"]}/environ').read_bytes().split(b"\0")
    assert any(value.startswith(b"DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent/") for value in compositor_environment), "private screenshot tests require a disconnected compositor notification bus"
    result.update(DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/window-capture-test")
    result.pop("MARK_SHOT_CAPTURE", None)
    return result


def niri(env, *arguments):
    """【窗口捕获回归】【私有请求】仅向 env 指定合成器执行 arguments，返回 JSON 或空结果"""
    result = subprocess.check_output(["niri", "msg", "-j", *arguments], env=env, timeout=3)
    return json.loads(result) if result.strip() else None


def wait_window(env, identity, trace):
    """【窗口捕获回归】【窗口等待】按 identity 与 trace 等待本次测试来源，env 指向私有会话；返回窗口信息"""
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        matching = [window for window in niri(env, "windows") if window["app_id"] == f"mark-shot-window-capture-target-{identity}"]
        if len(matching) == 1 and trace.exists():
            return matching[0]
        time.sleep(0.02)
    raise AssertionError("private capture target did not map")


@contextmanager
def targets(binary, env, root):
    """【窗口捕获回归】【来源生命周期】用 binary 在 env 建立两个身份窗口，root 记录原图尺寸；退出关闭本次来源"""
    processes, logs, windows, traces = [], [], [], []
    try:
        for identity in (0, 1):
            trace = root / f"source-{identity}.json"
            log = (root / f"source-{identity}.log").open("w")
            logs.append(log)
            process = subprocess.Popen([str(binary), str(identity), str(trace)], env=env, stdout=subprocess.DEVNULL, stderr=log)
            processes.append(process)
            window = wait_window(env, identity, trace)
            windows.append(window)
            traces.append(trace)
        yield processes, windows, traces
    finally:
        for process in processes:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)
        for log in logs:
            log.close()


def focus(env, window):
    """【窗口捕获回归】【焦点切换】聚焦 env 中 window 的稳定 ID 并核对实际焦点，无返回值"""
    niri(env, "action", "focus-window", "--id", str(window["id"]))
    current = niri(env, "focused-window")
    assert current and current["id"] == window["id"]


def capture(binary, env, root, name, extra=(), to="co"):
    """【窗口捕获回归】【生产入口】按 binary、env 与 extra 执行窗口截图，root 保存 name 证据；返回进程结果"""
    config = root / "config.toml"
    process = subprocess.run([str(binary), "--window", "--to", to, "--config", str(config), "--nonotify", *extra], env=env, capture_output=True, timeout=8)
    (root / f"{name}.stderr").write_bytes(process.stderr)
    (root / f"{name}.stdout").write_bytes(process.stdout)
    assert not list(Path(env["TMPDIR"]).iterdir()), f"{name}: temporary screenshot was retained"
    return process


def expected(trace):
    """【窗口捕获回归】【独立原图】由 trace 重建来源窗口完整预乘 RGBA，返回尺寸与期望字节"""
    data = json.loads(trace.read_text())
    width, height = data["width"], data["height"]
    color = (224, 96, 32) if data["identity"] == 0 else (32, 160, 224)
    rows = []
    for bottom in (False, True):
        row = bytearray()
        for x in range(width):
            alpha = [255, 128, 64, 0][int(bottom) * 2 + int(x >= width // 2)]
            row.extend([*(channel * alpha // 255 for channel in color), alpha])
        rows.append(bytes(row))
    pixels = rows[0] * (height // 2) + rows[1] * (height - height // 2)
    return (width, height), pixels


def premultiplied(image):
    """【窗口捕获回归】【导出颜色】按 image 返回预乘 RGBA 字节，比较所有颜色与透明度，不忽略透明区域"""
    pixels = bytearray(image.convert("RGBA").tobytes())
    for index in range(0, len(pixels), 4):
        alpha = pixels[index + 3]
        for channel in range(3):
            pixels[index + channel] = (pixels[index + channel] * alpha + 127) // 255
    return bytes(pixels)


def verify_image(process, trace, name):
    """【窗口捕获回归】【完整像素】核对 process 成功截图与 trace 的原始尺寸及全部像素，name 标识失败；返回图像"""
    assert process.returncode == 0, f"{name}: {process.stderr.decode()}"
    image = Image.open(io.BytesIO(process.stdout)).convert("RGBA")
    size, pixels = expected(trace)
    assert image.size == size, f"{name}: {image.size} != {size}"
    actual = premultiplied(image)
    differences = sum(left != right for left, right in zip(actual, pixels))
    assert actual == pixels, f"{name}: {differences} changed RGBA bytes"
    return image


@contextmanager
def clipboard(env, text):
    """【窗口捕获回归】【剪贴板来源】在 env 的私有座席拥有 text，退出回收本次拥有进程；返回来源句柄"""
    process = subprocess.Popen(["wl-copy", "--foreground", "--type", "text/plain"], env=env, stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        process.stdin.write(text.encode())
        process.stdin.close()
        deadline = time.monotonic() + 2
        while time.monotonic() < deadline:
            result = subprocess.run(["wl-paste", "--no-newline", "--type", "text/plain"], env=env, capture_output=True, timeout=2)
            if result.returncode == 0 and result.stdout == text.encode():
                break
            time.sleep(0.02)
        else:
            raise AssertionError("private clipboard sentinel did not become available")
        yield process
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=3)


def rejected(binary, env, root, name, to, message, extra=()):
    """【窗口捕获回归】【拒绝边界】按 name 检查 binary 拒绝 to 且保持剪贴板，message 为预期原因，extra 为附加参数；返回错误证据"""
    sentinel = f"preserve clipboard for {name}"
    with clipboard(env, sentinel):
        process = capture(binary, env, root, name, extra, to)
        assert process.returncode != 0 and not process.stdout
        assert message in process.stderr.decode(), process.stderr.decode()
        remaining = subprocess.check_output(["wl-paste", "--no-newline", "--type", "text/plain"], env=env, timeout=2)
        assert remaining == sentinel.encode(), f"{name}: an unrequested capture changed the clipboard"
    return {"name": name, "returncode": process.returncode, "clipboard_preserved": True}


def run(binary, target, baseline, root):
    """【窗口捕获回归】【完整矩阵】在已认证私有会话运行固定 binary 与 target，baseline 复现旧行为，root 保存新证据；返回结果"""
    root.mkdir(parents=True, exist_ok=False)
    env = environment()
    env.update(TMPDIR=str(root / "temporary"), XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    Path(env["TMPDIR"]).mkdir()
    (root / "config.toml").write_text('[general]\nto="c"\n[notifications]\nenabled=false\n[history]\nenabled=false\n')
    fingerprint, target_fingerprint = digest(binary), digest(target)
    cases = []
    with targets(target, env, root) as (processes, windows, traces):
        focus(env, windows[0])
        if baseline:
            old = capture(baseline, env, root, "old-baseline")
            assert old.returncode != 0 and not old.stdout
            cases.append({"name": "old-baseline", "returncode": old.returncode, "binary_sha256": digest(baseline)})
        for identity in (0, 1):
            focus(env, windows[identity])
            name = f"window-{identity}"
            process = capture(binary, env, root, name)
            image = verify_image(process, traces[identity], name)
            data = subprocess.check_output(["wl-paste", "--type", "image/png"], env=env, timeout=3)
            assert premultiplied(Image.open(io.BytesIO(data))) == premultiplied(image)
            cases.append({"name": name, "window_id": windows[identity]["id"], "size": image.size, "rgba_bytes": len(premultiplied(image)), "all_pixels_equal": True})
        # 1. 【窗口捕获回归】【延迟焦点】延迟期间切换窗口，捕获时间的真实活动身份决定最终图像
        focus(env, windows[0])
        with (root / "delay.stderr").open("w") as log:
            delayed = subprocess.Popen([str(binary), "--window", "--to", "co", "--delay", "0.6", "--config", str(root / "config.toml"), "--nonotify"], env=env, stdout=subprocess.PIPE, stderr=log)
            try:
                time.sleep(0.15)
                focus(env, windows[1])
                output, _ = delayed.communicate(timeout=8)
                process = subprocess.CompletedProcess([], delayed.returncode, output, (root / "delay.stderr").read_bytes())
                image = verify_image(process, traces[1], "delayed-focus")
                (root / "delay.png").write_bytes(output)
                cases.append({"name": "delayed-focus", "window_id": windows[1]["id"], "size": image.size, "all_pixels_equal": True})
            finally:
                if delayed.poll() is None:
                    delayed.terminate()
                    delayed.wait(timeout=3)
        # 2. 【窗口捕获回归】【指针职责】将来源改为浮动窗口，私有 X11 指针放在不透明区域
        niri(env, "action", "move-window-to-floating", "--id", str(windows[1]["id"]))
        niri(env, "action", "move-floating-window", "--id", str(windows[1]["id"]), "--x", "100", "--y", "100")
        focus(env, windows[1])
        time.sleep(0.3)
        state = json.loads((Path(env["XDG_RUNTIME_DIR"]) / "runtime.json").read_text())
        logical = state["outputs"]["winit"]["logical"]
        x, y = round(140 * 1280 / logical["width"]), round(140 * 800 / logical["height"])
        subprocess.run(["xdotool", "mousemove", "--sync", str(x), str(y)], env=env, check=True, timeout=3)
        time.sleep(0.1)
        without = verify_image(capture(binary, env, root, "cursor-excluded"), traces[1], "cursor-excluded")
        included = capture(binary, env, root, "cursor-included", ("--include-cursor",))
        assert included.returncode == 0, included.stderr.decode()
        with_cursor = Image.open(io.BytesIO(included.stdout)).convert("RGBA")
        assert with_cursor.size == without.size
        cursor_difference = ImageChops.difference(without, with_cursor).convert("RGB").getbbox()
        assert cursor_difference is not None, "include-cursor did not paint the pointer"
        assert (cursor_difference[2] - cursor_difference[0]) < 80 and (cursor_difference[3] - cursor_difference[1]) < 80
        cases.append({"name": "cursor-options", "size": without.size, "difference_bounds": cursor_difference})
        saved_directory = root / "saved"
        saved_directory.mkdir()
        saved_process = capture(binary, env, root, "save-copy", ("--output", str(saved_directory)), "cso")
        saved_image = verify_image(saved_process, traces[1], "save-copy")
        saved_files = list(saved_directory.glob("*.png"))
        assert len(saved_files) == 1
        assert premultiplied(Image.open(saved_files[0])) == premultiplied(saved_image)
        forced_process = capture(binary, dict(env, MARK_SHOT_CAPTURE="niri-window"), root, "forced-niri")
        verify_image(forced_process, traces[1], "forced-niri")
        cases.append({"name": "save-copy-and-forced-backend", "saved_file": str(saved_files[0]), "all_pixels_equal": True})
        # 3. 【窗口捕获回归】【交付授权】纯标准输出、保存与贴图不能触发合成器隐式复制
        for to in ("o", "s", "p"):
            cases.append(rejected(binary, env, root, f"no-copy-{to}", to, "requires a copy action"))
        wrong = dict(env, NIRI_SOCKET=str(root / "missing.sock"))
        cases.append(rejected(binary, wrong, root, "missing-ipc", "co", "No such file"))
        # 4. 【窗口捕获回归】【无焦点】关闭全部来源后再调用，不能回退为整屏图像
        for process in processes:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)
        deadline = time.monotonic() + 2
        while niri(env, "windows"):
            assert time.monotonic() < deadline
            time.sleep(0.02)
        cases.append(rejected(binary, env, root, "no-focused-window", "co", "no focused window"))
    assert not list(Path(env["TMPDIR"]).iterdir())
    assert digest(binary) == fingerprint and digest(target) == target_fingerprint
    result = {"binary_sha256": fingerprint, "target_sha256": target_fingerprint, "niri_version": subprocess.check_output(["niri", "--version"], text=True).strip(), "runtime": str(Path(env["XDG_RUNTIME_DIR"]) / "runtime.json"), "cases": cases, "temporary_files_removed": True}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    """【窗口捕获回归】【命令入口】解析固定构建、来源和证据目录，执行私有回归；无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--target", type=Path, required=True)
    parser.add_argument("--baseline", type=Path)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.target.resolve(), options.baseline.resolve() if options.baseline else None, options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
