#!/usr/bin/env python3
"""【Windows回归】【私有运行】只使用本次Wine前缀和不可见Xvfb，不连接用户显示、桌面或既有Wine服务"""

import argparse
import json
import os
from pathlib import Path
import select
import subprocess
import tempfile


def run(root, command, execute=None):
    """【Windows回归】【隔离执行】在新root保存证据，按command与可选execute执行私有流程，返回状态并回收本次显示与Wine服务"""
    root.mkdir(parents=True, exist_ok=False)
    with tempfile.TemporaryDirectory(prefix="mark-shot-win-private-") as temporary:
        private = Path(temporary)
        runtime = private / "runtime"
        runtime.mkdir(mode=0o700)
        environment = dict(os.environ)
        for name in ("WAYLAND_DISPLAY", "NIRI_SOCKET", "SWAYSOCK", "DBUS_SESSION_BUS_ADDRESS", "XAUTHORITY"):
            environment.pop(name, None)
        environment.update(WINEPREFIX=str(root / "wine-prefix"), WINEDEBUG="-all",
            WINEDLLOVERRIDES="mscoree,mshtml=;winewayland.drv=;winex11.drv=b", XDG_RUNTIME_DIR=str(runtime),
            MARK_SHOT_PRIVATE_WINDOWS_TEST="1")
        reader, writer = os.pipe()
        with (root / "xvfb.log").open("w") as display_log:
            display = subprocess.Popen(["Xvfb", "-displayfd", str(writer), "-screen", "0", "1280x800x24", "-nolisten", "tcp", "-noreset"],
                pass_fds=(writer,), env=environment, stdout=display_log, stderr=display_log)
            os.close(writer)
            try:
                if not select.select([reader], [], [], 6)[0]:
                    raise RuntimeError("private Windows Xvfb did not report its display")
                number = os.read(reader, 64).decode().strip()
                if not number.isdigit():
                    raise RuntimeError("private Windows Xvfb returned an invalid display")
                environment["DISPLAY"] = ":" + number
                with (root / "wineboot.log").open("w") as boot_log:
                    subprocess.run(["wineboot", "--init"], env=environment, cwd=root,
                        stdout=boot_log, stderr=boot_log, check=True, timeout=45)
                with (root / "program.log").open("w") as log:
                    if execute is None:
                        result = subprocess.run(["wine", *command], env=environment, cwd=root,
                            stdout=log, stderr=log, timeout=120)
                    else:
                        result = execute(command, environment, root, log)
                report = {"exit_code": result.returncode, "display": environment["DISPLAY"],
                    "wine_prefix": environment["WINEPREFIX"], "command": command,
                    "isolated_xvfb": True, "isolated_wine": True, "real_windows_validation": False}
                (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
                return report
            finally:
                os.close(reader)
                try:
                    subprocess.run(["wineserver", "-k"], env=environment, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=8)
                finally:
                    display.terminate()
                    try:
                        display.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        display.kill()
                        display.wait(timeout=3)


def main():
    """【Windows回归】【命令入口】读取新root与待执行程序参数，输出私有回归结果，无参数、无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    options = parser.parse_args()
    command = options.command[1:] if options.command[:1] == ["--"] else options.command
    if not command:
        parser.error("a Windows executable is required")
    command[0] = str(Path(command[0]).resolve())
    result = run(options.root.resolve(), command)
    print(json.dumps(result, indent=2))
    raise SystemExit(result["exit_code"])


if __name__ == "__main__":
    main()
