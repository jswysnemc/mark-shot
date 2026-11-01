#!/usr/bin/env python3
"""在隔离 Xvfb、D-Bus 和 niri 会话中测试录制，不连接用户桌面。"""

import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import tempfile
import time


def run_session(executable: str) -> int:
    """【录制测试】【隔离会话】在临时 niri 内运行 executable，返回测试退出码。"""
    with tempfile.TemporaryDirectory(prefix="mark-shot-recording-wayland-") as directory:
        root = Path(directory)
        runtime = root / "runtime"
        runtime.mkdir(mode=0o700)
        config = root / "niri.kdl"
        config.write_text("hotkey-overlay { skip-at-startup; }\nanimations { off; }\n")
        env = dict(os.environ, XDG_RUNTIME_DIR=str(runtime), HOME=str(root),
                   MARK_SHOT_RECORDING_TEST_RUNTIME=str(runtime),
                   XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"),
                   XDG_CACHE_HOME=str(root / "cache"), WINIT_UNIX_BACKEND="x11",
                   LIBGL_ALWAYS_SOFTWARE="1", QT_QPA_PLATFORM="wayland",
                   XDG_CURRENT_DESKTOP="niri", XDG_SESSION_TYPE="wayland",
                   QT_LOGGING_RULES="", QT_FORCE_STDERR_LOGGING="1")
        for name in ("WAYLAND_DISPLAY", "NIRI_SOCKET", "WAYLAND_SOCKET", "QT_QPA_PLATFORM_PLUGIN_PATH"):
            env.pop(name, None)
        with (root / "niri.log").open("w+") as log:
            compositor = subprocess.Popen(["niri", "-c", str(config)], env=env, stdout=log, stderr=log)
            try:
                deadline = time.monotonic() + 15
                socket = None
                while time.monotonic() < deadline and compositor.poll() is None:
                    socket = next((p for p in runtime.glob("wayland-*")
                                   if stat.S_ISSOCK(p.stat().st_mode)), None)
                    if socket:
                        break
                    time.sleep(0.05)
                if not socket:
                    log.seek(0)
                    print(log.read(), file=sys.stderr)
                    return 1
                env["WAYLAND_DISPLAY"] = str(socket)
                return subprocess.run([executable, "-o", "-,txt"], env=env, timeout=70).returncode
            finally:
                compositor.terminate()
                try:
                    compositor.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    compositor.kill()
                    compositor.wait()


def main() -> int:
    """【录制测试】【调度】隔离总线和显示服务器，返回测试结果，缺少工具返回跳过码 77。"""
    if len(sys.argv) == 3 and sys.argv[1] == "--session":
        return run_session(sys.argv[2])
    if len(sys.argv) != 2:
        raise SystemExit("Expected the wlroots capture test executable")
    for program in ("niri", "xvfb-run", "Xvfb", "dbus-run-session"):
        if not shutil.which(program):
            print(f"Skipping Wayland recording integration: missing {program}")
            return 77
    with tempfile.TemporaryDirectory(prefix="mark-shot-recording-bus-") as directory:
        config = Path(directory) / "session.conf"
        config.write_text('<busconfig><type>session</type><listen>unix:tmpdir=/tmp</listen>'
                          '<policy context="default"><allow send_destination="*"/>'
                          '<allow receive_sender="*"/><allow own="*"/></policy></busconfig>')
        return subprocess.call(["dbus-run-session", "--config-file=" + str(config), "--",
                                "xvfb-run", "-a", sys.executable, str(Path(__file__).resolve()),
                                "--session", sys.argv[1]])


if __name__ == "__main__":
    sys.exit(main())
