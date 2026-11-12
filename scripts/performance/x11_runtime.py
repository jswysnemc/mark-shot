"""【跨平台性能】【私有桌面】管理性能采样专用 Xvfb 与可选 Wine 会话。"""

import os
import select
import subprocess
from contextlib import contextmanager


@contextmanager
def runtime(root, size, runner=None):
    """【跨平台性能】【环境生命周期】按 root、size 和 runner 创建私有服务器，返回环境变量，退出时仅清理本次资源。"""
    env = {key: value for key, value in os.environ.items()
           if key not in ("WAYLAND_DISPLAY", "WAYLAND_SOCKET", "NIRI_SOCKET")}
    env.update(XDG_RUNTIME_DIR=str(root), XDG_CONFIG_HOME=str(root / "config"),
               XDG_STATE_HOME=str(root / "state"), XDG_DATA_HOME=str(root / "data"),
               MARK_SHOT_BACKEND="x11", XDG_SESSION_TYPE="x11")
    read, write = os.pipe()
    with (root / "xvfb.log").open("w") as output:
        server = subprocess.Popen(
            ["Xvfb", "-displayfd", str(write), "-screen", "0", f"{size[0]}x{size[1]}x24", "-nolisten", "tcp"],
            env=env, pass_fds=(write,), stdout=output, stderr=output)
        os.close(write)
        try:
            if not select.select([read], [], [], 8)[0]:
                raise RuntimeError("private X11 server did not start")
            number = os.read(read, 64).decode().strip()
            if not number.isdecimal():
                raise RuntimeError("private X11 server returned invalid display")
            env["DISPLAY"] = f":{number}"
            if runner:
                # 1. 【Windows性能】【独立前缀】模拟环境只属于本次私有桌面，不读取或修改用户 Wine 配置
                env.update(WINEPREFIX=str(root / "wine"), WINEDEBUG="-all", WINEDLLOVERRIDES="mscoree,mshtml=")
                subprocess.run(["wineboot", "-u"], env=env, stdout=output, stderr=output, check=True, timeout=30)
            yield env
        finally:
            os.close(read)
            try:
                if runner and env.get("WINEPREFIX") == str(root / "wine"):
                    subprocess.run(["wineserver", "-k"], env=env, stdout=subprocess.DEVNULL,
                                   stderr=subprocess.DEVNULL, timeout=8, check=False)
            finally:
                server.terminate()
                server.wait(timeout=5)
