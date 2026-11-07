#!/usr/bin/env python3
"""【Wayland协议回归】【私有代理】仅在已校验的私有Sway转发连接，隐藏Layer Shell并保留描述符传递"""

import argparse
import array
import errno
import json
import os
import selectors
import signal
import socket
from pathlib import Path

import wayland_text_sway_session as session
from wayland_protocol_filter import ProtocolFilter


class Stream:
    """【Wayland协议回归】【传输方向】保留输出缓冲与尚未交付的描述符，发送成功后关闭本地副本"""

    def __init__(self, connection, direction, parser):
        """【Wayland协议回归】【传输构造】connection 为本端套接字，direction 和 parser 标记协议方向，无返回值"""
        self.connection, self.direction, self.parser = connection, direction, parser
        self.peer = None
        self.pending = bytearray()
        self.fds = []
        self.received_fds = 0
        self.sent_fds = 0

    def receive(self):
        """【Wayland协议回归】【接收消息】读取本端消息并排入对端缓冲，无参数，EOF 返回假，其他结果返回真"""
        data, ancillary, flags, _address = self.connection.recvmsg(65536, socket.CMSG_SPACE(256 * 4), socket.MSG_CMSG_CLOEXEC)
        descriptors = []
        for level, kind, value in ancillary:
            if level == socket.SOL_SOCKET and kind == socket.SCM_RIGHTS:
                values = array.array("i")
                values.frombytes(value[:len(value) - len(value) % values.itemsize])
                descriptors.extend(values)
        # 1. 【Wayland协议回归】【描述符所有权】先交给对端缓冲持有，解析失败和传输截断时也统一回收
        self.peer.fds.extend(descriptors)
        self.received_fds += len(descriptors)
        if flags & (socket.MSG_CTRUNC | socket.MSG_TRUNC):
            raise RuntimeError("private Wayland forwarding truncated a message or descriptor")
        if not data:
            return False
        self.peer.pending.extend(self.parser.feed(self.direction, data))
        return True

    def send(self):
        """【Wayland协议回归】【发送消息】转发输出缓冲和描述符，无参数，保留未发送字节，不重复发送描述符"""
        ancillary = [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array("i", self.fds))] if self.fds else []
        count = self.connection.sendmsg([self.pending], ancillary)
        if count <= 0:
            raise RuntimeError("private Wayland forwarding sent an empty message")
        del self.pending[:count]
        self.sent_fds += len(self.fds)
        for descriptor in self.fds:
            os.close(descriptor)
        self.fds.clear()

    def close(self):
        """【Wayland协议回归】【方向回收】关闭本端与尚未交付的描述符，无参数、无返回值"""
        self.connection.close()
        for descriptor in self.fds:
            os.close(descriptor)
        self.fds.clear()


class Relay:
    """【Wayland协议回归】【连接路由】管理多对私有连接和双向可写通知，不使用周期轮询"""

    def __init__(self):
        """【Wayland协议回归】【路由构造】创建选择器与停止管道，无参数，不建立上游连接"""
        self.selector = selectors.DefaultSelector()
        self.reader, self.writer = socket.socketpair()
        self.reader.setblocking(False)
        self.writer.setblocking(False)
        self.selector.register(self.reader, selectors.EVENT_READ, "stop")
        self.streams = set()
        self.parsers = []
        self.errors = []
        self.received_fds = 0
        self.sent_fds = 0
        self.peer_resets = 0

    def add(self, client, server, hidden=("zwlr_layer_shell_v1",)):
        """【Wayland协议回归】【连接加入】将已连接的 client 和 server 按 hidden 筛选；返回解析器用于检查证据"""
        parser = ProtocolFilter(hidden)
        source, target = Stream(client, "client", parser), Stream(server, "server", parser)
        source.peer, target.peer = target, source
        for stream in (source, target):
            stream.connection.setblocking(False)
            self.selector.register(stream.connection, selectors.EVENT_READ, stream)
            self.streams.add(stream)
        self.parsers.append(parser)
        return parser

    def update(self, stream):
        """【Wayland协议回归】【等待更新】按 stream 是否有输出启用可写事件，无返回值，不等待空输出"""
        self.selector.modify(stream.connection, selectors.EVENT_READ | (selectors.EVENT_WRITE if stream.pending else 0), stream)

    def remove(self, stream):
        """【Wayland协议回归】【连接回收】关闭 stream 所属连接对，无返回值；仅统计本路由持有的描述符"""
        for endpoint in (stream, stream.peer):
            if endpoint not in self.streams:
                continue
            self.selector.unregister(endpoint.connection)
            self.streams.remove(endpoint)
            self.received_fds += endpoint.received_fds
            self.sent_fds += endpoint.sent_fds
            endpoint.close()

    def stop(self):
        """【Wayland协议回归】【停止通知】唤醒当前选择器，无参数、无返回值，不操作外部进程"""
        try:
            self.writer.send(b"s")
        except BlockingIOError:
            pass

    def run(self, accept=None):
        """【Wayland协议回归】【转发循环】处理消息与描述符，accept 可加入新连接；停止通知到达时返回，无周期期限"""
        while True:
            for key, mask in self.selector.select():
                if key.data == "stop":
                    return
                if key.data == "listen":
                    accept()
                    continue
                stream = key.data
                if stream not in self.streams:
                    continue
                try:
                    if mask & selectors.EVENT_READ:
                        if not stream.receive():
                            self.remove(stream)
                            continue
                        self.update(stream.peer)
                    if mask & selectors.EVENT_WRITE:
                        stream.send()
                        self.update(stream)
                except BlockingIOError:
                    continue
                except (OSError, ValueError, RuntimeError) as error:
                    if isinstance(error, OSError) and error.errno in (errno.ECONNRESET, errno.EPIPE):
                        self.peer_resets += 1
                    else:
                        self.errors.append(str(error))
                    self.remove(stream)

    def close(self):
        """【Wayland协议回归】【路由关闭】回收全部自有连接、管道和选择器，无参数、无返回值"""
        for stream in list(self.streams):
            self.remove(stream)
        self.reader.close()
        self.writer.close()
        self.selector.close()

    def report(self):
        """【Wayland协议回归】【路由证据】返回连接、筛选与描述符数量，无参数，不读取外部服务"""
        return {"connections": len(self.parsers), "dropped": [event for parser in self.parsers for event in parser.dropped], "received_fds": self.received_fds, "sent_fds": self.sent_fds, "peer_resets": self.peer_resets, "errors": self.errors}


def serve(name, root):
    """【Wayland协议回归】【私有服务】在已验证会话绑定 name，root 保存启动与退出证据；返回前释放全部资源"""
    environment = session.environment()
    runtime = Path(environment["XDG_RUNTIME_DIR"]).resolve()
    if not name or Path(name).name != name or name in (".", ".."):
        raise ValueError("the private proxy requires a socket basename")
    path = runtime / name
    if path.exists() or path.is_symlink():
        raise ValueError("the private proxy socket already exists")
    upstream = runtime / environment["WAYLAND_DISPLAY"]
    root.mkdir(parents=True, exist_ok=False)
    listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    relay = Relay()
    previous = {}

    def accept():
        """【Wayland协议回归】【上游建立】接受本次客户端并连接已校验上游，无参数，失败仅关闭本次连接"""
        client, _address = listener.accept()
        server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        try:
            server.connect(str(upstream))
            relay.add(client, server)
        except BaseException:
            client.close()
            server.close()
            raise

    def terminate(_signum, _frame):
        """【Wayland协议回归】【退出信号】只通知本次代理停止，信号及栈帧不影响其他会话，无返回值"""
        relay.stop()

    try:
        listener.bind(str(path))
        os.chmod(path, 0o600)
        listener.listen()
        listener.setblocking(False)
        relay.selector.register(listener, selectors.EVENT_READ, "listen")
        for signum in (signal.SIGTERM, signal.SIGINT):
            previous[signum] = signal.signal(signum, terminate)
        (root / "ready.json").write_text(json.dumps({"pid": os.getpid(), "socket": str(path), "upstream": str(upstream), "hidden": ["zwlr_layer_shell_v1"]}, indent=2) + "\n")
        relay.run(accept)
    finally:
        relay.close()
        listener.close()
        if path.exists():
            path.unlink()
        for signum, handler in previous.items():
            signal.signal(signum, handler)
        (root / "result.json").write_text(json.dumps(relay.report(), indent=2) + "\n")


def main():
    """【Wayland协议回归】【命令入口】解析私有套接字名称与证据目录并启动本次服务，无参数、无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--name", required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    serve(options.name, options.root.resolve())


if __name__ == "__main__":
    main()
