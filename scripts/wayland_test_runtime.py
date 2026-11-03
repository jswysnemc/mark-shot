#!/usr/bin/env python3
"""【Wayland 测试】【独立运行器】在不可见的 Xvfb 显示器中运行 niri，仅向独立 Wayland 会话发送输入"""

import argparse
import json
import os
from pathlib import Path
import select
import signal
import subprocess
import time


CONFIG = '''
input {
    keyboard {
        repeat-rate 0
        repeat-delay 10000
    }
    focus-follows-mouse
}
output "winit" {
    scale 1.2
    transform "flipped-180"
    mode "1280x800"
}
prefer-no-csd
layout {
    gaps 8
    default-column-width {
        proportion 0.5
    }
    focus-ring {
        width 2
    }
}
hotkey-overlay {
    skip-at-startup
}
gestures {
    hot-corners {
        off
    }
}
'''


class TestRuntime:
    """【Wayland 测试】【会话生命周期】管理 root 中的独立显示服务器、合成器及验证状态"""

    def __init__(self, root, scale=1.2):
        """【测试运行器】【状态构造】使用 root 保存配置与日志，scale 为输出比例；不启动进程，无返回值"""
        self.root = root.resolve()
        if not 0.5 <= scale <= 4.0:
            raise ValueError("test output scale must be between 0.5 and 4")
        self.scale = scale
        self.processes = []
        self.logs = []
        self.environment = dict(os.environ)

    def start(self):
        """【测试运行器】【会话启动】启动独立 Xvfb 与 niri 并验证套接字；返回隔离环境，失败时关闭已启动进程"""
        self.root.mkdir(parents=True, exist_ok=True, mode=0o700)
        self.root.chmod(0o700)
        child = self.root / "niri-child"
        child.mkdir(exist_ok=True)
        config = child / "config.kdl"
        config.write_text(CONFIG.replace("scale 1.2", f"scale {self.scale}"))
        for key in ("WAYLAND_DISPLAY", "NIRI_SOCKET", "WINIT_UNIX_BACKEND"):
            self.environment.pop(key, None)
        self.environment.update(XDG_RUNTIME_DIR=str(self.root), XDG_CONFIG_HOME=str(self.root / "config"), XDG_DATA_HOME=str(self.root / "data"), XDG_STATE_HOME=str(self.root / "state"))
        subprocess.run(["niri", "validate", "-c", str(config)], env=self.environment, stdout=subprocess.DEVNULL, stderr=self._log("config-validation.log"), check=True)

        # 1. 【测试运行器】【会话启动】显示服务器自行分配未占用编号，不连接或关闭真实桌面服务
        read, write = os.pipe()
        try:
            log = self._log("xvfb.log")
            process = subprocess.Popen(["Xvfb", "-displayfd", str(write), "-screen", "0", "1280x800x24", "-nolisten", "tcp"], env=self.environment, stdout=subprocess.DEVNULL, stderr=log, pass_fds=(write,), start_new_session=True)
            self.processes.append(process)
            os.close(write)
            write = None
            if not select.select([read], [], [], 8)[0]:
                raise RuntimeError("the independent Xvfb display did not become ready")
            display = os.read(read, 64).decode().strip()
            if not display.isdecimal():
                raise RuntimeError(f"Xvfb returned an invalid display number: {display!r}")
            self.environment["DISPLAY"] = f":{display}"
            self.environment["MARK_SHOT_TEST_X11_DISPLAY"] = f":{display}"
        finally:
            os.close(read)
            if write is not None:
                os.close(write)

        # 2. 【测试运行器】【会话启动】只在私有运行目录创建套接字，现有嵌套会话保持原样
        process = subprocess.Popen(["niri", "-c", str(config)], env=self.environment, stdout=subprocess.DEVNULL, stderr=self._log("niri.log"), start_new_session=True)
        self.processes.append(process)
        deadline = time.monotonic() + 10
        socket = None
        while time.monotonic() < deadline:
            if process.poll() is not None:
                raise RuntimeError(f"the independent niri exited with status {process.returncode}; see {self.root / 'niri.log'}")
            matching = list(self.root.glob(f"niri.wayland-*.{process.pid}.sock"))
            if len(matching) == 1:
                socket = matching[0]
                break
            time.sleep(0.02)
        if socket is None:
            raise RuntimeError("the independent niri socket did not appear")
        wayland = socket.name.split(".")[1]
        if not (self.root / wayland).exists():
            raise RuntimeError("niri published IPC before its Wayland socket became available")
        self.environment.update(WAYLAND_DISPLAY=wayland, NIRI_SOCKET=str(socket))
        (child / "state").write_text(f"CHILD_DISPLAY={wayland}\nCHILD_SOCKET={socket}\nCHILD_PID={process.pid}\n")

        # 3. 【测试运行器】【会话启动】从新合成器读取实际输出，确认后才允许运行回归
        outputs = json.loads(subprocess.check_output(["niri", "msg", "-j", "outputs"], env=self.environment))
        if len(outputs) != 1 or "winit" not in outputs:
            raise RuntimeError(f"the independent output is ambiguous: {outputs}")
        (self.root / "runtime.json").write_text(json.dumps({"xvfb_pid": self.processes[0].pid, "niri_pid": process.pid, "x11_display": self.environment["DISPLAY"], "wayland_display": wayland, "niri_socket": str(socket), "outputs": outputs}, indent=2) + "\n")
        return self.environment

    def _log(self, name):
        """【测试运行器】【日志文件】打开 root 下 name 普通文件；返回句柄，避免后台进程保留输出管道"""
        log = (self.root / name).open("w")
        self.logs.append(log)
        return log

    def close(self):
        """【测试运行器】【会话关闭】只终止本实例启动的进程组并关闭日志；不读取旧 PID 记录，无返回值"""
        for process in reversed(self.processes):
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGTERM)
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=3)
        self.processes.clear()
        for log in self.logs:
            log.close()
        self.logs.clear()

    def __enter__(self):
        """【测试运行器】【上下文进入】启动并返回隔离环境；失败时释放已经创建的资源"""
        try:
            return self.start()
        except BaseException:
            self.close()
            raise

    def __exit__(self, *_arguments):
        """【测试运行器】【上下文退出】关闭本实例资源，不抑制 _arguments 中的异常；返回 False"""
        self.close()
        return False


def main():
    """【测试运行器】【入口】解析私有目录与命令，执行后关闭会话；返回子命令退出状态"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path("target/wayland-test-runtime"))
    parser.add_argument("--scale", type=float, default=1.2)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    options = parser.parse_args()
    command = options.command
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        parser.error("a test command is required after --")
    with TestRuntime(options.root, options.scale) as environment:
        return subprocess.run(command, env=environment).returncode


if __name__ == "__main__":
    raise SystemExit(main())
