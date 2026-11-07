#!/usr/bin/env python3
"""【Windows识别诊断】【首帧确认】直接回放真实OCR结果帧，验证辅助进程三秒期限内的就绪信号"""

import argparse
import json
from pathlib import Path
import select
import struct
import subprocess
import time

from windows_ocr_actions import path_w, snapshot, stop, window
from windows_test_runtime import run


ROOT = Path(__file__).resolve().parents[1]


def execute(command, environment, root, log):
    """【Windows识别诊断】【确认回放】按command启动真实结果窗口并读取标准输出，返回确认状态，在root保留映射与期限证据"""
    binary = command[0]
    config = root / "config.toml"
    config.write_text('[ui]\ntheme="light"\n[notifications]\nenabled=false\n')
    header = json.dumps({"text": "Windows OCR ready", "lines": []}).encode()
    image = (ROOT / "tests/fixtures/ocr.png").read_bytes()
    frame = b"MSOCR001" + struct.pack("<II", len(header), len(image)) + header + image
    started = time.monotonic()
    process = subprocess.Popen(["wine", binary, "--config", path_w(config, environment), "--ocr-result"],
                               stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=log, env=environment, cwd=root)
    report = {"ready": False, "mapped": False, "real_windows_validation": False}
    try:
        # 1. 【Windows识别诊断】【真实传递】使用生产帧格式，输入交付后开始三秒确认期限，与生产调用的计时位置相同
        process.stdin.write(frame)
        process.stdin.close()
        delivered = time.monotonic()
        readable = bool(select.select([process.stdout], [], [], 3)[0])
        line = process.stdout.readline() if readable else None
        report["confirmation"] = None if line is None else line.decode(errors="replace")
        report["ready"] = line == b"\n"
        report["confirmation_seconds"] = time.monotonic() - delivered
        # 2. 【Windows识别诊断】【迟到区分】确认失败后仍等待实际窗口，记录迟到信号，不能把已经映射视为按时交接
        window(environment, "OCR Result", root, process)
        report["mapped"] = True
        report["mapping_seconds"] = time.monotonic() - started
        snapshot(environment, root, "mapped-result")
        if not report["ready"] and select.select([process.stdout], [], [], 3)[0]:
            report["late_confirmation"] = process.stdout.readline().decode(errors="replace")
        report["seconds"] = time.monotonic() - started
        return subprocess.CompletedProcess(command, 0 if report["ready"] else 1)
    finally:
        (root / "ready.json").write_text(json.dumps(report, indent=2) + "\n")
        stop(process)


def main():
    """【Windows识别诊断】【私有入口】读取binary和新root，回放窗口确认并输出结果，无参数、无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--timing", action="store_true")
    options = parser.parse_args()
    def timed(command, environment, root, log):
        """【Windows识别诊断】【基础命令计时】按command与environment比较版本和显示器命令，保存root中的耗时再回放结果确认"""
        times = {}
        for argument in ["--version", "--list-displays"]:
            started = time.monotonic()
            result = subprocess.run(["wine", command[0], argument], env=environment, cwd=root, stdout=subprocess.PIPE,
                                    stderr=log, check=True, timeout=20)
            times[argument] = {"seconds": time.monotonic() - started, "stdout": result.stdout.decode(errors="replace")}
        (root / "startup.json").write_text(json.dumps(times, indent=2) + "\n")
        return execute(command, environment, root, log)
    report = run(options.root.resolve(), [str(options.binary.resolve())], execute=timed if options.timing else execute)
    print(json.dumps(report, indent=2))
    raise SystemExit(report["exit_code"])


if __name__ == "__main__":
    main()
