#!/usr/bin/env python3
"""【长图内存】【完整交付】在私有桌面测量默认功能与复制、保存阶段，不以进入编辑器替代完成流程"""

import argparse
from contextlib import contextmanager, nullcontext
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import struct
import time

from wayland_memory_metrics import MemorySampler
from wayland_notification_stub import notification_service
import wayland_text_format_session as session
from wayland_text_pointer import pointer_context

SPEC = importlib.util.spec_from_file_location("baseline", Path(__file__).with_name("wayland-memory-baseline.py"))
BASELINE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BASELINE)


def isolated_processes(root):
    """【长图内存】【独立进程】按私有配置 root 查找已脱离父进程的剪贴板服务，返回 PID 集合，不读取或输出其他环境变量"""
    marker = b"XDG_CONFIG_HOME=" + os.fsencode(root / "config-home")
    result = set()
    for entry in Path("/proc").iterdir():
        if not entry.name.isdecimal():
            continue
        try:
            if marker in (entry / "environ").read_bytes().split(b"\0"):
                result.add(int(entry.name))
        except (FileNotFoundError, ProcessLookupError, PermissionError):
            pass
    return result


@contextmanager
def launch(binary, root, minimal, bus, language, entry):
    """【长图内存】【隔离启动】按 binary、root、minimal、bus、language 和 entry 创建进程，返回进程及日志，结束回收本次进程"""
    root.mkdir()
    environment = dict(session.environment(), XDG_CONFIG_HOME=str(root / "config-home"),
                       XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"),
                       MARK_SHOT_LANG=language, WAYLAND_DEBUG="client",
                       DBUS_SESSION_BUS_ADDRESS=bus or "unix:path=/nonexistent/scroll-stress")
    config = root / "config.toml"
    (root / "saved").mkdir()
    config.write_text('[general]\noverlay="layer"\nto=""\n[notifications]\nenabled=' + str(bool(bus)).lower() + '\n' +
                      ('[history]\nenabled=false\n[region_detection]\nenabled=false\n[window_detection]\nenabled=false\n' if minimal else ''))
    with (root / "client.log").open("w") as log:
        process = subprocess.Popen([str(binary), "--config", str(config), "--output", str(root / "saved")]
                                   + (["-S"] if entry == "direct" else []),
                                   env=environment, stdout=subprocess.DEVNULL, stderr=log)
        try:
            yield process, root / "client.log"
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)


def run(options):
    """【长图内存】【场景执行】按options驱动固定长图并完成指定动作，返回分阶段资源及图像尺寸，保留原始采样"""
    root = options.root.resolve()
    root.mkdir(parents=True, exist_ok=False)
    binary = options.binary.resolve()
    service = notification_service(root / "notify") if options.notifications else nullcontext(None)
    with service as bus, session.input_context(), pointer_context():
        session.sway.keyboard_ready()
        with BASELINE.SCROLL.page(options.page.resolve(), session.environment(), root) as (_, trace):
            with launch(binary, root / "app", options.minimal, bus, options.language, options.entry) as (process, log):
                with MemorySampler(process, root / "app/samples.jsonl", interval=0.015,
                                   related_processes=lambda: isolated_processes(root / "app")) as sampler:
                    BASELINE.presented(process, log)
                    time.sleep(options.startup_wait)
                    BASELINE.SCROLL.INPUT.drag((80, 80), (3280, 1880))
                    if options.entry == "shortcut":
                        time.sleep(0.3)
                        session.key("ctrl+shift+s")
                    time.sleep(0.7)
                    BASELINE.stable(sampler, "scroll-first")
                    BASELINE.raw_capture().save(root / "scroll-first.png")
                    BASELINE.SCROLL.INPUT.move((400, 400))
                    pointer = BASELINE.SCROLL.INPUT.CURRENT.get()[0]
                    sampler.mark("scroll-growth")
                    for index in range(options.steps):
                        assert process.poll() is None, log.read_text()[-2000:]
                        pointer.scroll(dy=10.0)
                        BASELINE.SCROLL.wait(lambda: BASELINE.SCROLL.origin(trace) == (0, (index + 1) * 48), "scroll input lost")
                        time.sleep(0.18)
                    controls = BASELINE.panel_controls(BASELINE.raw_capture(), "rust")
                    BASELINE.SCROLL.INPUT.click(controls["pause"])
                    BASELINE.stable(sampler, "scroll-paused")
                    BASELINE.raw_capture().save(root / "scroll-paused.png")
                    sampler.mark(options.action)
                    controls = BASELINE.panel_controls(BASELINE.raw_capture(), "rust")
                    BASELINE.SCROLL.INPUT.click(controls[options.action])
                    if options.action == "save":
                        BASELINE.SCROLL.wait(lambda: any(node.get("app_id") == "mark-shot-save-as" for node in BASELINE.SCROLL.tree(session.environment())), "save dialog missing")
                        BASELINE.stable(sampler, "save-dialog")
                        sampler.mark("save-confirm")
                        BASELINE.SCROLL.command(session.environment(), '[app_id="mark-shot-save-as"] focus')
                        session.key("Return")
                    assert process.wait(timeout=30) == 0, log.read_text()[-2000:]
                    BASELINE.stable(sampler, "delivered", 0.7)
                    if options.action == "copy":
                        sampler.mark("paste")
                        digests = []
                        for index in range(2):
                            destination = root / f"paste-{index}.png"
                            with destination.open("wb") as stream:
                                subprocess.run(["wl-paste", "--type", "image/png"], env=session.environment(),
                                               stdout=stream, check=True, timeout=30)
                            with destination.open("rb") as stream:
                                digests.append(hashlib.file_digest(stream, "sha256").hexdigest())
                            with destination.open("rb") as stream:
                                assert struct.unpack(">II", stream.read(24)[16:]) == (3200, 1800 + options.steps * 48)
                        assert digests[0] == digests[1], "repeated clipboard reads differ"
                        assert subprocess.check_output(["wl-paste", "--type", "x-kde-force-image-copy"],
                                                       env=session.environment(), timeout=5) == b""
                        subprocess.run(["wl-copy", "--clear"], env=session.environment(), check=True)
                        BASELINE.SCROLL.wait(lambda: not isolated_processes(root / "app"), "clipboard helper did not exit after takeover")
                    BASELINE.stable(sampler, "exited", 0.5)
                report = sampler.report()
    report.update(binary=str(binary), sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                  action=options.action, minimal=options.minimal, notifications=options.notifications,
                  language=options.language, startup_wait=options.startup_wait, entry=options.entry,
                  expected_size=[3200, 1800 + options.steps * 48])
    if options.notifications:
        report["received_notifications"] = [json.loads(line) for line in (root / "notify/notifications.jsonl").read_text().splitlines()]
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    summary = {"peak_mib": report["peak_rss_with_unmapped_original_kib"] / 1024,
               "phases": {name: round(values["peak_rss_with_unmapped_original_kib"] / 1024, 2) for name, values in report["phases"].items()}}
    print(json.dumps(summary, indent=2))
    if options.action == "copy":
        assert summary["phases"]["copy"] <= options.max_copy_mib, "copy memory exceeds budget"
        assert summary["phases"]["delivered"] <= options.max_helper_mib, "clipboard keeps the full PNG in memory"
    assert summary["phases"]["scroll-growth"] <= options.max_scroll_mib, "scrolling memory exceeds budget"
    assert not report["remaining_processes"], "isolated application processes remain"
    return 0


def main():
    """【长图内存】【命令入口】解析二进制、页面、步骤和结果操作，返回场景执行状态，无参数"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--page", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--steps", type=int, default=256)
    parser.add_argument("--action", choices=("copy", "save"), default="copy")
    parser.add_argument("--minimal", action="store_true")
    parser.add_argument("--notifications", action="store_true")
    parser.add_argument("--language", choices=("english", "chinese"), default="chinese")
    parser.add_argument("--startup-wait", type=float, default=3)
    parser.add_argument("--entry", choices=("direct", "shortcut"), default="direct")
    parser.add_argument("--max-scroll-mib", type=float, default=450)
    parser.add_argument("--max-copy-mib", type=float, default=450)
    parser.add_argument("--max-helper-mib", type=float, default=48)
    return run(parser.parse_args())


if __name__ == "__main__":
    raise SystemExit(main())
