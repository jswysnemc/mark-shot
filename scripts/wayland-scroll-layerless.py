#!/usr/bin/env python3
"""【无浮层长截图回归】【真实入口】通过私有协议代理移除Layer Shell，再以真实页面核对应用行为"""

import argparse
import hashlib
import importlib.util
import json
import os
import subprocess
import time
from contextlib import contextmanager
from pathlib import Path

from PIL import ImageChops

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("layerless_scroll_support", PROJECT / "scripts/wayland-scroll-capture.py")
SUPPORT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SUPPORT)


@contextmanager
def proxy(environment, root):
    """【无浮层长截图回归】【代理生命周期】按已验证environment创建本次代理，root保存证据；退出仅终止本次进程"""
    name = "mark-shot-layerless"
    directory = root / "proxy"
    with (root / "proxy.log").open("w") as log:
        process = subprocess.Popen(["python", str(PROJECT / "scripts/wayland_layer_shell_filter.py"), "--name", name, "--root", str(directory)], env=environment, stdout=subprocess.DEVNULL, stderr=log)
        try:
            SUPPORT.wait(lambda: (directory / "ready.json").is_file() or process.poll() is not None, "the private layerless proxy did not start")
            if process.poll() is not None:
                raise RuntimeError("the private layerless proxy exited before readiness")
            yield name
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)


def run(binary, page_binary, root, expect_rejection, steps, large=False):
    """【无浮层长截图回归】【完整入口】以binary和page_binary验证真实协议缺失及steps次位移，root保存原图，expect_rejection记录旧错误"""
    root.mkdir(parents=True, exist_ok=False)
    environment = SUPPORT.session.environment()
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"), MARK_SHOT_LANG="english", MARK_SHOT_LOG="info", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/layerless-test")
    previous = dict(os.environ)
    os.environ.update(environment)
    metadata = json.loads((Path(environment["XDG_RUNTIME_DIR"]) / "runtime.json").read_text())
    logical = metadata["outputs"]["HEADLESS-1"]["logical"]
    region = (8, 8, logical["width"] - 8, logical["height"] - 8) if large else SUPPORT.REGION
    exported = None
    paused_counts = None
    try:
        with SUPPORT.session.input_context(), SUPPORT.FLOW.NAV.SIZE.pointer_context():
            SUPPORT.session.sway.keyboard_ready()
            with proxy(environment, root) as name, SUPPORT.page(page_binary, environment, root) as (_page, trace):
                SUPPORT.INPUT.click((160, 160))
                first = SUPPORT.frame(environment, root, "page-before", True)
                config = root / "scroll.toml"
                config.write_text('[general]\noverlay="auto"\nto="c"\n[notifications]\nenabled=false\n[history]\nenabled=false\n')
                path = root / "scroll.log"
                with path.open("w") as log:
                    process = subprocess.Popen([str(binary), "-S", "--config", str(config)], env=dict(environment, WAYLAND_DISPLAY=name, WAYLAND_DEBUG="client"), stdout=subprocess.DEVNULL, stderr=log)
                    try:
                        SUPPORT.FLOW.wait_input(process, path, region[:2])
                        SUPPORT.INPUT.drag(region[:2], region[2:])
                        if expect_rejection:
                            assert process.wait(timeout=5) != 0, "the old unsupported entry unexpectedly succeeded"
                            assert "scroll capture requires the wlr layer-shell protocol" in path.read_text(), "the actual missing-protocol failure was not reached"
                            restored = SUPPORT.frame(environment, root, "page-restored", True)
                            assert ImageChops.difference(first, restored).getbbox() is None
                        else:
                            SUPPORT.wait(lambda: "【滚动截图】【会话开始】" in path.read_text() or process.poll() is not None, "the layerless scroll entry did not respond")
                            assert process.poll() is None, "layerless scroll capture exited instead of opening its controls"
                            time.sleep(0.2)
                            if large:
                                SUPPORT.wait(lambda: "【滚动截图】【窗口避让】" in path.read_text(), "the overlapping panel was not hidden")
                            live = SUPPORT.crop(SUPPORT.frame(environment, root, "live-page", True), region)
                            assert ImageChops.difference(live, SUPPORT.crop(first, region)).getbbox() is None, "the ordinary scroll window hides or contaminates the live page"
                            if not large:
                                SUPPORT.controls(SUPPORT.frame(environment, root, "controls"))
                            parts = [SUPPORT.crop(first, region)]
                            step = round(48 * parts[0].width / (region[2] - region[0]))
                            for index in range(steps):
                                SUPPORT.scroll(trace, 1)
                                current = SUPPORT.crop(SUPPORT.frame(environment, root, f"page-scrolled-{index}", True), region)
                                parts.append(current.crop((0, current.height - step, current.width, current.height)))
                            if large:
                                def restored_controls():
                                    """【无浮层长截图回归】【静止恢复】等待真实面板恢复，返回按钮位置，仍隐藏时返回空值"""
                                    try:
                                        return SUPPORT.controls(SUPPORT.frame(environment, root, "controls-scrolled"))
                                    except AssertionError:
                                        return None
                                buttons = SUPPORT.wait(restored_controls, "the layerless panel did not return after idle")
                                # 1. 【无浮层长截图回归】【恢复捕获】恢复后必须再次隐藏重叠面板，并继续同一原图而非重新授权或丢失旧内容
                                count = path.read_text().count("【滚动截图】【窗口避让】")
                                SUPPORT.INPUT.click(buttons["pause"])
                                SUPPORT.wait(lambda: path.read_text().count("【滚动截图】【窗口避让】") > count, "resumed layerless capture did not hide the overlapping controls")
                                SUPPORT.scroll(trace, 1)
                                current = SUPPORT.crop(SUPPORT.frame(environment, root, "page-resumed", True), region)
                                parts.append(current.crop((0, current.height - step, current.width, current.height)))
                                buttons = SUPPORT.wait(restored_controls, "the controls did not return after resumed capture settled")
                            else:
                                buttons = SUPPORT.controls(SUPPORT.frame(environment, root, "controls-scrolled"))
                                SUPPORT.INPUT.click(buttons["pause"])
                            paused_counts = SUPPORT.paused_idle(path)
                            buttons = SUPPORT.controls(SUPPORT.frame(environment, root, "controls-paused"))
                            SUPPORT.INPUT.click(buttons["copy"])
                            expected = SUPPORT.joined(parts, False)
                            expected.save(root / "expected.png")
                            exported = SUPPORT.exported(environment, process, root, expected)
                    finally:
                        if process.poll() is None:
                            process.terminate()
                            process.wait(timeout=3)
        evidence = json.loads((root / "proxy/result.json").read_text())
        assert evidence["dropped"] and all(event["interface"] == "zwlr_layer_shell_v1" for event in evidence["dropped"])
        assert evidence["received_fds"] > 0 and evidence["received_fds"] == evidence["sent_fds"]
        assert not evidence["errors"], "the private proxy reported a forwarding failure"
        report = {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "expected_rejection": expect_rejection, "region": region, "steps": steps + int(large), "large": large, "resumed_after_avoidance": large, "export": exported, "paused_counts": paused_counts, "protocol_filter": evidence}
        (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
        return report
    finally:
        os.environ.clear()
        os.environ.update(previous)


def main():
    """【无浮层长截图回归】【命令入口】解析当前binary、page和新root，拒绝模式仅用于保留已知失败，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--page", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--expect-rejection", action="store_true")
    parser.add_argument("--steps", type=int, default=8)
    parser.add_argument("--large", action="store_true")
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.page.resolve(), options.root.resolve(), options.expect_rejection, options.steps, options.large), indent=2))


if __name__ == "__main__":
    main()
