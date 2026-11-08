#!/usr/bin/env python3
"""【交互性能】【实际提交】在私有桌面测量连续输入期间的主表面帧率，不把输入次数当作帧数"""

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import statistics
import subprocess
import time

from PIL import ImageChops

import wayland_text_format_session as session
from wayland_text_pointer import pointer_context
from wayland_frame_buffer_contract import inspect
from wayland_memory_metrics import original_source_files


SPEC = importlib.util.spec_from_file_location("memory_baseline", Path(__file__).with_name("wayland-memory-baseline.py"))
BASELINE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BASELINE)


def submissions(text, segment, size=(3840, 2160)):
    """【交互性能】【帧日志】从text识别size指定的主缓冲，返回segment中的提交毫秒时间；兼容两种Wayland日志格式"""
    buffers = set(re.findall(rf"create_buffer\((?:new id )?wl_buffer([@#]\d+),\s*\d+,\s*{size[0]},\s*{size[1]},", text))
    result = []
    for stamp, buffer in re.findall(r"\[\s*([\d.:]+)\].*wl_surface[@#]\d+\.attach\(wl_buffer([@#]\d+),", segment):
        if buffer not in buffers:
            continue
        if ":" in stamp:
            hour, minute, second = map(float, stamp.split(":"))
            result.append((hour * 3600 + minute * 60 + second) * 1000)
        else:
            result.append(float(stamp))
    return result


def samples(binary, implementation, root, seconds, scenarios, image=None, size=(3840, 2160)):
    """【交互性能】【场景采样】按binary与implementation执行scenarios，root保存日志，seconds指定时长，image指定图片入口，size指定输出尺寸；返回实际提交统计"""
    results = []
    shortcuts = {"rectangle": "r", "pen": "p", "ruler": "u"}
    for scenario in scenarios:
        flags = ["--fullscreen"] if scenario in shortcuts or scenario == "zoom" else []
        if image:
            flags = [str(image)]
        with BASELINE.launch(binary, implementation, root / scenario, flags) as (process, log):
            if image:
                node = BASELINE.SCROLL.wait(
                    lambda: next((node for node in BASELINE.SCROLL.tree(session.environment())
                                  if node.get("pid") == process.pid and node.get("app_id")), None),
                    "image editor did not map")
                BASELINE.SCROLL.command(session.environment(), f'[con_id="{node["id"]}"] focus, fullscreen enable')
            BASELINE.presented(process, log, implementation, size)
            BASELINE.SCROLL.INPUT.move((700 * size[0] / 3840, 500 * size[1] / 2160))
            if scenario in shortcuts:
                session.key(shortcuts[scenario])
            if scenario == "zoom":
                BASELINE.SCROLL.INPUT.click((1900 * size[0] / 3840, 1000 * size[1] / 2160), "right")
            time.sleep(0.5)
            original = BASELINE.raw_capture()
            pointer = BASELINE.SCROLL.INPUT.CURRENT.get()[0]
            if scenario not in ("hover", "zoom"):
                pointer.press()
            offset = log.stat().st_size
            before = session.resources(process.pid)
            started = time.monotonic()
            events = 0
            while time.monotonic() - started < seconds:
                elapsed = time.monotonic() - started
                phase = (elapsed % 2.0) / 2.0
                fraction = phase * 2 if phase < 0.5 else 2 - phase * 2
                if scenario == "zoom":
                    pointer.scroll(dy=0.75 if int(elapsed * 2) % 2 == 0 else -0.75)
                else:
                    pointer.move_abs(900 + 1800 * fraction, 700 + 700 * fraction, 3840, 2160)
                events += 1
                time.sleep(max(0, started + events / 120 - time.monotonic()))
            elapsed = time.monotonic() - started
            after = session.resources(process.pid)
            end = log.stat().st_size
            if scenario not in ("hover", "zoom"):
                pointer.release()
            text = log.read_text()
            segment = log.read_bytes()[offset:end].decode(errors="replace")
            frames = submissions(text, segment, size)
            intervals = sorted(right - left for left, right in zip(frames, frames[1:]) if right >= left)
            shown = BASELINE.raw_capture()
            shown.save(root / scenario / "final.png")
            changed = ImageChops.difference(original, shown).getbbox() is not None
            contract = inspect(text)
            measured_contract = inspect(text, minimum_pixels=size[0] * size[1])
            result = dict(scenario=scenario, seconds=elapsed, inputs=events, frames=len(frames),
                          fps=len(frames) / elapsed, cpu_ticks=after["cpu_ticks"] - before["cpu_ticks"],
                          rss_kib=after["rss_kib"], median_ms=statistics.median(intervals) if intervals else None,
                          p95_ms=intervals[min(len(intervals) - 1, int(len(intervals) * 0.95))] if intervals else None,
                          changed=changed, buffer_contract=contract,
                          measured_buffer_contract=measured_contract)
            if image:
                result["active_originals"] = original_source_files(process.pid)
                time.sleep(1.0)
                result["idle_rss_kib"] = session.resources(process.pid)["rss_kib"]
                result["idle_originals"] = original_source_files(process.pid)
                result["idle_original_reclaimed"] = sum(
                    source["resident_file_kib"] for source in result["idle_originals"]
                    if source["kind"] == "original") <= 4096
            results.append(result)
            if image:
                BASELINE.SCROLL.command(session.environment(), f'[con_id="{node["id"]}"] kill')
            else:
                session.key("Escape")
            try:
                process.wait(timeout=5)
                result["closed_promptly"] = True
            except subprocess.TimeoutExpired:
                result["closed_promptly"] = False
                process.terminate()
                process.wait(timeout=3)
    return results


def main():
    """【交互性能】【测量入口】解析二进制、结果目录和帧率门槛；记录指纹后执行场景，返回是否通过"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--implementation", choices=("rust", "qt"), default="rust")
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--image", type=Path)
    parser.add_argument("--width", type=int, default=3840)
    parser.add_argument("--height", type=int, default=2160)
    parser.add_argument("--seconds", type=float, default=3)
    parser.add_argument("--min-fps", type=float, default=45)
    parser.add_argument("--scenarios", nargs="+", choices=("hover", "selection", "rectangle", "pen", "ruler", "zoom"),
                        default=("hover", "selection", "rectangle", "pen", "ruler"))
    options = parser.parse_args()
    if options.image and options.scenarios != ["zoom"]:
        parser.error("--image requires --scenarios zoom")
    root = options.root.resolve()
    root.mkdir(parents=True, exist_ok=False)
    binary = options.binary.resolve()
    with session.input_context(), pointer_context():
        session.sway.keyboard_ready()
        report = dict(binary=str(binary), sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                      image=str(options.image.resolve()) if options.image else None,
                      size=[options.width, options.height],
                      scenarios=samples(binary, options.implementation, root, options.seconds, options.scenarios,
                                        options.image.resolve() if options.image else None,
                                        (options.width, options.height)))
    report["passed"] = all(item["fps"] >= options.min_fps and item["changed"]
                           and item["buffer_contract"]["safe"]
                           and item["measured_buffer_contract"]["maximum_buffers_per_surface"] <= 2
                           and item.get("idle_original_reclaimed", True)
                           and item.get("closed_promptly", False)
                           for item in report["scenarios"])
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
