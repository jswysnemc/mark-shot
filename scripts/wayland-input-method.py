#!/usr/bin/env python3
"""【输入法回归】【真实投递】只在私有 Wayland 发送输入法批次，核验候选、字节删除、撤销和导出"""

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import tempfile
import time
from pathlib import Path

import wayland_text_format_session as session
from PIL import Image, ImageChops
from wayland_input_method_transport import InputMethod, build, protocol_evidence
from wayland_text_pointer import click, pointer_context
from wayland_text_property_locator import font_button
from wayland_input_method_fields import routing


def capture(process, root, name):
    """【输入法回归】【画面读取】保存 process 的 name 画面到 root，返回 RGB 图像；提前退出即失败"""
    if process.poll() is not None:
        raise AssertionError(f"editor exited: {process.returncode}")
    path = root / f"{name}.png"
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def finish(process, environment, private, root):
    """【输入法回归】【导出完成】触发 process 保存到 private，核验退出和原图尺寸，复制证据到 root；返回图片"""
    session.key("ctrl+s")
    process.wait(timeout=6)
    exports = list((private / "export").glob("*.png"))
    if process.returncode != 0 or len(exports) != 1:
        raise AssertionError(
            f"native input export failed: {process.returncode}, {exports}"
        )
    shutil.copyfile(exports[0], root / "export.png")
    with Image.open(exports[0]) as image:
        if image.size != (1600, 900):
            raise AssertionError(f"native input changed the source size: {image.size}")
        result = image.convert("RGB")
    if (Path(environment["XDG_DATA_HOME"]) / "mark-shot/models").exists():
        raise AssertionError("native text input initialized OCR models")
    return result


def focus(process, shown):
    """【输入法回归】【私有失焦】按 shown 切换 process 所在私有工作区，返回合成器结果；不修改真实布局或窗口尺寸"""
    environment = session.sway.environment()
    binary = str(Path("target/test-tools/sway/usr/bin/swaymsg").resolve())
    tree = json.loads(
        subprocess.check_output(
            [binary, "-r", "-t", "get_tree"], env=environment, text=True
        )
    )
    pending, windows = [(tree, None)], []
    while pending:
        node, workspace = pending.pop()
        if node.get("type") == "workspace":
            workspace = node["name"]
        if node.get("app_id") == "mark-shot":
            windows.append((node, workspace))
        pending.extend((child, workspace) for child in node.get("nodes", []))
        pending.extend((child, workspace) for child in node.get("floating_nodes", []))
    if len(windows) != 1 or process.poll() is not None:
        raise AssertionError(f"private Sway editor identity is ambiguous: {windows}")
    workspace = windows[0][1] if shown else "__mark_shot_ime_focus_test__"
    if workspace is None:
        raise AssertionError("the private editor workspace is unavailable")
    command = f"workspace --no-auto-back-and-forth {json.dumps(workspace)}"
    result = json.loads(
        subprocess.check_output([binary, "-r", command], env=environment, text=True)
    )
    if not result or not all(row.get("success") for row in result):
        raise AssertionError(f"private native focus command failed: {result}")
    return result


def scenario(binary, client, root, overlay, mode):
    """【输入法回归】【场景执行】用 binary、client 与 overlay 执行 mode，root 保存真实协议和像素；返回结果与导出图片"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    with tempfile.TemporaryDirectory(prefix="mark-shot-ime-") as temporary:
        private = Path(temporary)
        config, state = private / "config/mark-shot", private / "state/mark-shot"
        config.mkdir(parents=True)
        state.mkdir(parents=True)
        (config / "config.toml").write_text(
            f'[general]\noverlay = "{overlay}"\n[history]\nenabled = false\n[notifications]\nenabled = false\n'
        )
        (state / "tools.toml").write_text(
            'magnifier = false\nfont_size = 20.0\nfont_size_unit = "pt"\ntext_family = "Inter"\nbold = false\nitalic = false\n'
        )
        environment.update(
            XDG_CONFIG_HOME=str(private / "config"),
            XDG_STATE_HOME=str(private / "state"),
            XDG_DATA_HOME=str(private / "data"),
            WAYLAND_DEBUG="client",
        )
        source = private / "source.png"
        Image.new("RGB", (1600, 900), (30, 34, 38)).save(source)
        method = InputMethod(client, environment, root)
        process = None
        try:
            with (root / "process.log").open("w") as log:
                process = subprocess.Popen(
                    [
                        str(binary),
                        "--fullscreen",
                        "--fullscreen-default-tool",
                        "text",
                        "--default-color",
                        "#FF4D4D",
                        "--to",
                        "s",
                        "--output",
                        str(private / "export"),
                        str(source),
                    ],
                    env=environment,
                    stderr=log,
                    stdout=subprocess.DEVNULL,
                )
                time.sleep(0.3)
                initial = capture(process, root, "initial")
                point = round(initial.width * 0.62), round(initial.height * 0.52)
                region = (point[0] - 5, point[1] - 5, point[0] + 280, point[1] + 90)
                click(point)
                method.state("", active=True)
                time.sleep(0.1)
                baseline = capture(process, root, "empty").crop(region)
                result = {"mode": mode, "overlay": overlay}
                if mode == "delivery":
                    # 1. 【输入法回归】【候选隔离】候选必须实际显示，周边正文不能包含候选
                    method.batch(preedit="ni")
                    time.sleep(0.15)
                    candidate = capture(process, root, "candidate-ni").crop(region)
                    bounds = ImageChops.difference(baseline, candidate).getbbox()
                    if bounds is None or bounds[2] - bounds[0] < 10:
                        raise AssertionError(
                            f"native preedit was not displayed: {bounds}"
                        )
                    method.state("", active=True)
                    method.batch(commit="你", preedit="hao")
                    method.state("你")
                    time.sleep(0.12)
                    capture(process, root, "commit-with-preedit")
                    method.batch(commit="好")
                    method.state("你好")
                    method.batch(commit="𠀀")
                    supplementary = method.state("你好𠀀")
                    if supplementary["cursor"] != 10:
                        raise AssertionError(
                            f"supplementary CJK cursor is not a UTF-8 byte offset: {supplementary}"
                        )
                    method.batch(commit="您", delete=(4, 0))
                    method.state("你好您")
                    # 2. 【输入法回归】【独立历史】原生确认与字节替换分别撤销、重做
                    for text in ("你好𠀀", "你好", "你"):
                        session.key("ctrl+z")
                        method.state(text)
                    for text in ("你好", "你好𠀀", "你好您"):
                        session.key("ctrl+shift+z")
                        method.state(text)
                    session.key("ctrl+a")
                    session.key("ctrl+c")
                    session.wait_clipboard(environment, "你好您")
                    session.key("End")
                    result.update(
                        candidate_bounds=bounds,
                        clipboard="你好您",
                        supplementary_cursor=10,
                        independent_undo=True,
                        byte_deletion=True,
                    )
                else:
                    method.batch(commit="base")
                    method.state("base")
                    if mode == "pending-export":
                        # 3. 【输入法回归】【最终隔离】保存正文时不确认仍在合成的候选
                        method.batch(preedit="pending", cursor=(-1, -1))
                        time.sleep(0.15)
                        capture(process, root, "pending-hidden-caret")
                        method.state("base")
                        session.wait_single_thread(process.pid)
                        before = session.resources(process.pid)
                        frames_before = (
                            (root / "process.log").read_text().count(".damage_buffer(")
                        )
                        time.sleep(1.1)
                        after = session.resources(process.pid)
                        frames_after = (
                            (root / "process.log").read_text().count(".damage_buffer(")
                        )
                        if (
                            before["cpu_ticks"] != after["cpu_ticks"]
                            or frames_before != frames_after
                            or after["threads"] != 1
                        ):
                            raise AssertionError(
                                f"hidden preedit did not stop polling: {before}, {after}, {frames_before}, {frames_after}"
                            )
                        result.update(
                            hidden_caret_no_redraw=True,
                            idle_before=before,
                            idle_after=after,
                        )
                    elif mode == "field-routing":
                        button = font_button(capture(process, root, "fields-before"))
                        result.update(routing(process, method, root, button, region, capture))
                    elif mode == "menu-cancel":
                        method.batch(preedit="pending")
                        time.sleep(0.15)
                        capture(process, root, "preedit-before-menu")
                        click((point[0] + 8, point[1] + 12), "right")
                        method.state(active=False)
                        capture(process, root, "native-menu")
                        session.key("Escape")
                        method.state("base", active=True)
                        time.sleep(0.15)
                        capture(process, root, "after-menu")
                        result["menu_disable_resets_preedit"] = True
                    elif mode == "native-leave":
                        if overlay != "window" or session.backend() is not session.sway:
                            raise AssertionError(
                                "the workspace focus test requires a private Sway window"
                            )
                        method.batch(preedit="pending")
                        time.sleep(0.15)
                        capture(process, root, "preedit-before-focus")
                        focus(process, False)
                        with (root / "auxiliary.log").open("w") as auxiliary_log:
                            auxiliary = subprocess.Popen(
                                [str(binary), "--history"],
                                env=environment,
                                stderr=auxiliary_log,
                                stdout=subprocess.DEVNULL,
                            )
                            try:
                                method.state(active=False)
                                capture(process, root, "focus-out")
                                focus(process, True)
                                method.state("base", active=True)
                                time.sleep(0.15)
                                capture(process, root, "focus-return")
                            finally:
                                if auxiliary.poll() is None:
                                    auxiliary.terminate()
                                    auxiliary.wait(timeout=3)
                        result["native_leave_resets_preedit"] = True
                exported = finish(process, environment, private, root)
                protocol = (root / "process.log").read_text()
                evidence = protocol_evidence(protocol)
                if not evidence["native_protocol"]:
                    raise AssertionError(
                        "production backend did not bind and deliver native text-input-v3"
                    )
                if mode != "baseline" and not evidence["preedit_delivered"]:
                    raise AssertionError(
                        "the compositor did not deliver native preedit events"
                    )
                native_leave = bool(
                    re.search(r"zwp_text_input_v3@\d+\.leave, \(", protocol)
                )
                if mode == "native-leave" and not native_leave:
                    raise AssertionError(
                        "the window did not exercise a real native text-input leave"
                    )
                if mode == "menu-cancel":
                    if "xdg_popup@" not in protocol or ".disable()" not in protocol:
                        raise AssertionError(
                            "the native menu did not disable annotation input"
                        )
                    if session.backend() is session.niri and not native_leave:
                        raise AssertionError(
                            "the private niri popup did not deliver its native leave"
                        )
                    result["menu_native_leave_received"] = native_leave
                result.update(**evidence, export_size=[1600, 900], no_ocr=True)
                return result, exported
        finally:
            if process and process.poll() is None:
                process.terminate()
                process.wait(timeout=3)
            method.close()


def main():
    """【输入法回归】【运行入口】解析发布 binary、证据 root 和协议 xml，校验私有会话后运行全部场景；无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--overlay", choices=("layer", "window"), default="layer")
    parser.add_argument(
        "--xml",
        type=Path,
        default=Path("target/test-tools/input-method/input-method-unstable-v2.xml"),
    )
    args = parser.parse_args()
    session.environment()
    args.root.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve()
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    client = build(args.root / "client", args.xml.resolve())
    result = {"sha256": fingerprint, "bytes": binary.stat().st_size, "scenarios": []}
    with session.input_context(), pointer_context():
        # 1. 【输入法回归】【键盘就绪】无界面 Sway 没有物理键盘，输入法投递前先提供持续键盘能力
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        modes = [
            "delivery",
            "baseline",
            "pending-export",
            "field-routing",
            "menu-cancel",
        ]
        if args.overlay == "window" and session.backend() is session.sway:
            modes.append("native-leave")
        for mode in modes:
            row, exported = scenario(
                binary, client, args.root / mode, args.overlay, mode
            )
            result["scenarios"].append(row)
            if mode == "baseline":
                baseline = exported
            elif mode in (
                "pending-export",
                "field-routing",
                "menu-cancel",
                "native-leave",
            ):
                if ImageChops.difference(baseline, exported).getbbox():
                    raise AssertionError(
                        f"{mode} leaked candidate text or changed committed pixels"
                    )
                row["exact_baseline_export"] = True
    (args.root / "result.json").write_text(
        json.dumps(result, ensure_ascii=False, indent=2) + "\n"
    )
    print(json.dumps(result, ensure_ascii=False))


if __name__ == "__main__":
    main()
