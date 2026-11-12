"""【截图性能】【空闲恢复】私有 Niri 对照连续绘图和空闲回收后的首次局部帧请求延迟。"""

import argparse
import hashlib
import json
import os
import statistics
import subprocess
import time
from pathlib import Path

from capture_interaction import frames, key, wait_started
from compare_capture_memory import benchmark, output_size
from wayland_test_runtime import TestRuntime
from wayland_virtual_pointer import VirtualPointer, transform_for_pointer


def next_frame(process, log, count, started):
    """【截图性能】【首帧等待】按 process、log、count 和 started 返回新帧请求的观察延迟，退出或超时直接失败。"""
    deadline = started + 2
    while time.monotonic() < deadline:
        if len(frames(log.read_text())) > count:
            return (time.monotonic() - started) * 1000
        if process.poll() is not None:
            raise RuntimeError(f"editor exited: {log}")
        time.sleep(.001)
    raise RuntimeError(f"no local redraw within two seconds: {log}")


def sample(binary, environment, root, size, idle, repetitions):
    """【截图性能】【恢复采样】按参数进入缩放矩形工具，返回每次绘图等待帧请求的延迟及等待后 RSS，证据写入 root。"""
    config = root / "app.toml"
    config.write_text('[desktop]\ntray_enabled=false\nshortcuts_enabled=false\n[history]\nenabled=false\n')
    log = root / f"wait-{idle}.log"
    environment = dict(environment, WAYLAND_DEBUG="client")
    with log.open("w") as output:
        started = time.monotonic()
        process = subprocess.Popen([str(binary), "--config", str(config), "--fullscreen", "--speed"],
                                   env=environment, stdout=output, stderr=output)
        try:
            wait_started(process, log, started)
            with VirtualPointer() as pointer:
                def move(x, y):
                    """【截图性能】【私有指针】按 x、y 发送真实物理坐标，无返回值。"""
                    pointer.move_abs(*transform_for_pointer(x, y, *size, "flipped180"))

                move(size[0] * .15, size[1] * .3)
                pointer.click(0x111)
                pointer.scroll(dy=-3)
                time.sleep(.3)
                key({name: value for name, value in environment.items() if name != "WAYLAND_DEBUG"}, "r")
                time.sleep(.3)
                values = []
                # 1. 【截图性能】【交互稳定】先绘出一个矩形，确保后续测量包含稳定局部缓存与空闲释放后的重建
                for index in range(repetitions + 1):
                    x = size[0] * (.2 + (index % 5) * .06)
                    y = size[1] * (.35 + (index % 3) * .07)
                    move(x, y)
                    pointer.press()
                    time.sleep(.05)
                    pointer.move_abs(*transform_for_pointer(x + 50, y + 40, *size, "flipped180"))
                    time.sleep(.1)
                    pointer.release()
                    time.sleep(idle)
                    resident = benchmark.snapshot(process.pid)
                    count = len(frames(log.read_text()))
                    started = time.monotonic()
                    pointer.press()
                    move(x + 150, y + 115)
                    latency = next_frame(process, log, count, started)
                    pointer.release()
                    if index:
                        values.append({"request_observed_ms": latency,
                                       "idle_rss_mib": sum(v["rss_kib"] for v in resident.values()) / 1024})
                    time.sleep(.05)
                timings = [value["request_observed_ms"] for value in values]
                return {"idle_seconds": idle, "samples": values, "median_ms": statistics.median(timings),
                        "maximum_ms": max(timings)}
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)


def main():
    """【截图性能】【命令入口】读取 binary、root 和 samples 建立私有桌面，保存两种等待时长的实际观察结果。"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--samples", type=int, default=8)
    options = parser.parse_args()
    root = options.root.resolve()
    if root.exists():
        parser.error("use a new private directory")
    binary = options.binary.resolve()
    report = {"binary": str(binary), "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "capture_scene": os.environ.get("MARK_SHOT_CAPTURE_SCENE", "auto"),
              "metric": "input to next wl_surface.frame request; excludes display latency", "cases": []}
    root.mkdir(mode=0o700)
    original = dict(os.environ)
    for idle in [.05, .7]:
        case_root = root / f"wait-{idle}"
        with TestRuntime(case_root, 1.0) as environment:
            os.environ.update(environment)
            try:
                size = (3840, 2160)
                output_size(environment, size)
                result = sample(binary, environment, case_root, size, idle, options.samples)
                report["cases"].append(result)
                print(json.dumps(result), flush=True)
            finally:
                os.environ.clear()
                os.environ.update(original)
    (root / "result.json").write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
