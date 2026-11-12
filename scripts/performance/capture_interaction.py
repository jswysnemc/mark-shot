"""【截图性能】【交互基准】私有 Niri 下记录实际帧请求、CPU、驻留内存和启动时长。"""

import argparse
import hashlib
import itertools
import json
import os
import re
import statistics
import subprocess
import sys
import time
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
from compare_capture_memory import benchmark, output_size
from wayland_test_runtime import TestRuntime
from wayland_virtual_pointer import VirtualPointer, transform_for_pointer


def frames(text):
    """【截图性能】【主帧统计】从 text 的层角色找到主表面，返回请求显示下一帧的时间，不依赖完整缓冲尺寸。"""
    parents = set(re.findall(r'get_layer_surface\([^\n]*wl_surface[@#](\d+)[^\n]*Some\("(?:mark-shot|utility)"\)', text))
    values = []
    for stamp, surface in re.findall(r'\[\s*([\d.:]+)\].*wl_surface[@#](\d+)\.frame\(', text):
        if surface not in parents:
            continue
        if ":" in stamp:
            hour, minute, second = map(float, stamp.split(":"))
            values.append((hour * 3600 + minute * 60 + second) * 1000)
        else:
            values.append(float(stamp))
    return values


def wait_started(process, log, started):
    """【截图性能】【启动等待】按 process/log 等待真实首帧日志，返回内部阶段时长和父进程观察上限。"""
    until = time.monotonic() + 10
    while time.monotonic() < until:
        text = log.read_text(errors="replace")
        match = re.search(r'\[\s*([\d.]+)ms\].*frames staged \+ flushed', text)
        if match and frames(text):
            return {"staged_ms": float(match[1]), "observed_ms": (time.monotonic() - started) * 1000}
        if process.poll() is not None:
            raise RuntimeError(f"editor exited during startup: {text[-1000:]}")
        time.sleep(.01)
    raise RuntimeError(f"editor did not present: {log}")


def key(env, value):
    """【截图性能】【键盘输入】向已经验证的私有 env 发送 value 按键，无返回值。"""
    subprocess.run(["wtype", "-k", value], env=env, check=True)


def reads(pid):
    """【截图性能】【读取计数】按 pid 返回当前进程的读取字节与读取调用数量，来源为 Linux 进程统计，不读取程序内容。"""
    values = dict(line.split(":", 1) for line in Path(f"/proc/{pid}/io").read_text().splitlines())
    return int(values["rchar"]), int(values["syscr"])


def sample(binary, env, root, config, size, scenario, seconds):
    """【截图性能】【交互采样】按参数持续发送 120Hz 输入，返回实际主表面帧间隔、CPU 和最终内存。"""
    log = root / f"{scenario}.log"
    options = ["--fullscreen"] if scenario in ("rectangle", "pen", "zoom", "zoomed_rectangle", "zoomed_pen") else []
    command = [str(binary), "--config", str(config), "--speed", *options]
    with log.open("w") as errors:
        started = time.monotonic()
        process = subprocess.Popen(command, env=dict(env, WAYLAND_DEBUG="client"), stdout=subprocess.DEVNULL, stderr=errors)
        try:
            startup = wait_started(process, log, started)
            time.sleep(.5)
            with VirtualPointer() as pointer:
                move = lambda x, y: pointer.move_abs(*transform_for_pointer(x, y, *size, "flipped180"))
                move(size[0] * .15, size[1] * .22)
                if scenario in ("zoomed_rectangle", "zoomed_pen"):
                    pointer.click(0x111)
                    pointer.scroll(dy=-3.0)
                    time.sleep(.3)
                    key(env, "r" if scenario == "zoomed_rectangle" else "p")
                if scenario in ("rectangle", "pen"):
                    key(env, "r" if scenario == "rectangle" else "p")
                if scenario == "zoom":
                    pointer.click(0x111)
                time.sleep(.2)
                if scenario not in ("hover", "zoom"):
                    pointer.press()
                initial_frames = len(frames(log.read_text()))
                before = benchmark.snapshot(process.pid)
                reads_before = reads(process.pid)
                compositor = json.loads((root / "runtime.json").read_text())["niri_pid"]
                compositor_before = benchmark.snapshot(compositor)
                started = time.monotonic()
                events = 0
                while time.monotonic() - started < seconds:
                    phase = (time.monotonic() - started) % 2 / 2
                    fraction = phase * 2 if phase < .5 else 2 - phase * 2
                    if scenario == "zoom":
                        pointer.scroll(dy=.75 if int((time.monotonic() - started) * 2) % 2 else -.75)
                    else:
                        move(size[0] * (.2 + .5 * fraction), size[1] * (.3 + .35 * fraction))
                    events += 1
                    time.sleep(max(0, started + events / 120 - time.monotonic()))
                duration = time.monotonic() - started
                after = benchmark.snapshot(process.pid)
                reads_after = reads(process.pid)
                compositor_after = benchmark.snapshot(compositor)
                timing = frames(log.read_text())[initial_frames:]
                if scenario not in ("hover", "zoom"):
                    pointer.release()
            gaps = sorted(b - a for a, b in itertools.pairwise(timing) if b >= a)
            ticks = sum(value["ticks"] - before[pid]["ticks"] for pid, value in after.items() if pid in before)
            compositor_ticks = sum(value["ticks"] - compositor_before[pid]["ticks"] for pid, value in compositor_after.items() if pid in compositor_before)
            time.sleep(.6)
            idle = benchmark.snapshot(process.pid)
            subprocess.run(["grim", "-o", "winit", str(root / f"{scenario}.png")], env=env, check=True)
            return {"scenario": scenario, "startup": startup, "seconds": duration, "inputs": events,
                "frames": len(timing), "fps": len(timing) / duration,
                "median_ms": statistics.median(gaps) if gaps else None,
                "p95_ms": gaps[min(len(gaps)-1, int(len(gaps)*.95))] if gaps else None,
                "cpu_percent": ticks / os.sysconf("SC_CLK_TCK") / duration * 100,
                "io_read_bytes": reads_after[0] - reads_before[0],
                "io_read_calls": reads_after[1] - reads_before[1],
                "compositor_cpu_percent": compositor_ticks / os.sysconf("SC_CLK_TCK") / duration * 100,
                "rss_mib": sum(v["rss_kib"] for v in after.values()) / 1024,
                "idle_rss_mib": sum(v["rss_kib"] for v in idle.values()) / 1024,
                "idle_pss_mib": sum(v["pss_kib"] for v in idle.values()) / 1024}
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)


def main():
    """【截图性能】【命令入口】读取 binary/root 参数，建立私有桌面执行标准交互并保存 JSON 证据。"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--seconds", type=float, default=4)
    parser.add_argument("--width", type=int, default=3840)
    parser.add_argument("--height", type=int, default=2160)
    parser.add_argument("--scenarios", nargs="+", choices=("hover", "selection", "rectangle", "pen", "zoom", "zoomed_rectangle", "zoomed_pen"),
                        default=("hover", "selection", "rectangle", "pen", "zoom"))
    options = parser.parse_args()
    root = options.root.resolve()
    if root.exists():
        parser.error("use a new private directory")
    binary = options.binary.resolve()
    report = {"binary": str(binary), "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "capture_scene": os.environ.get("MARK_SHOT_CAPTURE_SCENE", "auto"), "scenarios": []}
    original = dict(os.environ)
    with TestRuntime(root, 1.0) as env:
        os.environ.update(env)
        try:
            size = (options.width, options.height)
            output_size(env, size)
            config = root / "app.toml"
            config.write_text('[desktop]\ntray_enabled=false\nshortcuts_enabled=false\n[history]\nenabled=false\n[ui]\nlanguage="zh-CN"\n')
            for scenario in options.scenarios:
                result = sample(binary, env, root, config, size, scenario, options.seconds)
                report["scenarios"].append(result)
                print(json.dumps(result), flush=True)
            report["size"] = list(size)
            (root / "result.json").write_text(json.dumps(report, indent=2))
        finally:
            os.environ.clear()
            os.environ.update(original)


if __name__ == "__main__":
    main()
