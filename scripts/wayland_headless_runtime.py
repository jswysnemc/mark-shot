"""【贴图回归】【双输出会话】仅启动私有 headless Sway，不连接真实桌面或导入会话环境"""

import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import time


CONFIG = '''
xwayland disable
output HEADLESS-1 mode 1200x900 position 0 0 scale 1.25
output HEADLESS-2 mode 1200x900 position 960 0 scale 1.5
seat seat0 attach *
focus_follows_mouse yes
default_border none
for_window [app_id="mark-shot-pin"] floating enable
'''


class HeadlessRuntime:
    """【贴图回归】【隔离资源】管理 root 下 sway 进程和日志，binary 可为未安装的独立测试工具"""

    def __init__(self, root, binary, renderer="pixman"):
        """【贴图回归】【会话构造】保存 root、binary，不启动进程；environment 只属于本实例"""
        self.root = root.resolve()
        self.binary = binary.resolve()
        self.process = None
        self.log = None
        self.runtime = None
        self.environment = dict(os.environ)
        self.renderer = renderer

    def __enter__(self):
        """【贴图回归】【会话启动】创建私有目录，启动两输出并校验套接字；返回环境，失败时关闭进程"""
        try:
            self.root.mkdir(parents=True, exist_ok=False, mode=0o700)
            config = self.root / "config.kdl"
            config.write_text(CONFIG)
            # 1. 【贴图回归】【套接字路径】Unix 套接字路径有固定上限，长证据目录不能直接作为运行目录
            self.runtime = tempfile.TemporaryDirectory(prefix="mark-shot-wl-")
            runtime = Path(self.runtime.name)
            for key in ("DISPLAY", "WAYLAND_DISPLAY", "NIRI_SOCKET", "SWAYSOCK", "MARK_SHOT_TEST_X11_DISPLAY"):
                self.environment.pop(key, None)
            self.environment.update(XDG_RUNTIME_DIR=str(runtime), XDG_CONFIG_HOME=str(self.root / "config"),
                                    XDG_DATA_HOME=str(self.root / "data"), XDG_STATE_HOME=str(self.root / "state"),
                                    WLR_BACKENDS="headless", WLR_RENDERER=self.renderer, WLR_HEADLESS_OUTPUTS="2")
            library = self.binary.parent.parent / "lib"
            self.environment["LD_LIBRARY_PATH"] = str(library)
            self.log = (self.root / "sway.log").open("w")
            self.process = subprocess.Popen([str(self.binary), "-c", str(config)], env=self.environment,
                                            stdout=subprocess.DEVNULL, stderr=self.log, start_new_session=True)
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline:
                if self.process.poll() is not None:
                    raise RuntimeError(f"headless compositor exited: {self.process.returncode}; see {self.root / 'sway.log'}")
                sockets = list(runtime.glob(f"sway-ipc.*.{self.process.pid}.sock"))
                wayland = list(runtime.glob("wayland-*"))
                wayland = [path for path in wayland if path.is_socket()]
                if len(sockets) == 1 and len(wayland) == 1:
                    self.environment.update(SWAYSOCK=str(sockets[0]), WAYLAND_DISPLAY=wayland[0].name)
                    break
                time.sleep(0.04)
            else:
                raise RuntimeError("headless sockets did not appear")
            outputs = json.loads(subprocess.check_output([str(self.binary.parent / "swaymsg"), "-r", "-t", "get_outputs"],
                                                        env=self.environment))
            expected = {"HEADLESS-1": {"x": 0, "y": 0, "width": 960, "height": 720},
                        "HEADLESS-2": {"x": 960, "y": 0, "width": 800, "height": 600}}
            if {item["name"]: item["rect"] for item in outputs} != expected:
                raise RuntimeError(f"unexpected headless layout: {outputs}")
            (self.root / "runtime.json").write_text(json.dumps({"sway_pid": self.process.pid, "outputs": outputs,
                                                              "wayland_display": self.environment["WAYLAND_DISPLAY"]}, indent=2) + "\n")
            return self.environment
        except BaseException:
            self.close()
            raise

    def close(self):
        """【贴图回归】【会话释放】只终止本实例进程组，不依赖旧记录；随后关闭普通日志文件"""
        if self.process and self.process.poll() is None:
            os.killpg(self.process.pid, signal.SIGTERM)
            try:
                self.process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                os.killpg(self.process.pid, signal.SIGKILL)
                self.process.wait(timeout=3)
        if self.log:
            self.log.close()
        if self.runtime:
            self.runtime.cleanup()

    def __exit__(self, *_arguments):
        """【贴图回归】【上下文结束】释放本实例资源，不抑制 _arguments 中的异常"""
        self.close()
