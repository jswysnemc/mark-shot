#!/usr/bin/env python3
"""【共享内存验证】【页锁定】记录重复交互截图前后的全系统 FOLL_PIN 净增量"""

import argparse
import json
import os
from pathlib import Path
import platform
import subprocess
import time


def counters():
    """【共享内存验证】【计数读取】无传参，读取内核获取和释放页数，返回累计计数与净占用页数"""
    values = {}
    for line in Path("/proc/vmstat").read_text().splitlines():
        name, value = line.split()
        if name in ("nr_foll_pin_acquired", "nr_foll_pin_released"):
            values[name] = int(value)
    if len(values) != 2:
        raise RuntimeError("the kernel does not expose FOLL_PIN counters")
    acquired, released = values["nr_foll_pin_acquired"], values["nr_foll_pin_released"]
    return {
        "acquired": acquired,
        "released": released,
        "pinned_pages": acquired - released,
    }


def settled(timeout):
    """【共享内存验证】【释放等待】按 timeout 等待净计数稳定，返回最后采样，其他进程活动不会无限延长等待"""
    deadline = time.monotonic() + timeout
    previous = counters()
    stable = 0
    while time.monotonic() < deadline:
        time.sleep(0.2)
        current = counters()
        stable = (
            stable + 1 if current["pinned_pages"] == previous["pinned_pages"] else 0
        )
        previous = current
        if stable >= 5:
            break
    return previous


def main():
    """【共享内存验证】【交互验证】读取命令、次数和输出文件，记录每次正常退出后的净增量并写入 JSON，无返回值"""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--iterations", type=int, default=3)
    parser.add_argument("--settle-seconds", type=float, default=5.0)
    parser.add_argument("--wait-for-pin-close", action="store_true")
    parser.add_argument(
        "--output", type=Path, default=Path("target/foll-pin-validation.json")
    )
    parser.add_argument("command", nargs=argparse.REMAINDER)
    options = parser.parse_args()
    command = options.command
    if command[:1] == ["--"]:
        command = command[1:]
    if not command:
        command = ["mark-shot", "--capture"]
    if options.iterations < 1 or not 1.0 <= options.settle_seconds <= 60.0:
        parser.error(
            "iterations must be positive and settle-seconds must be between 1 and 60"
        )
    page_size = os.sysconf("SC_PAGE_SIZE")
    report = {
        "kernel": platform.release(),
        "desktop": os.environ.get("XDG_CURRENT_DESKTOP", ""),
        "session_type": os.environ.get("XDG_SESSION_TYPE", ""),
        "page_size": page_size,
        "command": command,
        "samples": [],
        "scope": "system-wide counters; other applications can affect the result",
    }
    # 1. 【共享内存验证】【交互循环】由操作者取消、复制或贴图结束截图，不自动操作或终止桌面窗口
    baseline = settled(options.settle_seconds)
    report["baseline"] = baseline
    for iteration in range(1, options.iterations + 1):
        before = counters()
        print(
            f"Capture {iteration}/{options.iterations}: complete the requested action and close the capture",
            flush=True,
        )
        started = time.monotonic()
        result = subprocess.run(command, check=False)
        if options.wait_for_pin_close:
            input("Close all pinned windows from this capture, then press Enter: ")
        after = settled(options.settle_seconds)
        growth = after["pinned_pages"] - before["pinned_pages"]
        sample = {
            "iteration": iteration,
            "exit_code": result.returncode,
            "elapsed_seconds": round(time.monotonic() - started, 3),
            "before": before,
            "after": after,
            "growth_pages": growth,
            "growth_bytes": growth * page_size,
        }
        report["samples"].append(sample)
        report["total_growth_pages"] = after["pinned_pages"] - baseline["pinned_pages"]
        # 2. 【共享内存验证】【逐次保存】每轮立即保存，后续操作失败时仍保留此前实际采样
        options.output.parent.mkdir(parents=True, exist_ok=True)
        options.output.write_text(
            json.dumps(report, ensure_ascii=False, indent=2) + "\n"
        )
        print(
            f"Capture {iteration}: growth={growth} pages ({growth * page_size} bytes)",
            flush=True,
        )
    print(f"Evidence: {options.output}")


if __name__ == "__main__":
    main()
