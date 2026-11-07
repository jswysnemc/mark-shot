"""【Wayland代理测试】【真实传输】验证分片、双向描述符与可写等待，不连接任何桌面服务"""

import array
import os
import socket
import struct
import threading
import unittest

from wayland_layer_shell_filter import Relay
from wayland_protocol_filter import ProtocolFilter


def message(object_id, opcode, body=b""):
    """【Wayland代理测试】【报文构造】按 object_id、opcode 和 body 返回原生字节序的完整协议消息"""
    return struct.pack("=II", object_id, ((len(body) + 8) << 16) | opcode) + body


def global_event(registry, name, interface):
    """【Wayland代理测试】【全局通知】按 registry、name 与 interface 返回带版本的合法注册表事件"""
    encoded = interface.encode() + b"\0"
    body = struct.pack("=II", name, len(encoded)) + encoded + b"\0" * (-len(encoded) % 4) + struct.pack("=I", 1)
    return message(registry, 0, body)


def receive(connection, length):
    """【Wayland代理测试】【完整接收】从 connection 接收 length 字节及实际描述符，返回字节与描述符列表"""
    result, descriptors = bytearray(), []
    connection.settimeout(3)
    while len(result) < length:
        data, ancillary, flags, _address = connection.recvmsg(length - len(result), socket.CMSG_SPACE(256 * 4))
        if not data or flags & (socket.MSG_TRUNC | socket.MSG_CTRUNC):
            raise RuntimeError("test message or descriptor was truncated")
        result.extend(data)
        for level, kind, value in ancillary:
            if level == socket.SOL_SOCKET and kind == socket.SCM_RIGHTS:
                values = array.array("i")
                values.frombytes(value[:len(value) - len(value) % values.itemsize])
                descriptors.extend(values)
    return bytes(result), descriptors


class ProxyChecks(unittest.TestCase):
    """【Wayland代理测试】【回归范围】使用自有Unix套接字验证代理和解析器的实际行为"""

    def test_fragmented_globals_only_remove_hidden_interfaces_and_their_removal(self):
        """【Wayland代理测试】【分片筛选】逐字节解析真实注册表结构，非注册表消息保持原样，无参数、无返回值"""
        parser = ProtocolFilter(["zwlr_layer_shell_v1"])
        request = message(1, 1, struct.pack("=I", 7))
        self.assertEqual(b"".join(parser.feed("client", bytes([value])) for value in request), request)
        keep = global_event(7, 18, "wl_output")
        hidden = global_event(7, 19, "zwlr_layer_shell_v1")
        unrelated = global_event(8, 19, "zwlr_layer_shell_v1")
        removal = message(7, 1, struct.pack("=I", 19))
        data = keep + hidden + unrelated + removal
        self.assertEqual(b"".join(parser.feed("server", bytes([value])) for value in data), keep + unrelated)
        self.assertEqual(parser.dropped, [{"registry": 7, "global": 19, "interface": "zwlr_layer_shell_v1"}])

    def test_invalid_message_sizes_and_strings_are_rejected(self):
        """【Wayland代理测试】【边界拒绝】错误长度和字符串不能跨消息读取或错误隐藏协议，无参数、无返回值"""
        parser = ProtocolFilter(["zwlr_layer_shell_v1"])
        with self.assertRaisesRegex(ValueError, "message size"):
            parser.feed("server", struct.pack("=II", 1, 7 << 16))
        parser = ProtocolFilter(["zwlr_layer_shell_v1"])
        parser.feed("client", message(1, 1, struct.pack("=I", 7)))
        with self.assertRaisesRegex(ValueError, "interface string"):
            parser.feed("server", message(7, 0, struct.pack("=IIII", 18, 100, 0, 1)))

    def test_real_descriptors_survive_filtering_and_bidirectional_forwarding(self):
        """【Wayland代理测试】【共享内存转发】两方向交付真实memfd，筛选注册表不丢失相邻载荷和描述符，无返回值"""
        client, proxy_client = socket.socketpair()
        server, proxy_server = socket.socketpair()
        relay = Relay()
        relay.add(proxy_client, proxy_server)
        worker = threading.Thread(target=relay.run)
        worker.start()
        descriptors = []
        try:
            request = message(1, 1, struct.pack("=I", 7))
            client.sendall(request)
            data, attached = receive(server, len(request))
            self.assertEqual((data, attached), (request, []))
            for source, target, body in [(server, client, b"server-keymap"), (client, server, b"client-shared-memory")]:
                descriptor = os.memfd_create("private-proxy-test", os.MFD_CLOEXEC)
                os.write(descriptor, body)
                event = message(20, 0, struct.pack("=II", 1, len(body)))
                prefix = global_event(7, 19, "zwlr_layer_shell_v1") if source is server else b""
                try:
                    source.sendmsg([prefix + event], [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array("i", [descriptor]))])
                finally:
                    os.close(descriptor)
                data, attached = receive(target, len(event))
                descriptors.extend(attached)
                self.assertEqual(data, event)
                self.assertEqual(len(attached), 1)
                self.assertEqual(os.pread(attached[0], len(body), 0), body)
        finally:
            relay.stop()
            worker.join(timeout=3)
            relay.close()
            client.close()
            server.close()
            for descriptor in descriptors:
                os.close(descriptor)
        self.assertFalse(worker.is_alive())
        self.assertEqual((relay.report()["received_fds"], relay.report()["sent_fds"]), (2, 2))
        self.assertEqual(relay.errors, [])

    def test_small_socket_buffers_preserve_large_fragmented_transfers(self):
        """【Wayland代理测试】【部分写入】受限套接字通过多次可写事件发送完整协议流，不阻塞反向消息，无返回值"""
        client, proxy_client = socket.socketpair()
        server, proxy_server = socket.socketpair()
        proxy_server.setsockopt(socket.SOL_SOCKET, socket.SO_SNDBUF, 2048)
        relay = Relay()
        relay.add(proxy_client, proxy_server)
        worker = threading.Thread(target=relay.run)
        worker.start()
        data = message(40, 3, bytes(range(256)) * 32) * 40
        sender = threading.Thread(target=client.sendall, args=(data,))
        sender.start()
        try:
            actual, attached = receive(server, len(data))
            self.assertEqual((actual, attached), (data, []))
        finally:
            sender.join(timeout=3)
            relay.stop()
            worker.join(timeout=3)
            relay.close()
            client.close()
            server.close()
        self.assertFalse(sender.is_alive())
        self.assertFalse(worker.is_alive())
        self.assertEqual(relay.errors, [])


if __name__ == "__main__":
    unittest.main()
