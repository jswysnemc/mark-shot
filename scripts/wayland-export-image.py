#!/usr/bin/env python3
"""【外框回归】【真实导出】在已校验私有 Wayland 验证保存、复制、贴图、历史和另存为，不连接用户桌面"""

import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import time

from PIL import Image
import wayland_text_format_session as session


PROJECT = Path(__file__).resolve().parents[1]
FIXTURES = PROJECT / "tests/fixtures/qt-export-image"
SWAYMSG = PROJECT / "target/test-tools/sway/usr/bin/swaymsg"
FRAME = """
[export.imageFrame]
enabled = true
padding = 8
cornerRadius = 4
shadowRadius = 6
shadowOffsetY = 3
shadowOpacity = 0.45
"""


def windows(environment):
    """【外框回归】【私有窗口】从已校验 environment 读取本次窗口，返回普通窗口身份列表"""
    session.environment()
    if session.backend() is session.niri:
        return session.niri.windows()
    tree = json.loads(subprocess.check_output([str(SWAYMSG), "-r", "-t", "get_tree"], env=environment))
    result = []
    pending = [tree]
    while pending:
        node = pending.pop()
        if node.get("app_id") and node.get("pid"):
            result.append(node)
        pending.extend(node.get("nodes", []) + node.get("floating_nodes", []))
    return result


def wait_window(environment, app_id, pid=None):
    """【外框回归】【窗口等待】等待 environment 中 app_id、pid 对应唯一窗口，返回实际身份，歧义或超时报错"""
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        matches = [node for node in windows(environment) if node["app_id"] == app_id and (pid is None or node["pid"] == pid)]
        if len(matches) == 1:
            return matches[0]
        if len(matches) > 1:
            raise AssertionError(f"ambiguous private window: {app_id}")
        time.sleep(0.04)
    raise AssertionError(f"private window did not appear: {app_id}")


def focus(environment, node):
    """【外框回归】【窗口激活】只激活 environment 中 node 对应私有窗口，无返回值"""
    if session.backend() is session.niri:
        session.niri.control("focus", node["id"])
    else:
        subprocess.run([str(SWAYMSG), f"[con_id={node['id']}] focus"], env=environment,
                       stdout=subprocess.DEVNULL, check=True, timeout=3)
    time.sleep(0.12)


def equal(actual, expected, label):
    """【外框回归】【完整像素】比较 actual、expected 的完整 RGBA，label 标识失败场景，返回物理尺寸"""
    with Image.open(io.BytesIO(actual)) as first, Image.open(io.BytesIO(expected)) as second:
        first, second = first.convert("RGBA"), second.convert("RGBA")
        if first.size != second.size or first.tobytes() != second.tobytes():
            raise AssertionError(f"{label} differs in complete RGBA: {first.size} != {second.size}")
        return list(first.size)


def clipboard(environment, expected, label, root):
    """【外框回归】【持久复制】等待 environment 的 PNG 等于 expected，label 标识场景，root 保存失败像素，返回读取字节"""
    deadline = time.monotonic() + 4
    actual = b""
    while time.monotonic() < deadline:
        result = subprocess.run(["wl-paste", "--type", "image/png"], env=environment,
                                stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=2)
        actual = result.stdout
        if result.returncode == 0:
            try:
                equal(actual, expected, label)
                return actual
            except (AssertionError, OSError):
                pass
        time.sleep(0.05)
    (root / (label.replace(" ", "-") + "-failed.png")).write_bytes(actual)
    equal(actual, expected, label)
    raise AssertionError(f"{label} did not produce the expected private PNG clipboard")


def history(environment):
    """【外框回归】【记录集合】返回 environment 私有数据目录中的实际截图记录，无副作用"""
    return set((Path(environment["XDG_DATA_HOME"]) / "mark-shot/capture-history").glob("capture-*.png"))


def new_record(environment, previous, expected):
    """【外框回归】【单次记录】验证 environment 相对 previous 只增加一条与 expected 完全一致的记录，返回记录路径"""
    created = history(environment) - previous
    if len(created) != 1:
        raise AssertionError(f"export must record exactly once: {created}")
    path = next(iter(created))
    equal(path.read_bytes(), expected, "history image")
    return str(path)


def configuration(path, enabled):
    """【外框回归】【配置写入】只更新本次 path 的外框开关，enabled 决定参数，无返回值"""
    text = '[general]\noverlay="window"\nto=""\n[pinned]\nalways_on_top=false\n[notifications]\nenabled=false\n'
    path.write_text(text + (FRAME if enabled else "\n[export]\nimageFrame=false\n"))


def launch(binary, environment, root, name, outputs):
    """【外框回归】【编辑启动】在 environment 用 binary 打开冻结源图，root、name 分配日志与输出，返回进程和日志句柄"""
    log = (root / f"{name}.log").open("w")
    process = subprocess.Popen([str(binary), "--fullscreen", "--to", outputs,
                                "--output", str(root / name), str(FIXTURES / "case-17-source.png")],
                               env=dict(environment, WAYLAND_DEBUG="client"), stdin=subprocess.DEVNULL,
                               stdout=subprocess.PIPE, stderr=log, start_new_session=True)
    return process, log


def ready(process, environment):
    """【外框回归】【编辑就绪】等待 process 的实际窗口与后台任务结束并取得输入焦点，返回窗口"""
    node = wait_window(environment, "mark-shot", process.pid)
    session.wait_single_thread(process.pid)
    focus(environment, node)
    return node


def history_ready(process, environment, root):
    """【外框回归】【历史资格】把 process 窗口设为原版 800×520 并等待实际 Copy 按钮启用，root 保存画面，返回当前窗口"""
    node = wait_window(environment, "mark-shot-history", process.pid)
    if session.backend() is session.niri:
        if not node["floating"]:
            session.niri.control("float", node["id"])
        session.niri.control("action", "set-window-width", "--id", node["id"], 800)
        session.niri.control("action", "set-window-height", "--id", node["id"], 520)
    else:
        subprocess.run([str(SWAYMSG), f"[con_id={node['id']}] floating enable, resize set width 800 px height 520 px, move position center, focus"],
                       env=environment, check=True, stdout=subprocess.DEVNULL, timeout=3)
    focus(environment, node)
    session.key("Home")
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        node = wait_window(environment, "mark-shot-history", process.pid)
        if session.backend() is session.niri:
            point = node["floating_pos"]
        else:
            point = (node["rect"]["x"], node["rect"]["y"])
        path = root / "history-ready.png"
        session.screenshot(path)
        with Image.open(path) as image:
            if image.convert("RGB").getpixel((round(point[0] + 20), round(point[1] + 52))) == (94, 234, 212):
                return node
        time.sleep(0.05)
    raise AssertionError("the selected history preview did not enable the actual Copy button")


def finish(process):
    """【外框回归】【完成等待】等待 process 的全部标准输出和退出，返回字节，失败报告真实状态"""
    stdout, _ = process.communicate(timeout=8)
    if process.returncode:
        raise AssertionError(f"screenshot process exited with {process.returncode}")
    return stdout


def close(process, log):
    """【外框回归】【进程回收】终止本实例 process 的进程组并关闭 log，不向其他会话发送信号，无返回值"""
    try:
        os.killpg(process.pid, 15)
    except ProcessLookupError:
        pass
    if process.poll() is None:
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, 9)
            process.wait(timeout=3)
    log.close()


def run(binary, root):
    """【外框回归】【完整场景】在已校验私有会话用 binary 完成真实动作，root 保存证据和同一构建指纹，返回验证记录"""
    environment = session.environment()
    root.mkdir(parents=True, exist_ok=False)
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    config = root / "config/mark-shot/config.toml"
    config.parent.mkdir(parents=True)
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    expected = (FIXTURES / "case-17-expected.png").read_bytes()
    result = {"binary_sha256": digest, "compositor": session.backend().__name__}
    with session.input_context():
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        # 1. 【外框回归】【原图基准】使用同一应用的关闭外框输出作为原图交接基准
        configuration(config, False)
        process, log = launch(binary, environment, root, "baseline", "so")
        try:
            ready(process, environment)
            session.key("Return")
            raw = finish(process)
            files = list((root / "baseline").glob("*.png"))
            if len(files) != 1:
                raise AssertionError("disabled export did not save exactly one PNG")
            result["disabled"] = equal(files[0].read_bytes(), raw, "disabled save and stdout")
            (root / "raw.png").write_bytes(raw)
        finally:
            close(process, log)
        # 2. 【外框回归】【共享与原图】编辑期间改动配置不能改变预览，保存和复制带框，标准输出及贴图复制保持原图
        previous = history(environment)
        process, log = launch(binary, environment, root, "combined", "psco")
        try:
            ready(process, environment)
            before = root / "preview-before.png"
            session.screenshot(before)
            configuration(config, True)
            after = root / "preview-after.png"
            time.sleep(0.15)
            session.screenshot(after)
            equal(after.read_bytes(), before.read_bytes(), "frame configuration changed the selection preview")
            time.sleep(0.8)
            first = session.resources(process.pid)
            first_damage = (root / "combined.log").read_text().count(".damage_buffer(")
            time.sleep(1)
            second = session.resources(process.pid)
            second_damage = (root / "combined.log").read_text().count(".damage_buffer(")
            if second["threads"] != 1 or first["cpu_ticks"] != second["cpu_ticks"]:
                raise AssertionError(f"idle export configuration keeps polling: {first} -> {second}")
            if first_damage != second_damage:
                raise AssertionError("idle frame configuration kept submitting surface pixels")
            result["idle"] = {"before": first, "after": second, "surface_damage_before": first_damage, "surface_damage_after": second_damage}
            session.key("Return")
            stdout = finish(process)
            result["stdout_raw"] = equal(stdout, raw, "combined stdout")
            files = list((root / "combined").glob("*.png"))
            if len(files) != 1:
                raise AssertionError("combined export did not save exactly once")
            result["shared"] = equal(files[0].read_bytes(), expected, "framed save")
            (root / "copied.png").write_bytes(clipboard(environment, expected, "framed copy after editor exit", root))
            result["history_record"] = new_record(environment, previous, expected)
            pin = wait_window(environment, "mark-shot-pin")
            focus(environment, pin)
            session.key("ctrl+c")
            (root / "pin-copy.png").write_bytes(clipboard(environment, raw, "raw pin copy", root))
            session.key("Escape")
        finally:
            close(process, log)
        # 3. 【外框回归】【另存为冻结】取消恢复同一画面，确认使用打开时的外框快照，不重复导出或记录
        previous = history(environment)
        process, log = launch(binary, environment, root, "save-as", "")
        try:
            ready(process, environment)
            before = root / "save-as-before.png"
            session.screenshot(before)
            session.key("ctrl+alt+s")
            dialog = wait_window(environment, "mark-shot-save-as", process.pid)
            focus(environment, dialog)
            session.wait_single_thread(process.pid)
            session.key("Escape")
            # 1. 【外框回归】【表面恢复】取消对话框后截图表面重新创建，必须重新读取当前窗口身份
            focus(environment, wait_window(environment, "mark-shot", process.pid))
            after = root / "save-as-canceled.png"
            session.screenshot(after)
            equal(after.read_bytes(), before.read_bytes(), "save-as cancellation changed the frozen editor")
            if history(environment) != previous or list((root / "save-as").glob("*.png")):
                raise AssertionError("save-as cancellation wrote an image or history")
            session.key("ctrl+alt+s")
            dialog = wait_window(environment, "mark-shot-save-as", process.pid)
            focus(environment, dialog)
            session.wait_single_thread(process.pid)
            configuration(config, False)
            session.key("Return")
            finish(process)
            files = list((root / "save-as").glob("*.png"))
            if len(files) != 1:
                raise AssertionError("save-as did not save exactly once")
            result["save_as"] = equal(files[0].read_bytes(), expected, "save-as frozen frame")
            new_record(environment, previous, expected)
        finally:
            close(process, log)
        # 4. 【外框回归】【历史再复制】配置重新启用外框后，历史原图复制不能再次增加边距
        configuration(config, True)
        previous = history(environment)
        log = (root / "history.log").open("w")
        process = subprocess.Popen([str(binary), "--history"], env=dict(environment, WAYLAND_DEBUG="client"), stdout=subprocess.DEVNULL, stderr=log, start_new_session=True)
        try:
            history_ready(process, environment, root)
            session.key("ctrl+c")
            clipboard(environment, expected, "history copy without a second frame", root)
            if history(environment) != previous:
                raise AssertionError("history copy created a duplicate record")
            session.key("Escape")
            process.wait(timeout=5)
        finally:
            close(process, log)
        # 5. 【外框回归】【无界面捕获】原版全桌面和快捷捕获输出不应用外框
        captured = subprocess.run([str(binary), "--full", "--to", "o"], env=environment,
                                  stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True, timeout=8)
        with Image.open(io.BytesIO(captured.stdout)) as image:
            if image.size != (1280, 800):
                raise AssertionError(f"headless full capture unexpectedly gained padding: {image.size}")
        quick = root / "quick.png"
        subprocess.run([str(binary), "--capture-to", str(quick)], env=environment,
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True, timeout=8)
        with Image.open(quick) as image:
            if image.size != (1280, 800):
                raise AssertionError(f"quick capture unexpectedly gained padding: {image.size}")
        result["headless"] = [1280, 800]
    if (root / "data/mark-shot/models").exists():
        raise AssertionError("image frame initialized OCR models")
    if hashlib.sha256(binary.read_bytes()).hexdigest() != digest:
        raise AssertionError("the tested binary changed while scenarios were running")
    (root / "report.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    """【外框回归】【命令入口】解析 binary、root 并执行私有实际场景，返回完整验证结果"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
