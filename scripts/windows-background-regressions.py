#!/usr/bin/env python3
"""【Windows后台回归】【快捷截图】验证无参数启动保持静默，并记录空闲前后快捷键到首帧的时间"""

import argparse
import json
from pathlib import Path
import subprocess
import time

PROBE_SOURCE = r'''
#include <windows.h>
#include <tlhelp32.h>
#include <stdio.h>
/** 【快捷键验证】【动作消息】无参数，向私有Wine的Mark Shot线程发送快捷动作，返回交付状态 */
int main(void) {
    HANDLE processes = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    PROCESSENTRY32W process = {0}; process.dwSize = sizeof(process);
    FILETIME now; GetSystemTimeAsFileTime(&now);
    ULARGE_INTEGER ticks; ticks.LowPart = now.dwLowDateTime; ticks.HighPart = now.dwHighDateTime;
    double sent_at = (ticks.QuadPart - 116444736000000000ULL) / 10000000.0;
    int sent = 0;
    if (Process32FirstW(processes, &process)) do {
        if (lstrcmpiW(process.szExeFile, L"mark-shot.exe") != 0) continue;
        HANDLE threads = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        THREADENTRY32 thread = {0}; thread.dwSize = sizeof(thread);
        if (Thread32First(threads, &thread)) do {
            if (thread.th32OwnerProcessID == process.th32ProcessID)
                sent += PostThreadMessageW(thread.th32ThreadID, WM_HOTKEY, 1, 0) != 0;
        } while (Thread32Next(threads, &thread));
        CloseHandle(threads);
    } while (Process32NextW(processes, &process));
    CloseHandle(processes);
    printf("%.6f\n", sent_at);
    return sent ? 0 : 1;
}
'''

from PIL import Image
from windows_ocr_actions import path_w, stop, visible, window, xdo
from windows_test_runtime import run


def execute(command, environment, root, log, message_probe=False, idle_seconds=2):
    """【Windows后台回归】【完整链路】按 command、environment、root 与 log 启动真实后台并发送热键，返回回归状态与时延记录"""
    binary = command[0]
    process = subprocess.Popen(["wine", binary], env=environment, cwd=root, stdout=log, stderr=log)
    report = {"passed": False, "real_windows_validation": False, "latency_ms": [],
              "input_mode": "wm_hotkey" if message_probe else "printscreen", "idle_seconds": idle_seconds}
    probe = root / "hotkey-probe.exe"
    if message_probe:
        source = root / "hotkey-probe.c"
        source.write_text(PROBE_SOURCE)
        subprocess.run(["winegcc", "-m64", "-o", str(probe), str(source), "-luser32"], check=True, timeout=20)
    foreground = None
    try:
        # 1. 【Windows后台回归】【注册确认】等候真实服务状态，默认启动不应产生截图窗口
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            paths = list((root / "wine-prefix/drive_c/users").glob("*/AppData/Local/mark-shot/desktop/*/status.json"))
            if paths and json.loads(paths[0].read_text())["shortcuts"]["state"] == "registered":
                break
            assert process.poll() is None, "default startup did not remain in the background"
            time.sleep(0.05)
        else:
            raise AssertionError("default Windows background did not register its shortcut")
        assert not visible(environment, "Mark Shot"), "default startup opened a screenshot"
        report["silent_startup"] = True
        # 2. 【Windows后台回归】【系统焦点】Wine需要前台Windows窗口接收真实系统按键，贴图保留为独立应用角色
        source = root / "source.png"
        Image.new("RGB", (160, 100), "white").save(source)
        foreground = subprocess.Popen(["wine", binary, "--pin", path_w(source, environment)],
                                      env=environment, cwd=root, stdout=log, stderr=log)
        identity = window(environment, "Pinned Mark Shot", root, foreground)
        # 3. 【Windows后台回归】【空闲唤醒】两次动作使用同一后台进程，消息模式不声明真实按键验证
        for index in range(2):
            if index:
                time.sleep(idle_seconds)
            xdo(environment, "windowfocus", "--sync", identity)
            started = time.monotonic()
            sent_at = None
            if message_probe:
                sent_at = float(subprocess.check_output([str(probe)], env=environment, text=True, timeout=10).strip())
            else:
                xdo(environment, "key", "--clearmodifiers", "Print")
            window(environment, "Mark Shot", root)
            elapsed = time.time() - sent_at if sent_at is not None else time.monotonic() - started
            report["latency_ms"].append(round(elapsed * 1000, 2))
            xdo(environment, "key", "--clearmodifiers", "Escape")
            deadline = time.monotonic() + 3
            while visible(environment, "Mark Shot") and time.monotonic() < deadline:
                time.sleep(0.02)
            assert not visible(environment, "Mark Shot"), "shortcut screenshot did not cancel"
            assert process.poll() is None, "background exited with its screenshot"
        assert report["latency_ms"][0] < 3000, "first shortcut was blocked by standby preparation"
        assert report["latency_ms"][1] < 1000, "idle shortcut took more than one second to map"
        report["passed"] = True
        return subprocess.CompletedProcess(command, 0)
    finally:
        if foreground:
            stop(foreground)
        stop(process)
        (root / "background-regressions.json").write_text(json.dumps(report, indent=2) + "\n")


def main():
    """【Windows后台回归】【私有入口】读取候选和新 root 后运行独立 Wine 验证，无参数、无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--message-probe", action="store_true", help="synthesize WM_HOTKEY in private Wine; does not test a physical key")
    parser.add_argument("--idle-seconds", type=float, default=2)
    options = parser.parse_args()
    if not 0 <= options.idle_seconds <= 45:
        parser.error("idle seconds must be between zero and 45")
    result = run(options.root.resolve(), [str(options.binary.resolve())], execute=lambda *args: execute(*args, options.message_probe, options.idle_seconds))
    print(json.dumps(result, indent=2))
    raise SystemExit(result["exit_code"])


if __name__ == "__main__":
    main()
