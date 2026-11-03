"""【Wayland 测试】【客户端回归】校验自包含协议编码、输出变换和连接错误"""

import struct
import unittest
from unittest.mock import Mock, patch

from wayland_test_connection import TestConnection, wire_string
from wayland_virtual_pointer import BTN_RIGHT, VirtualPointer, transform_for_pointer


class PointerTests(unittest.TestCase):
    """【Wayland 测试】【指针场景】使用协议替身覆盖设备创建和按键生命周期"""

    def connection(self):
        """【Wayland 测试】【连接替身】返回带两个输出和版本二指针协议的模拟连接，无参数"""
        connection = Mock()
        connection.outputs = [(7, 4), (8, 4)]
        connection.globals = {"zwlr_virtual_pointer_manager_v1": (9, 2)}
        connection.bind.side_effect = [10, 11]
        connection.bind_name.return_value = 12
        connection.new_id.return_value = 13
        return connection

    def test_output_and_global_binding(self):
        """【Wayland 测试】【设备绑定】单输出绑定指定输出，全局设备不绑定输出，无参数、无返回值"""
        for bound in (True, False):
            with self.subTest(bound=bound):
                connection = self.connection()
                with patch("wayland_virtual_pointer.TestConnection", return_value=connection):
                    pointer = VirtualPointer(output_index=1, bind_output=bound)
                if bound:
                    connection.bind_name.assert_called_once_with(8, "wl_output", 1)
                    connection.send.assert_called_once_with(11, 2, struct.pack("=III", 10, 12, 13))
                else:
                    connection.bind_name.assert_not_called()
                    connection.send.assert_called_once_with(11, 0, struct.pack("=II", 10, 13))
                pointer.close()

    def test_creation_error_closes_connection(self):
        """【Wayland 测试】【创建失败】缺少输出或往返错误时关闭本次连接，无参数、无返回值"""
        connection = self.connection()
        with patch("wayland_virtual_pointer.TestConnection", return_value=connection), self.assertRaises(ValueError):
            VirtualPointer(output_index=5)
        connection.close.assert_called_once()

    def test_button_state_and_exit_release(self):
        """【Wayland 测试】【按钮清理】退出时释放仍按下的真实右键，不转换为左键，无参数、无返回值"""
        connection = self.connection()
        with patch("wayland_virtual_pointer.TestConnection", return_value=connection):
            pointer = VirtualPointer()
        connection.send.reset_mock()
        with patch.object(pointer, "timestamp", return_value=42):
            pointer.press(BTN_RIGHT)
            pointer.close()
        requests = connection.send.call_args_list
        self.assertEqual(requests[0].args, (13, 2, struct.pack("=III", 42, BTN_RIGHT, 1)))
        self.assertEqual(requests[2].args, (13, 2, struct.pack("=III", 42, BTN_RIGHT, 0)))
        self.assertEqual(requests[4].args, (13, 8))
        connection.close.assert_called_once()

    def test_motion_and_signed_scroll(self):
        """【Wayland 测试】【动作编码】坐标保留输出范围，负滚轮按有符号定点编码，无参数、无返回值"""
        connection = self.connection()
        with patch("wayland_virtual_pointer.TestConnection", return_value=connection):
            pointer = VirtualPointer()
        connection.send.reset_mock()
        with patch.object(pointer, "timestamp", return_value=42):
            pointer.move_abs(10, 20, 800, 600)
            pointer.scroll(dy=-30)
        self.assertEqual(connection.send.call_args_list[0].args, (13, 1, struct.pack("=IIIII", 42, 10, 20, 800, 600)))
        self.assertEqual(connection.send.call_args_list[3].args, (13, 3, struct.pack("=IIi", 42, 0, -7680)))
        pointer.close()

    def test_all_output_transforms(self):
        """【Wayland 测试】【八种变换】覆盖所有输出变换和非法值，无参数、无返回值"""
        expected = {"normal": (10, 20, 800, 600), "90": (20, 790, 600, 800), "180": (790, 580, 800, 600), "270": (580, 10, 600, 800), "flipped": (790, 20, 800, 600), "flipped90": (580, 790, 600, 800), "Flipped-180": (10, 580, 800, 600), "flipped270": (20, 10, 600, 800)}
        for transform, coordinates in expected.items():
            with self.subTest(transform=transform):
                self.assertEqual(transform_for_pointer(10, 20, 800, 600, transform), coordinates)
        with self.assertRaises(ValueError):
            transform_for_pointer(10, 20, 800, 600, "unknown")


class ConnectionTests(unittest.TestCase):
    """【Wayland 测试】【传输场景】不连接真实桌面，校验消息分片和失败信号"""

    def connection(self, chunks):
        """【Wayland 测试】【传输替身】返回按 chunks 接收消息的未连接对象，无参数"""
        connection = TestConnection.__new__(TestConnection)
        connection.socket = Mock()
        connection.socket.recv.side_effect = chunks
        connection.buffer = bytearray()
        connection.next_id = 3
        connection.registry = 2
        connection.globals, connection.outputs = {}, []
        return connection

    def test_fragmented_registry_and_sync(self):
        """【Wayland 测试】【消息分片】注册表与回调在任意网络分片下仍完整读取，无参数、无返回值"""
        body = struct.pack("=I", 7) + wire_string("wl_output") + struct.pack("=I", 4)
        message = struct.pack("=II", 2, (8 + len(body)) << 16) + body
        message += struct.pack("=III", 3, 12 << 16, 99)
        connection = self.connection([message[:5], message[5:17], message[17:]])
        connection.roundtrip()
        self.assertEqual(connection.outputs, [(7, 4)])
        self.assertEqual(connection.globals["wl_output"], (7, 4))

    def test_disconnect_and_protocol_error(self):
        """【Wayland 测试】【明确失败】断连、非法消息尺寸和服务端错误必须抛出异常，无参数、无返回值"""
        for data in (b"", struct.pack("=II", 3, 7 << 16), struct.pack("=II", 1, 8 << 16)):
            with self.subTest(data=data), self.assertRaises((ConnectionError, RuntimeError)):
                self.connection([data]).roundtrip()


if __name__ == "__main__":
    unittest.main()
