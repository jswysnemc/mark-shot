"""【截图性能】【配对拖选】在同一私有合成器中平衡顺序对照旧模式与优化模式。"""

import argparse
import hashlib
import json
import os
import shutil
import statistics
from pathlib import Path

from capture_interaction import sample
from compare_capture_memory import output_size
from wayland_test_runtime import TestRuntime


def summarize(runs, minimum_fps, maximum_p95):
    """【截图性能】【结果判定】按 runs 汇总两种模式，使用 minimum_fps 与 maximum_p95 比例判定，返回完整统计与通过状态。"""
    modes = {}
    for mode in ("legacy", "sparse"):
        selected = [run for run in runs if run["mode"] == mode]
        modes[mode] = {
            "fps_mean": statistics.mean(run["fps"] for run in selected),
            "fps_median": statistics.median(run["fps"] for run in selected),
            "p95_median_ms": statistics.median(run["p95_ms"] for run in selected),
            "cpu_mean_percent": statistics.mean(run["cpu_percent"] for run in selected),
        }
    fps_ratio = modes["sparse"]["fps_mean"] / modes["legacy"]["fps_mean"]
    p95_ratio = modes["sparse"]["p95_median_ms"] / modes["legacy"]["p95_median_ms"]
    return {"modes": modes, "fps_ratio": fps_ratio, "p95_ratio": p95_ratio,
            "minimum_fps_ratio": minimum_fps, "maximum_p95_ratio": maximum_p95,
            "passed": fps_ratio >= minimum_fps and p95_ratio <= maximum_p95}


def main():
    """【截图性能】【命令入口】读取 binary、root、seconds 与 repetitions，保存真实拖选配对证据，超过性能容差时退出失败。"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--seconds", type=float, default=8)
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--min-fps-ratio", type=float, default=.98,
                        help="Minimum optimized/legacy mean FPS ratio")
    parser.add_argument("--max-p95-ratio", type=float, default=1.05,
                        help="Maximum optimized/legacy median p95 frame interval ratio")
    options = parser.parse_args()
    if options.repetitions < 2 or options.seconds < 1:
        parser.error("use at least two repetitions and one second per run")
    if not 0 < options.min_fps_ratio <= 1 or options.max_p95_ratio < 1:
        parser.error("invalid performance tolerance")
    root = options.root.resolve()
    root.mkdir(mode=0o700)
    binary = options.binary.resolve()
    report = {"binary": str(binary), "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "size": [3840, 2160], "metric": "main surface frame requests; excludes display latency",
              "runs": []}
    original = dict(os.environ)
    # 1. 【截图性能】【固定环境】共用一个私有合成器，保持相同输出尺寸、配置和输入轨迹
    with TestRuntime(root / "desktop", 1.0) as environment:
        os.environ.update(environment)
        try:
            output_size(environment, (3840, 2160))
            config = root / "app.toml"
            config.write_text('[desktop]\ntray_enabled=false\nshortcuts_enabled=false\n'
                              '[history]\nenabled=false\n[ui]\nlanguage="zh-CN"\n')
            for repetition in range(options.repetitions):
                # 2. 【截图性能】【顺序平衡】相邻两组交换先后顺序，每次单独保留帧日志、读取计数和截图
                modes = ("legacy", "sparse") if repetition % 2 == 0 else ("sparse", "legacy")
                for mode in modes:
                    run_root = root / f"{repetition}-{mode}"
                    run_root.mkdir(mode=0o700)
                    shutil.copyfile(root / "desktop/runtime.json", run_root / "runtime.json")
                    run = sample(binary, dict(environment, MARK_SHOT_CAPTURE_SCENE="1" if mode == "sparse" else "0"),
                                 run_root, config, (3840, 2160), "selection", options.seconds)
                    run.update(mode=mode, repetition=repetition)
                    report["runs"].append(run)
                    print(json.dumps({key: run[key] for key in ("mode", "repetition", "fps", "p95_ms", "cpu_percent",
                                                               "io_read_bytes", "io_read_calls")}), flush=True)
        finally:
            os.environ.clear()
            os.environ.update(original)
    # 3. 【截图性能】【容差判定】记录全部运行后判断平均帧率与95分位帧间隔，失败结果同样保留证据
    report["summary"] = summarize(report["runs"], options.min_fps_ratio, options.max_p95_ratio)
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report["summary"]), flush=True)
    if not report["summary"]["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
