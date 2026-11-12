"""采样指定 Linux 录制进程树，报告单核口径 CPU 和 RSS/PSS，不包含桌面合成器。"""

import argparse
import json
import os
import statistics
import time
from pathlib import Path


def snapshot(pid: int) -> dict:
    """按 pid 读取自身与全部子进程的 CPU 时钟及内存，返回每个进程的快照。"""
    result = {}
    pending = [pid]
    while pending:
        current = pending.pop()
        if current in result:
            continue
        root = Path(f"/proc/{current}")
        try:
            stat = (root / "stat").read_text().rsplit(")", 1)[1].split()
            memory = {}
            for line in (root / "smaps_rollup").read_text().splitlines():
                key, _, value = line.partition(":")
                if key in ("Rss", "Pss", "SwapPss"):
                    memory[key] = int(value.split()[0])
            result[current] = {
                "start": int(stat[19]),
                "ticks": int(stat[11]) + int(stat[12]),
                "rss_kib": memory.get("Rss", 0),
                "pss_kib": memory.get("Pss", 0),
                "swap_pss_kib": memory.get("SwapPss", 0),
                "name": (root / "comm").read_text().strip(),
            }
            for task in (root / "task").iterdir():
                pending.extend(int(value) for value in (task / "children").read_text().split())
        except (FileNotFoundError, ProcessLookupError):
            continue
    return result


def measure(pid: int, seconds: float, interval: float = 0.5) -> dict:
    """按 pid、seconds 和 interval 采样进程树，返回 CPU 均值及内存均值、峰值和原始数据。"""
    ticks_per_second = os.sysconf("SC_CLK_TCK")
    samples = []
    previous = snapshot(pid)
    if pid not in previous:
        raise RuntimeError("benchmark process is not running")
    started = last = time.monotonic()
    cpu_seconds = 0.0
    while time.monotonic() - started < seconds:
        time.sleep(min(interval, max(0.0, seconds - (time.monotonic() - started))))
        current = snapshot(pid)
        now = time.monotonic()
        if pid not in current:
            raise RuntimeError("recording process exited during measurement")
        delta = sum(
            max(0, value["ticks"] - previous[process]["ticks"])
            for process, value in current.items()
            if process in previous and previous[process]["start"] == value["start"]
        ) / ticks_per_second
        cpu_seconds += delta
        samples.append({
            "elapsed": now - started,
            "cpu_percent_one_core": delta / (now - last) * 100,
            "rss_mib": sum(value["rss_kib"] for value in current.values()) / 1024,
            "pss_mib": sum(value["pss_kib"] for value in current.values()) / 1024,
            "swap_pss_mib": sum(value["swap_pss_kib"] for value in current.values()) / 1024,
            "processes": current,
        })
        previous, last = current, now
    return {
        "pid": pid,
        "duration_seconds": last - started,
        "cpu_percent_one_core": cpu_seconds / (last - started) * 100,
        "rss_mean_mib": statistics.mean(item["rss_mib"] for item in samples),
        "rss_peak_mib": max(item["rss_mib"] for item in samples),
        "pss_mean_mib": statistics.mean(item["pss_mib"] for item in samples),
        "pss_peak_mib": max(item["pss_mib"] for item in samples),
        "swap_pss_mean_mib": statistics.mean(item["swap_pss_mib"] for item in samples),
        "swap_pss_peak_mib": max(item["swap_pss_mib"] for item in samples),
        "samples": samples,
    }


def main() -> None:
    """无参数，读取命令行并把指定进程的测量结果写入 JSON 文件。"""
    parser = argparse.ArgumentParser()
    parser.add_argument("pid", type=int)
    parser.add_argument("--seconds", type=float, default=20)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.seconds <= 0:
        parser.error("seconds must be positive")
    report = measure(args.pid, args.seconds)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps({key: value for key, value in report.items() if key != "samples"}))


if __name__ == "__main__":
    main()
