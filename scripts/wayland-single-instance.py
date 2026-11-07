#!/usr/bin/env python3
"""【单实例原生回归】【完整链路】重复启动、延迟替换、活动保护、辅助窗口与失效路径恢复"""

import argparse
import hashlib
import json
import time
from pathlib import Path

import wayland_single_instance_checks as checks
import wayland_single_instance_session as runtime
import wayland_text_format_session as session
from wayland_scroll_raw_capture import capture as raw_capture
from wayland_text_pointer import pointer_context


def run(binary, root):
    """【单实例原生回归】【场景调度】在当前私有会话使用固定 binary，root 保存完整证据，返回场景及构建结果"""
    root.mkdir(parents=True, exist_ok=False, mode=0o700)
    private = session.environment()
    metadata = json.loads((Path(private["XDG_RUNTIME_DIR"]) / "runtime.json").read_text())
    compositor = metadata.get("compositor", "niri")
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    reports = []
    with session.input_context(), pointer_context(backend="virtual" if compositor == "sway" else "x11"):
        if compositor == "sway":
            session.sway.keyboard_ready()
        with runtime.launch(binary, root, "active") as first:
            first.wait_present()
            path = runtime.wait_endpoint(first)
            before = checks.idle(first)
            with runtime.launch(binary, root, "duplicate", ["--fullscreen", "--all-outputs"]) as second:
                elapsed = checks.forwarded(second)
            after = checks.idle(first)
            assert before[-1]["commits"] == after[0]["commits"], "duplicate request repainted or replaced the active capture"
            reports.append({"name": "active-duplicate", "forward_seconds": elapsed, "no_extra_surface": True, "idle": after})
            with runtime.launch(binary, root, "active-delay", ["--delay", "0.05"]) as second:
                elapsed = checks.forwarded(second)
            time.sleep(0.12)
            after_delay = checks.idle(first)
            assert after[-1]["commits"] == after_delay[0]["commits"], "due duplicate created or changed the existing capture"
            reports.append({"name": "active-delayed-request", "forward_seconds": elapsed, "active_capture_preserved": True, "idle": after_delay})
            replies = [checks.packet(path, b'{"capture":false}\n'),
                       checks.packet(path, b'{"capture":true,"recordingStatus":true}\n', expected=False),
                       checks.packet(path, b'not-json\n', expected=False),
                       checks.packet(path, b' ' * (16 * 1024 + 1), expected=False)]
            expiry = checks.expired_partial(path)
            after_requests = checks.idle(first)
            assert after_delay[-1]["commits"] == after_requests[0]["commits"]
            assert before[-1]["fds"] == after_requests[0]["fds"]
            reports.append({"name": "bounded-protocol", "responses": replies, "partial_closed_seconds": expiry, "no_business_repaint": True, "idle": after_requests})
            bounded = checks.bounded_clients(path, first)
            after_bounded = checks.idle(first)
            assert after_requests[-1]["commits"] == after_bounded[0]["commits"]
            reports.append({"name": "bounded-clients", **bounded, "no_business_repaint": True, "idle": after_bounded})
            first.cancel()
            assert not path.exists(), "normal cancellation retained the socket path"
        for label, replacement in [("shorter", ["--delay", "0.25", "--fullscreen"]), ("immediate", [])]:
            original = raw_capture().convert("RGBA")
            with runtime.launch(binary, root, f"waiting-{label}", ["--delay", "1.5"]) as first:
                path = runtime.wait_endpoint(first)
                time.sleep(0.08)
                assert ".damage_buffer(" not in first.log.read_text(), "waiting capture presented before its deadline"
                requested = time.monotonic()
                with runtime.launch(binary, root, f"replace-{label}", replacement) as second:
                    checks.forwarded(second)
                presented = first.wait_present()
                elapsed = presented - requested
                assert 0.15 < elapsed < 0.9 if label == "shorter" else elapsed < 0.7, elapsed
                resources = checks.idle(first)
                report = {"name": f"delay-{label}", "present_seconds_after_request": elapsed, "replacement_applied": True, "idle": resources}
                if label == "shorter":
                    report["fullscreen_png"] = checks.finish(first, original)
                else:
                    first.cancel()
                reports.append(report)
                assert not path.exists()
        with runtime.launch(binary, root, "waiting-killed", ["--delay", "1.5"]) as first:
            path = runtime.wait_endpoint(first)
            first.process.terminate()
            first.process.wait(timeout=3)
        with runtime.launch(binary, root, "stale-recovered") as first:
            first.wait_present()
            response = checks.packet(runtime.wait_endpoint(first), b'{"capture":false}\n')
            reports.append({"name": "stale-recovery", "recovered": response["handled"]})
            first.cancel()
        with runtime.launch(binary, root, "save-as") as first:
            first.wait_present()
            checks.idle(first)
            checks.select_region(first)
            session.key("ctrl+alt+s")
            first.wait_text("mark-shot-save-as")
            modal_before = checks.idle(first)
            with runtime.launch(binary, root, "dialog-duplicate") as second:
                elapsed = checks.forwarded(second)
            modal_after = checks.idle(first)
            assert modal_before[-1]["commits"] == modal_after[0]["commits"], "forwarding changed the save dialog"
            bounded = checks.bounded_clients(runtime.wait_endpoint(first), first, copies=2)
            session.key("Escape")
            first.wait_text("已取消，恢复原截图")
            restored = checks.idle(first)
            reports.append({"name": "save-as-duplicate", "forward_seconds": elapsed, "modal_forwarding": True, "capture_restored": True,
                            "modal_idle": modal_after, "restored_idle": restored})
            reports[-1]["bounded_clients"] = bounded
            first.cancel()
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == fingerprint
    result = {"binary_sha256": fingerprint, "compositor": compositor,
              "logical": next(iter(metadata["outputs"].values()))["logical"], "cases": reports}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    """【单实例原生回归】【命令入口】解析 binary 与新 root，完成私有验证后输出场景摘要，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    result = run(options.binary.resolve(), options.root.resolve())
    print(json.dumps({"compositor": result["compositor"], "binary_sha256": result["binary_sha256"],
                      "cases": [case["name"] for case in result["cases"]]}, indent=2))


if __name__ == "__main__":
    main()
