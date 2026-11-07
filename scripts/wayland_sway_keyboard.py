"""【私有键盘】【标准输入】管理持续 Wayland 键盘，标准键码和修饰键与真实 US 键盘相同"""

import hashlib
import os
from pathlib import Path
import select
import subprocess
import tempfile

from wayland_test_keyboard import canonical_chord

PROJECT = Path(__file__).resolve().parents[1]
CHARACTERS = dict(zip("1234567890-=", range(2, 14)))
CHARACTERS.update(zip("qwertyuiop[]", range(16, 28)))
CHARACTERS.update(zip("asdfghjkl;'`", range(30, 42)))
CHARACTERS.update(zip("zxcvbnm,./", range(44, 54)))
CHARACTERS["\\"] = 43
NAMED = {"Escape": 1, "Return": 28, "space": 57, "Tab": 15, "BackSpace": 14,
         "Delete": 111, "Insert": 110, "Home": 102, "End": 107, "Prior": 104,
         "Next": 109, "Left": 105, "Right": 106, "Up": 103, "Down": 108,
         "bracketleft": 26, "bracketright": 27, "minus": 12, "equal": 13,
         "semicolon": 39, "apostrophe": 40, "comma": 51, "period": 52,
         "slash": 53, "backslash": 43, "grave": 41}
NAMED.update({f"F{number}": 58 + number for number in range(1, 11)})
NAMED.update(F11=87, F12=88)


def chord(value):
    """【私有键盘】【组合转换】将 value 转为标准键码和抽象修饰位，返回二元组，未知键名直接拒绝"""
    parts = canonical_chord(value).split("+")
    code = CHARACTERS.get(parts[-1], NAMED.get(parts[-1]))
    if code is None:
        raise ValueError(f"unsupported private keyboard key: {value}")
    flags = {"ctrl": 1, "shift": 2, "alt": 4, "super": 8}
    return code, sum(flags[modifier] for modifier in parts[:-1])


def build():
    """【私有键盘】【客户端构建】按源文件与协议指纹创建私有测试工具，返回可执行路径，无参数"""
    source = Path(__file__).with_name("wayland_sway_keyboard_client.c")
    candidates = list((Path.home() / ".cargo/registry/src").glob("*/wayland-protocols-misc-*/protocols/virtual-keyboard-unstable-v1.xml"))
    if not candidates:
        raise RuntimeError("virtual-keyboard protocol XML was not found in the installed Rust protocol sources")
    xml = sorted(candidates)[-1]
    digest = hashlib.sha256(source.read_bytes() + xml.read_bytes()).hexdigest()
    root = PROJECT / "target/test-tools/sway-keyboard" / digest
    binary = root / "keyboard"
    if binary.exists():
        return binary
    root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=root) as directory:
        temporary = Path(directory)
        header, protocol = temporary / "virtual-keyboard-client.h", temporary / "virtual-keyboard-protocol.c"
        subprocess.run(["wayland-scanner", "client-header", str(xml), str(header)], check=True)
        subprocess.run(["wayland-scanner", "private-code", str(xml), str(protocol)], check=True)
        flags = subprocess.check_output(["pkg-config", "--cflags", "--libs", "wayland-client", "xkbcommon"], text=True).split()
        subprocess.run(["cc", "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror", "-I", str(temporary), str(source), str(protocol),
                        *flags, "-o", str(temporary / "keyboard")], check=True)
        os.replace(temporary / "keyboard", binary)
    return binary


class Keyboard:
    """【私有键盘】【设备生命周期】在同一个协议连接上投递完整按键，不替换或重新创建座席键盘"""

    def __init__(self, environment):
        """【私有键盘】【设备初始化】使用已经校验的 environment 启动客户端并等待映射就绪，返回设备对象"""
        self.process = subprocess.Popen([str(build())], env=environment, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        try:
            self.wait("ready")
        except BaseException:
            self.close()
            raise

    def wait(self, expected):
        """【私有键盘】【确认等待】等待客户端返回 expected，失败或超时抛出异常，无返回值"""
        if not select.select([self.process.stdout], [], [], 4)[0] or self.process.stdout.readline().decode().strip() != expected:
            raise RuntimeError(f"private keyboard did not acknowledge {expected}; status={self.process.poll()}")

    def key(self, value):
        """【私有键盘】【按键投递】按 value 发送同一设备的真实按下与释放，等待协议确认，无返回值"""
        code, flags = chord(value)
        self.process.stdin.write(f"{code} {flags}\n".encode())
        self.process.stdin.flush()
        self.wait("done")

    def close(self):
        """【私有键盘】【连接释放】关闭本次设备并回收协议进程，不向其他会话发送信号，无参数、无返回值"""
        self.process.stdin.close()
        try:
            self.process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait(timeout=3)
        self.process.stdout.close()

    def modifiers(self, flags):
        """【私有键盘】【修饰键持有】按 flags 的Ctrl、Shift、Alt、Super位更新同一设备，返回空值，零值释放全部修饰键"""
        if not isinstance(flags, int) or not 0 <= flags <= 15:
            raise ValueError("private modifier flags must be in 0..15")
        self.process.stdin.write(f"modifiers {flags}\n".encode())
        self.process.stdin.flush()
        self.wait("done")
