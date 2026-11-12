"""【截图性能】【隔离对照】私有 Xvfb 比较 X11 或 Wine 原生 Windows 后端的普通与稀疏前景。"""

import argparse
import hashlib
import json
import os
import subprocess
import time
from pathlib import Path

from compare_capture_memory import benchmark, mappings
from x11_runtime import runtime


def annotation_pixels(path):
    """【跨平台性能】【实际绘图】按 path 统计红色矩形像素，返回数量，未绘出标注直接失败；此检查需要 Pillow。"""
    from PIL import Image

    with Image.open(path) as image:
        pixels = image.convert("RGB").tobytes()
    count = sum(r > 150 and r > g * 1.4 and r > b * 1.4
                for r, g, b in zip(pixels[0::3], pixels[1::3], pixels[2::3]))
    if count < 1000:
        raise RuntimeError(f"the zoomed rectangle was not drawn: {path}, pixels={count}")
    return count


def case(binary, root, env, size, mode, seconds, runner=None, scenario="selection"):
    """【X11性能】【单项采样】按参数测量启动、静态内存与固定输入速率的拖选 CPU，返回结果并保存截图。"""
    config = root / "app.toml"
    language = "en" if runner else "zh-CN"
    config.write_text(f'[desktop]\ntray_enabled=false\nshortcuts_enabled=false\n[history]\nenabled=false\n[ui]\nlanguage="{language}"\n[startup]\nselection_loupe=false\n')
    env = dict(env, MARK_SHOT_CAPTURE_SCENE="1" if mode == "sparse" else "0")
    log = root / f"{mode}.log"
    config_argument = str(config)
    if runner:
        config_argument = subprocess.check_output(["winepath", "-w", str(config)], env=env, text=True).strip()
    with log.open("w") as output:
        started = time.monotonic()
        command = [str(binary), "--config", config_argument, "--speed"]
        if scenario != "selection":
            command.append("--fullscreen")
        if runner:
            command.insert(0, runner)
        process = subprocess.Popen(command, env=env, stdout=output, stderr=output)
        try:
            deadline = started + 15
            while "frames staged + flushed" not in log.read_text():
                if process.poll() is not None or time.monotonic() >= deadline:
                    raise RuntimeError(f"capture did not start: {log}")
                time.sleep(.01)
            startup = (time.monotonic() - started) * 1000
            if scenario == "zoomed_rectangle":
                # 1. 【跨平台性能】【缩放准备】通过真实后端输入改变视图，再选择矩形，避免只测恒等视图
                subprocess.run(["xdotool", "mousemove", str(size[0] // 5), str(size[1] // 5),
                                "click", "3", "click", "4", "click", "4", "click", "4", "key", "r"],
                               env=env, check=True)
            time.sleep(2)
            idle = benchmark.measure(process.pid, seconds)
            initial = mappings(process.pid)
            subprocess.run(["xdotool", "mousemove", str(size[0] // 5), str(size[1] // 5), "mousedown", "1"], env=env, check=True)
            before = benchmark.snapshot(process.pid)
            started = time.monotonic()
            count = 0
            while time.monotonic() - started < seconds:
                phase = (count % 120) / 120
                x = round(size[0] * (.4 + phase * .4))
                y = round(size[1] * (.4 + phase * .4))
                subprocess.run(["xdotool", "mousemove", str(x), str(y)], env=env, check=True)
                count += 1
                time.sleep(max(0, started + count / 60 - time.monotonic()))
            elapsed = time.monotonic() - started
            after = benchmark.snapshot(process.pid)
            ticks = sum(value["ticks"] - before[pid]["ticks"] for pid, value in after.items() if pid in before)
            subprocess.run(["xdotool", "mouseup", "1"], env=env, check=True)
            time.sleep(.7)
            post_idle = benchmark.snapshot(process.pid)
            subprocess.run(["import", "-window", "root", str(root / f"{mode}.png")], env=env, check=True)
            annotations = annotation_pixels(root / f"{mode}.png") if scenario == "zoomed_rectangle" else None
            return {"mode": mode, "scenario": scenario, "startup_observed_ms": startup, "idle": idle, "mappings": initial,
                    "drag": {"seconds": elapsed, "inputs": count, "cpu_percent": ticks / os.sysconf("SC_CLK_TCK") / elapsed * 100,
                             "rss_mib": sum(value["rss_kib"] for value in after.values()) / 1024},
                    "post_idle_rss_mib": sum(value["rss_kib"] for value in post_idle.values()) / 1024,
                    "annotation_pixels": annotations}
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)


def main():
    """【X11性能】【命令入口】按 binary/root/size 建立独立 X 服务，保存可重复的两种路径对照。"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--width", type=int, default=3840)
    parser.add_argument("--height", type=int, default=2160)
    parser.add_argument("--seconds", type=float, default=4)
    parser.add_argument("--runner", help="Executable runner, for example wine for the Windows build")
    parser.add_argument("--scenario", choices=("selection", "zoomed_rectangle"), default="selection")
    options = parser.parse_args()
    root = options.root.resolve()
    root.mkdir(mode=0o700)
    binary = options.binary.resolve()
    with runtime(root, (options.width, options.height), options.runner) as env:
        report = {"binary": str(binary), "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "platform": "windows-wine" if options.runner else "x11", "scenario": options.scenario, "size": [options.width, options.height], "cases": []}
        for mode in ("legacy", "sparse"):
            value = case(binary, root, env, (options.width, options.height), mode, options.seconds, options.runner, options.scenario)
            report["cases"].append(value)
            print(json.dumps({"mode": mode, "rss_mib": value["idle"]["rss_mean_mib"], "drag": value["drag"]}), flush=True)
        (root / "result.json").write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
