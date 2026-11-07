"""【长截图回归】【原始缓冲】独立wlr-screencopy客户端不重采样分数比例输出"""

import array
import json
import mmap
import os
import socket
import struct
import time
from pathlib import Path

import wayland_text_format_session as session
from PIL import Image
from wayland_test_connection import TestConnection


class RawCapture(TestConnection):
    """【长截图回归】【独立协议】复用测试传输读取合成器原始缓冲，生产捕获实现不参与期望图"""

    def __init__(self):
        """【长截图回归】【连接构造】初始化单次帧状态并校验私有合成器环境，无参数、无返回值"""
        session.environment()
        self.frame = None
        self.description = None
        self.flags = 0
        self.ready = False
        super().__init__()

    def event(self, object_id, opcode, body):
        """【长截图回归】【帧事件】接收object_id、opcode和body中的格式、方向及就绪状态，无返回值"""
        super().event(object_id, opcode, body)
        if object_id != self.frame:
            return
        if opcode == 0:
            self.description = struct.unpack("=IIII", body)
        elif opcode == 1:
            self.flags = struct.unpack("=I", body)[0]
        elif opcode == 2:
            self.ready = True
        elif opcode == 3:
            raise RuntimeError("the independent raw capture failed")

    def capture(self):
        """【长截图回归】【原始采集】返回唯一输出的RGB原始像素，包含真实物理边界，不按逻辑尺寸重新缩放"""
        if len(self.outputs) != 1:
            raise RuntimeError("raw scroll capture requires one private output")
        manager = self.bind("zwlr_screencopy_manager_v1", 1)
        output = self.bind_name(self.outputs[0][0], "wl_output", 1)
        self.frame = self.new_id()
        self.send(manager, 0, struct.pack("=III", self.frame, 0, output))
        self.roundtrip()
        if self.description is None:
            raise RuntimeError("raw capture did not receive a buffer description")
        format_id, width, height, stride = self.description
        if format_id not in (0, 1):
            raise RuntimeError(f"unsupported raw capture format {format_id}")
        shm = self.bind("wl_shm", 1)
        pool = self.new_id()
        fd = os.memfd_create("private-scroll-reference", os.MFD_CLOEXEC)
        try:
            os.ftruncate(fd, stride * height)
            with mmap.mmap(fd, stride * height) as data:
                # 1. 【长截图回归】【共享缓冲】文件描述符只交付当前私有合成器，尺寸严格来自协议事件
                body = struct.pack("=Ii", pool, stride * height)
                message = struct.pack("=II", shm, (16 << 16)) + body
                if self.socket.sendmsg([message], [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array("i", [fd]))]) != len(message):
                    raise RuntimeError("raw capture could not send its shared memory pool")
                buffer = self.new_id()
                self.send(pool, 0, struct.pack("=IiiiiI", buffer, 0, width, height, stride, format_id))
                self.send(self.frame, 0, struct.pack("=I", buffer))
                deadline = time.monotonic() + 3
                while not self.ready:
                    self.roundtrip()
                    if time.monotonic() > deadline:
                        raise TimeoutError("raw capture frame did not arrive")
                image = Image.frombytes("RGB", (width, height), bytes(data), "raw", "BGRX", stride, -1 if self.flags & 1 else 1)
                self.send(buffer, 0)
                self.send(pool, 1)
                self.send(self.frame, 1)
                self.send(manager, 2)
                self.roundtrip()
        finally:
            os.close(fd)
        metadata = json.loads((Path(os.environ["XDG_RUNTIME_DIR"]) / "runtime.json").read_text())
        transform = next(iter(metadata["outputs"].values()))["logical"]["transform"].lower().replace("-", "")
        # 2. 【长截图回归】【输出方向】Pillow只重新排列原始像素，不改变采样或颜色
        transforms = {"normal": None, "90": Image.Transpose.ROTATE_270, "180": Image.Transpose.ROTATE_180,
                      "270": Image.Transpose.ROTATE_90, "flipped": Image.Transpose.FLIP_LEFT_RIGHT,
                      "flipped180": Image.Transpose.FLIP_TOP_BOTTOM, "flipped90": Image.Transpose.TRANSPOSE,
                      "flipped270": Image.Transpose.TRANSVERSE}
        if transform not in transforms:
            raise RuntimeError(f"unsupported private raw capture transform {transform}")
        return image if transforms[transform] is None else image.transpose(transforms[transform])


def capture():
    """【长截图回归】【缓冲入口】建立独立私有连接并返回原始输出图像，结束后释放连接，无参数"""
    connection = RawCapture()
    try:
        return connection.capture()
    finally:
        connection.close()
