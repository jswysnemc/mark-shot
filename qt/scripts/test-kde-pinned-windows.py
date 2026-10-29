#!/usr/bin/env python3
"""在独立 D-Bus、显示服务和临时配置中验证 KWin 窗口，退出时清理所有子进程。"""

import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile


def parse_arguments():
    """读取检查程序和隔离显示参数，返回参数对象。"""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--kwin", default="kwin_wayland")
    parser.add_argument("--platform", choices=("wayland", "xcb"), default="wayland")
    parser.add_argument("--language", choices=("en", "zh"), default="en")
    parser.add_argument("--scale", type=float, default=1)
    parser.add_argument("--outputs", type=int, default=1)
    parser.add_argument("--idle-check", action="store_true")
    parser.add_argument("--artifacts", type=Path, required=True)
    return parser.parse_args()


def stop_process_group(process):
    """结束本次创建的进程组；参数为隔离启动进程，无返回值。"""
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        return
    try:
        process.wait(timeout=3)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait(timeout=3)


def run_check(args, root):
    """在临时目录 root 内运行 args 指定的检查，返回退出码并保存验证日志。"""
    # 1. 【KDE测试】【会话隔离】所有写入和显示连接均使用本次创建的临时目录
    env = os.environ.copy()
    # KWin 6.8 的虚拟后端不再支持 QPainter，强制合成模式会让合成器退出，窗口收不到帧回调
    for key in ("DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET", "NIRI_SOCKET", "SWAYSOCK",
                "QT_PLUGIN_PATH", "QT_QPA_PLATFORM", "QT_WAYLAND_SHELL_INTEGRATION", "KWIN_COMPOSE"):
        env.pop(key, None)
    for key, folder in (("XDG_CONFIG_HOME", "config"), ("XDG_DATA_HOME", "data"),
                        ("XDG_CACHE_HOME", "cache"), ("XDG_RUNTIME_DIR", "runtime"), ("TMPDIR", "tmp")):
        path = root / folder
        path.mkdir(mode=0o700)
        env[key] = str(path)
    env.update(XDG_CURRENT_DESKTOP="KDE", XDG_SESSION_DESKTOP="KDE", DESKTOP_SESSION="plasma",
               XDG_SESSION_TYPE="wayland", QT_QPA_PLATFORMTHEME="generic",
               QT_STYLE_OVERRIDE="Fusion", QT_NO_XDG_DESKTOP_PORTAL="1",
               GIO_USE_VFS="local", GVFS_DISABLE_FUSE="1",
               MARK_SHOT_KDE_TEST_RUNTIME=str(root), MARK_SHOT_KDE_TEST_LANGUAGE=args.language,
               MARK_SHOT_LANG=args.language)
    env.pop("MARK_SHOT_KDE_IDLE_CHECK", None)
    if args.idle_check:
        env["MARK_SHOT_KDE_IDLE_CHECK"] = "1"
    result_file = root / "result.json"
    binary = args.binary.resolve()
    session = root / "session.py"
    session.write_text("#!/usr/bin/env python3\nimport json, os, subprocess\nfrom pathlib import Path\n"
                       f"os.environ['QT_QPA_PLATFORM'] = {args.platform!r}\n"
                       f"os.environ['QT_PLUGIN_PATH'] = {str(binary.parent)!r}\n"
                       f"result = subprocess.run([{str(binary)!r}])\n"
                       f"Path({str(result_file)!r}).write_text(json.dumps({{'returncode': result.returncode}}))\n")
    session.chmod(0o700)
    command = ["dbus-run-session", "--", args.kwin, "--virtual", "--xwayland",
               "--socket", "mark-shot-kde-test", "--width", "1280", "--height", "900",
               "--scale", str(args.scale), "--output-count", str(args.outputs),
               "--no-lockscreen", "--no-global-shortcuts", "--no-kactivities",
               "--exit-with-session", str(session)]

    # 2. 【KDE测试】【执行清理】整个测试属于独立进程组，失败或超时也会回收
    log_path = args.artifacts / "kwin.log"
    with log_path.open("w") as log:
        process = subprocess.Popen(command, env=env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        try:
            process.wait(timeout=60)
        except subprocess.TimeoutExpired:
            print("FAIL isolated KWin check timed out", file=sys.stderr)
        finally:
            stop_process_group(process)
    lines = log_path.read_text().splitlines()
    for line in lines:
        if line.startswith(("PASS ", "FAIL ", "Observed:", "ALL ", "Checking ")):
            print(line)
    if not result_file.exists():
        print("\n".join(lines[-20:]), file=sys.stderr)
        print(f"FAIL check did not complete; log: {log_path}", file=sys.stderr)
        return 1
    result = json.loads(result_file.read_text())
    result.update(platform=args.platform, language=args.language, scale=args.scale, outputs=args.outputs,
                  kwin=subprocess.check_output([args.kwin, "--version"], text=True).strip())
    (args.artifacts / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result["returncode"]


def main():
    """准备验证目录并执行隔离测试，返回检查退出码。"""
    args = parse_arguments()
    args.artifacts = args.artifacts.resolve()
    args.artifacts.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="mark-shot-kde-") as name:
        return run_check(args, Path(name))


if __name__ == "__main__":
    sys.exit(main())
