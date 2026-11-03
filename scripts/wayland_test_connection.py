"""【Wayland 测试】【协议连接】仓库内的最小客户端传输，不读取本机工具目录"""

import os
from pathlib import Path
import socket
import struct
import time


def wire_string(value):
    """【Wayland 测试】【字符串编码】将 value 编码为零结尾且四字节对齐的协议字符串，返回字节串"""
    encoded = value.encode() + b"\0"
    return struct.pack("=I", len(encoded)) + encoded + b"\0" * (-len(encoded) % 4)


class TestConnection:
    """【Wayland 测试】【传输生命周期】管理一个协议连接、注册表和对象编号"""

    def __init__(self):
        """【Wayland 测试】【连接创建】连接当前已经隔离的环境并读取注册表，无参数、无返回值，失败关闭连接"""
        runtime, display = os.environ.get("XDG_RUNTIME_DIR"), os.environ.get("WAYLAND_DISPLAY")
        if not runtime or not display:
            raise RuntimeError("private Wayland environment is not configured")
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.buffer = bytearray()
        self.next_id = 2
        self.globals = {}
        self.outputs = []
        try:
            self.socket.connect(str(Path(runtime) / display))
            self.registry = self.new_id()
            self.send(1, 1, struct.pack("=I", self.registry))
            self.roundtrip()
        except BaseException:
            self.close()
            raise

    def new_id(self):
        """【Wayland 测试】【对象分配】返回本连接中新的客户端对象编号，无参数"""
        result = self.next_id
        self.next_id += 1
        return result

    def send(self, object_id, opcode, body=b""):
        """【Wayland 测试】【请求发送】把 object_id、opcode 和 body 编码到连接，无返回值"""
        if len(body) % 4 or len(body) + 8 > 65535:
            raise ValueError("invalid Wayland request size")
        self.socket.sendall(struct.pack("=II", object_id, ((8 + len(body)) << 16) | opcode) + body)

    def event(self, object_id, opcode, body):
        """【Wayland 测试】【事件处理】处理 object_id、opcode、body 的注册表或协议错误，不返回值"""
        if object_id == 1 and opcode == 0:
            raise RuntimeError(f"private Wayland protocol error: {body!r}")
        if object_id == self.registry and opcode == 0:
            name, length = struct.unpack_from("=II", body)
            if length < 1 or len(body) < 8 + ((length + 3) & ~3) + 4:
                raise RuntimeError("invalid Wayland registry event")
            interface = body[8:8 + length - 1].decode()
            version = struct.unpack_from("=I", body, 8 + ((length + 3) & ~3))[0]
            self.globals[interface] = (name, version)
            if interface == "wl_output":
                self.outputs.append((name, version))

    def roundtrip(self, timeout=1.0):
        """【Wayland 测试】【往返校验】在 timeout 秒内等待此前请求处理完毕，无返回值，超时或断连抛出异常"""
        callback = self.new_id()
        self.send(1, 0, struct.pack("=I", callback))
        deadline = time.monotonic() + timeout
        while True:
            while len(self.buffer) >= 8:
                object_id, header = struct.unpack_from("=II", self.buffer)
                size, opcode = header >> 16, header & 0xffff
                if size < 8 or size % 4:
                    raise RuntimeError("invalid Wayland event size")
                if len(self.buffer) < size:
                    break
                body = bytes(self.buffer[8:size])
                del self.buffer[:size]
                self.event(object_id, opcode, body)
                if object_id == callback and opcode == 0:
                    return
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError("private Wayland roundtrip timed out")
            self.socket.settimeout(remaining)
            received = self.socket.recv(4096)
            if not received:
                raise ConnectionError("private Wayland connection closed")
            self.buffer.extend(received)

    def bind_name(self, name, interface, version):
        """【Wayland 测试】【注册表绑定】以 version 绑定 name 对应的 interface，返回客户端对象编号"""
        object_id = self.new_id()
        self.send(self.registry, 0, struct.pack("=I", name) + wire_string(interface) + struct.pack("=II", version, object_id))
        return object_id

    def bind(self, interface, version):
        """【Wayland 测试】【全局绑定】绑定 interface，version 为请求版本上限，返回对象编号，缺少协议时抛出异常"""
        if interface not in self.globals:
            raise RuntimeError(f"private compositor does not advertise {interface}")
        name, advertised = self.globals[interface]
        return self.bind_name(name, interface, min(version, advertised))

    def close(self):
        """【Wayland 测试】【连接释放】关闭本连接，不影响其他客户端，无参数、无返回值"""
        self.socket.close()
