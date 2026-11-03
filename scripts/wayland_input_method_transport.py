"""【输入法测试】【客户端传输】编译测试协议客户端并记录完整状态，只接收已校验的私有环境"""

import hashlib
import json
import os
import re
import select
import subprocess
import time
from pathlib import Path


def protocol_evidence(text):
    """【输入法测试】【协议证据】从 text 分别识别 Rust 请求与事件格式，返回绑定、交付和提交计数；不能用全局广告代替实际绑定"""
    return {
        "native_protocol": bool(
            re.search(r"zwp_text_input_manager_v3@\d+\.get_text_input\(", text)
        )
        and bool(re.search(r"zwp_text_input_v3@\d+\.enable\(\)", text))
        and bool(re.search(r"zwp_text_input_v3@\d+\.set_surrounding_text\(", text))
        and bool(re.search(r"zwp_text_input_v3@\d+\.done(?:, \(|\()", text)),
        "preedit_delivered": bool(
            re.search(r"zwp_text_input_v3@\d+\.preedit_string(?:, \(|\()", text)
        ),
        "protocol_commits": len(re.findall(r"zwp_text_input_v3@\d+\.commit\(\)", text)),
    }


def build(root, xml):
    """【输入法测试】【客户端编译】在 root 从 xml 生成客户端，返回可执行路径；源文件和 XML 指纹写入记录"""
    root.mkdir(parents=True, exist_ok=True)
    source = Path(__file__).with_name("wayland_input_method_client.c")
    header, protocol = root / "input-method-client.h", root / "input-method-protocol.c"
    subprocess.run(
        ["wayland-scanner", "client-header", str(xml), str(header)], check=True
    )
    subprocess.run(
        ["wayland-scanner", "private-code", str(xml), str(protocol)], check=True
    )
    flags = subprocess.check_output(
        ["pkg-config", "--cflags", "--libs", "wayland-client"], text=True
    ).split()
    binary = root / "input-method-client"
    subprocess.run(
        [
            "cc",
            "-std=c11",
            "-O2",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-I",
            str(root),
            str(source),
            str(protocol),
            *flags,
            "-o",
            str(binary),
        ],
        check=True,
    )
    (root / "build.json").write_text(
        json.dumps(
            {
                "xml_sha256": hashlib.sha256(xml.read_bytes()).hexdigest(),
                "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
            },
            indent=2,
        )
        + "\n"
    )
    return binary


class InputMethod:
    """【输入法测试】【客户端生命周期】管理 binary、environment 与 root 下的协议记录"""

    def __init__(self, binary, environment, root):
        """【输入法测试】【连接构造】启动 binary 并校验就绪，environment 必须已经过私有会话验证，root 保存证据"""
        self.root = root
        self.buffer = b""
        self.states = []
        self.records = []
        self.content = {"hint": 0, "purpose": 0}
        self.log = (root / "input-method.log").open("w")
        self.process = subprocess.Popen(
            [str(binary)],
            env=environment,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=self.log,
        )
        try:
            self.wait(lambda line: line == "ready")
        except BaseException:
            self.close()
            raise

    def read(self, timeout):
        """【输入法测试】【状态读取】在 timeout 秒内读取一条完整协议行，返回文字；超时返回空值，退出视为失败"""
        deadline = time.monotonic() + timeout
        while b"\n" not in self.buffer:
            if self.process.poll() is not None:
                raise AssertionError(f"input method exited: {self.process.returncode}")
            left = deadline - time.monotonic()
            if left <= 0 or not select.select([self.process.stdout], [], [], left)[0]:
                return None
            value = os.read(self.process.stdout.fileno(), 65536)
            if not value:
                raise AssertionError("input method closed its output")
            self.buffer += value
        line, self.buffer = self.buffer.split(b"\n", 1)
        line = line.decode("utf-8")
        self.record(line)
        return line

    def record(self, line):
        """【输入法测试】【状态记录】按 line 保存类型与完整状态，不推断字段用途，协议不可用时抛出断言；无返回值"""
        self.records.append(line)
        if line.startswith("content\t"):
            _, hint, purpose = line.split("\t")
            self.content = {"hint": int(hint), "purpose": int(purpose)}
        if line.startswith("state\t"):
            _, serial, active, cursor, anchor, text = line.split("\t")
            self.states.append(
                {
                    "serial": int(serial),
                    "active": bool(int(active)),
                    "cursor": int(cursor),
                    "anchor": int(anchor),
                    "text": bytes.fromhex(text).decode("utf-8"),
                    **self.content,
                }
            )
        if line == "unavailable":
            raise AssertionError("the private seat already has an input method")

    def wait(self, predicate, timeout=4):
        """【输入法测试】【事件等待】等待 predicate 接受的协议行，timeout 控制总秒数；返回匹配行，超时保留已有记录并失败"""
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            line = self.read(max(0, deadline - time.monotonic()))
            if line is not None and predicate(line):
                return line
        raise AssertionError(f"input method wait timed out: {self.records[-8:]}")

    def state(self, text=None, active=True):
        """【输入法测试】【状态等待】等待新状态匹配 text、active，返回状态；text 为空值时不比较文字"""
        if (
            self.states
            and self.states[-1]["active"] == active
            and (text is None or self.states[-1]["text"] == text)
        ):
            return self.states[-1]
        self.wait(
            lambda line: (
                line.startswith("state\t")
                and self.states[-1]["active"] == active
                and (text is None or self.states[-1]["text"] == text)
            )
        )
        return self.states[-1]

    def command(self, value):
        """【输入法测试】【命令暂存】发送 value 并等待确认，返回协议确认文字；禁止内嵌换行"""
        if "\n" in value:
            raise ValueError("commands must contain a single line")
        self.process.stdin.write((value + "\n").encode())
        self.process.stdin.flush()
        return self.wait(lambda line: line == "ack" or line.startswith("sent\t"))

    def batch(self, commit=None, preedit=None, cursor=None, delete=None):
        """【输入法测试】【批次投递】暂存 commit、preedit、cursor、delete 后原子确认，返回发送序号；候选位置采用 UTF-8 字节"""
        if commit is not None:
            self.command("commit\t" + commit.encode().hex())
        if preedit is not None:
            begin, end = (
                cursor
                if cursor is not None
                else (len(preedit.encode()), len(preedit.encode()))
            )
            self.command(f"preedit\t{begin}\t{end}\t{preedit.encode().hex()}")
        if delete is not None:
            self.command(f"delete\t{delete[0]}\t{delete[1]}")
        return self.command("send")

    def close(self):
        """【输入法测试】【连接关闭】释放本实例进程和日志，保存全部协议状态，无参数、无返回值"""
        if self.process.poll() is None:
            try:
                self.process.stdin.write(b"quit\n")
                self.process.stdin.flush()
                self.process.wait(timeout=3)
            except (BrokenPipeError, subprocess.TimeoutExpired):
                self.process.kill()
                self.process.wait(timeout=3)
        self.log.close()
        (self.root / "input-method-states.json").write_text(
            json.dumps(self.states, ensure_ascii=False, indent=2) + "\n"
        )
        (self.root / "input-method-wire.txt").write_text("\n".join(self.records) + "\n")
