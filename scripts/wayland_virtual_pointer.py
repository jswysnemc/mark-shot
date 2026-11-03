"""【Wayland 测试】【虚拟指针】自包含的 wlr 指针客户端和输出坐标转换"""

import struct
import time

from wayland_test_connection import TestConnection


BTN_LEFT, BTN_RIGHT, BTN_MIDDLE = 0x110, 0x111, 0x112


def transform_for_pointer(x, y, width, height, transform):
    """【Wayland 测试】【坐标转换】把输出逻辑 x、y 按 transform 转换，width、height 为逻辑尺寸，返回坐标及范围"""
    normalized = (transform or "normal").replace("-", "").replace("_", "").lower()
    transformations = {
        "normal": (x, y, width, height),
        "90": (y, width - x, height, width),
        "180": (width - x, height - y, width, height),
        "270": (height - y, x, height, width),
        "flipped": (width - x, y, width, height),
        "flipped90": (height - y, width - x, height, width),
        "flipped180": (x, height - y, width, height),
        "flipped270": (y, x, height, width),
    }
    if normalized not in transformations:
        raise ValueError(f"unsupported private output transform: {transform}")
    return transformations[normalized]


class VirtualPointer:
    """【Wayland 测试】【设备生命周期】持有单个虚拟指针，支持单输出和全局坐标模式"""

    def __init__(self, output_index=0, bind_output=True):
        """【Wayland 测试】【设备创建】output_index 指定输出，bind_output 决定是否绑定输出，无返回值，失败释放连接"""
        self.connection = TestConnection()
        self.pressed = set()
        self.pointer = None
        try:
            seat = self.connection.bind("wl_seat", 1)
            manager = self.connection.bind("zwlr_virtual_pointer_manager_v1", 2)
            if bind_output:
                if not 0 <= output_index < len(self.connection.outputs):
                    raise ValueError("private pointer output index is unavailable")
                name, _ = self.connection.outputs[output_index]
                output = self.connection.bind_name(name, "wl_output", 1)
                if self.connection.globals["zwlr_virtual_pointer_manager_v1"][1] < 2:
                    raise RuntimeError("output-bound virtual pointer requires protocol version 2")
                # 1. 【Wayland 测试】【对象顺序】先发送输出对象创建，后分配指针，避免客户端编号出现空缺
                self.pointer = self.connection.new_id()
                self.connection.send(manager, 2, struct.pack("=III", seat, output, self.pointer))
            else:
                self.pointer = self.connection.new_id()
                self.connection.send(manager, 0, struct.pack("=II", seat, self.pointer))
            self.connection.roundtrip()
        except BaseException:
            self.connection.close()
            raise

    def request(self, opcode, body=b""):
        """【Wayland 测试】【指针请求】发送 opcode 和 body，帧提交后严格校验往返，无返回值"""
        self.connection.send(self.pointer, opcode, body)
        self.connection.send(self.pointer, 4)
        self.connection.roundtrip()

    @staticmethod
    def timestamp():
        """【Wayland 测试】【事件时间】返回单调时钟的三十二位毫秒值，无参数"""
        return int(time.monotonic() * 1000) & 0xffffffff

    def move_abs(self, x, y, x_extent, y_extent):
        """【Wayland 测试】【绝对移动】发送 x、y 和 x_extent、y_extent 范围，无返回值"""
        if x_extent <= 0 or y_extent <= 0:
            raise ValueError("private pointer extents must be positive")
        values = [int(round(value)) for value in (x, y, x_extent, y_extent)]
        self.request(1, struct.pack("=IIIII", self.timestamp(), *values))

    def press(self, button=BTN_LEFT):
        """【Wayland 测试】【按钮按下】发送 button 的按下事件，记录状态以便退出时释放，无返回值"""
        self.pressed.add(button)
        self.request(2, struct.pack("=III", self.timestamp(), button, 1))

    def release(self, button=BTN_LEFT):
        """【Wayland 测试】【按钮释放】发送 button 的释放事件并清除本地记录，无返回值"""
        self.request(2, struct.pack("=III", self.timestamp(), button, 0))
        self.pressed.discard(button)

    def click(self, button=BTN_LEFT, hold=0.04):
        """【Wayland 测试】【完整点击】按下 button 并保持 hold 秒，始终释放按钮，无返回值"""
        if not 0 < hold <= 2:
            raise ValueError("private click hold must be between zero and two seconds")
        self.press(button)
        try:
            time.sleep(hold)
        finally:
            self.release(button)

    def scroll(self, dy=0.0, dx=0.0):
        """【Wayland 测试】【连续滚轮】dy、dx 为纵向和横向位移，正值向下或向右，无返回值"""
        self.connection.send(self.pointer, 5, struct.pack("=I", 0))
        for axis, value in enumerate((dy, dx)):
            if value:
                self.connection.send(self.pointer, 3, struct.pack("=IIi", self.timestamp(), axis, int(round(value * 256))))
        self.connection.send(self.pointer, 4)
        self.connection.roundtrip()

    def close(self):
        """【Wayland 测试】【设备释放】释放仍按下的按钮并销毁设备，始终关闭连接，无参数、无返回值"""
        try:
            for button in tuple(self.pressed):
                self.release(button)
            self.connection.send(self.pointer, 8)
            self.connection.roundtrip()
        except (OSError, RuntimeError):
            pass
        finally:
            self.connection.close()

    def __enter__(self):
        """【Wayland 测试】【范围进入】返回本次设备实例，无参数"""
        return self

    def __exit__(self, *_arguments):
        """【Wayland 测试】【范围退出】释放设备且不吞掉 _arguments 中的异常，无返回值"""
        self.close()
