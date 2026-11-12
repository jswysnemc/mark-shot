"""【跨平台性能】【恢复对照】测量 X11 或 Windows/Wine 输入至服务器窗口像素改变的时长。"""

import argparse
import hashlib
import json
import statistics
import subprocess
import time
from pathlib import Path

from compare_capture_memory import benchmark
from x11_pixels import Pixels
from x11_runtime import runtime


def sample(binary, root, env, size, mode, scenario, repetitions, runner, rectangle_span):
    """【跨平台性能】【单项恢复】建立真实选区或缩放矩形，返回像素恢复数据与空闲 RSS。

    binary 与 runner 指定程序，root 保存证据，env 与 size 指定私有桌面；
    mode 与 scenario 指定绘制路径，repetitions 指定样本数，rectangle_span 指定矩形首次移动范围。
    """
    config = root / "app.toml"
    config.write_text('[desktop]\ntray_enabled=false\nshortcuts_enabled=false\n[history]\nenabled=false\n'
                      '[ui]\nlanguage="en"\n[startup]\nselection_loupe=false\n')
    env = dict(env, MARK_SHOT_CAPTURE_SCENE="1" if mode == "sparse" else "0")
    argument = subprocess.check_output(["winepath", "-w", str(config)], env=env, text=True).strip() if runner else str(config)
    command = [str(binary), "--config", argument, "--speed"]
    if scenario == "zoomed_rectangle":
        command.append("--fullscreen")
    if runner:
        command.insert(0, runner)
    probe = Pixels(env["DISPLAY"])
    probe.background(size)
    log = root / f"{mode}-{scenario}.log"
    with log.open("w") as output:
        process = subprocess.Popen(command, env=env, stdout=output, stderr=output)
        try:
            deadline = time.monotonic() + 15
            while "frames staged + flushed" not in log.read_text():
                if process.poll() is not None or time.monotonic() >= deadline:
                    raise RuntimeError(f"capture did not start: {log}")
                time.sleep(.01)
            time.sleep(.3)
            if scenario == "zoomed_rectangle":
                # 1. 【跨平台性能】【真实缩放】改变视图并选择矩形，恢复测量必须经过导航背景路径
                subprocess.run(["xdotool", "mousemove", str(size[0] // 5), str(size[1] // 5),
                                "click", "3", "click", "4", "click", "4", "click", "4", "key", "r"],
                               env=env, check=True)
                time.sleep(.3)
            origin = (round(size[0] * .2), round(size[1] * .2))
            small = (round(size[0] * .4), round(size[1] * .4))
            large = (round(size[0] * .8), round(size[1] * .8))
            region = (round(size[0] * .6), origin[1] - 4 if scenario == "zoomed_rectangle" else round(size[1] * .3), 8, 8)
            if scenario == "zoomed_rectangle":
                origin = (round(size[0] * .85), round(size[1] * .85))
                small = (origin[0] + 150, origin[1] + 115)
            probe.button(False)
            time.sleep(.1)
            probe.move(*origin)
            time.sleep(.1)
            probe.button(True)
            time.sleep(.1)
            probe.move(*small)
            time.sleep(.2)
            if scenario == "zoomed_rectangle":
                probe.button(False)
            groups = []
            for group_index, idle in enumerate((.05, .7)):
                values = []
                # 2. 【跨平台性能】【恢复触发】拖选暂停后继续改变角点，导航绘图释放鼠标后在独立高度开始新矩形
                for index in range(repetitions + 1):
                    if scenario == "zoomed_rectangle":
                        slot = (group_index * (repetitions + 1) + index) % 40
                        origin = ((round(size[0] * .2), round(size[1] * (.1 + slot * .015))) if rectangle_span == "large"
                                  else (round(size[0] * (.15 + slot % 8 * .07)), round(size[1] * (.15 + slot // 8 * .1))))
                        large = (round(size[0] * .8), round(size[1] * .8)) if rectangle_span == "large" else (origin[0] + 150, origin[1] + 115)
                        region = (round(size[0] * .6) if rectangle_span == "large" else origin[0] + 75, origin[1] - 4, 8, 8)
                        probe.move(*origin)
                    else:
                        probe.move(*small)
                    time.sleep(idle)
                    before = probe.read(region)
                    resident = benchmark.snapshot(process.pid)
                    started = time.monotonic()
                    if scenario == "zoomed_rectangle":
                        probe.button(True)
                    probe.move(*large)
                    try:
                        latency, pixels = probe.changed(region, before, started, process)
                    except RuntimeError:
                        failure = {"mode": mode, "scenario": scenario, "index": index, "idle_seconds": idle,
                                   "region": list(region), "before": before.hex(), "after": probe.read(region).hex()}
                        (root / "pixel-failure.json").write_text(json.dumps(failure, indent=2))
                        subprocess.run(["import", "-window", "root", str(root / "pixel-failure.png")], env=env, check=True)
                        raise
                    # 3. 【跨平台性能】【图形确认】24 位 Xvfb 使用四字节 BGRX 像素，更新区域必须包含红色标注
                    if scenario == "zoomed_rectangle" and not any(r > 150 and r > g * 1.4 and r > b * 1.4
                                                                for b, g, r in zip(pixels[0::4], pixels[1::4], pixels[2::4])):
                        raise RuntimeError("observed change did not contain the rectangle edge")
                    if scenario == "zoomed_rectangle":
                        probe.button(False)
                    if index:
                        values.append({"observed_ms": latency,
                                       "idle_rss_mib": sum(value["rss_kib"] for value in resident.values()) / 1024})
                    time.sleep(.05)
                timings = [value["observed_ms"] for value in values]
                groups.append({"idle_seconds": idle, "samples": values, "median_ms": statistics.median(timings),
                               "maximum_ms": max(timings)})
            probe.button(False)
            time.sleep(.1)
            return {"mode": mode, "scenario": scenario, "groups": groups}
        finally:
            probe.close()
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)


def main():
    """【跨平台性能】【命令入口】读取 binary、root、runner 和 samples，在私有桌面保存同一二进制的配对恢复结果。"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--runner", choices=("wine",))
    parser.add_argument("--samples", type=int, default=12)
    parser.add_argument("--width", type=int, default=3840)
    parser.add_argument("--height", type=int, default=2160)
    parser.add_argument("--rectangle-span", choices=("local", "large"), default="local",
                        help="First rectangle movement: 150x115 pixels or most of the screen")
    parser.add_argument("--scenarios", nargs="+", choices=("selection", "zoomed_rectangle"),
                        default=("selection", "zoomed_rectangle"))
    options = parser.parse_args()
    if options.samples < 1:
        parser.error("samples must be positive")
    root = options.root.resolve()
    root.mkdir(mode=0o700)
    binary = options.binary.resolve()
    size = (options.width, options.height)
    report = {"binary": str(binary), "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "platform": "windows-wine" if options.runner else "x11", "size": list(size),
              "rectangle_span": options.rectangle_span,
              "metric": "XTest input to changed server window pixels; excludes physical display latency", "complete": False, "cases": []}
    with runtime(root, size, options.runner) as env:
        for scenario in options.scenarios:
            for mode in ("legacy", "sparse"):
                value = sample(binary, root, env, size, mode, scenario, options.samples, options.runner, options.rectangle_span)
                report["cases"].append(value)
                (root / "result.json").write_text(json.dumps(report, indent=2))
                print(json.dumps({"mode": mode, "scenario": scenario,
                                  "groups": [{key: item for key, item in group.items() if key != "samples"}
                                             for group in value["groups"]]}), flush=True)
    report["complete"] = True
    (root / "result.json").write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
