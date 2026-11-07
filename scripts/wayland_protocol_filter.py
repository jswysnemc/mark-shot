"""【Wayland协议回归】【注册表筛选】保留对象编号与消息内容，只隐藏指定协议的注册表通知"""

import struct


class ProtocolFilter:
    """【Wayland协议回归】【消息解析】按连接保存两个方向的分片与注册表对象，不解释其他协议载荷"""

    def __init__(self, hidden):
        """【Wayland协议回归】【解析构造】hidden 为待隐藏接口名称集合；初始化缓冲与证据，无返回值"""
        self.hidden = frozenset(hidden)
        self.buffers = {"client": bytearray(), "server": bytearray()}
        self.registries = set()
        self.removed = set()
        self.dropped = []

    def feed(self, direction, data):
        """【Wayland协议回归】【分片解析】按 direction 接收 data，返回允许转发的完整消息，不改变消息编号"""
        if direction not in self.buffers:
            raise ValueError("unknown Wayland stream direction")
        buffer = self.buffers[direction]
        buffer.extend(data)
        result = bytearray()
        while len(buffer) >= 8:
            object_id, word = struct.unpack_from("=II", buffer)
            size, opcode = word >> 16, word & 0xFFFF
            if size < 8 or size % 4:
                raise ValueError("invalid Wayland message size")
            if len(buffer) < size:
                break
            message = bytes(buffer[:size])
            del buffer[:size]
            # 1. 【Wayland协议回归】【注册表身份】记录真正的 get_registry 请求，不能把其他对象事件误认为全局通知
            if direction == "client" and object_id == 1 and opcode == 1:
                if size != 12:
                    raise ValueError("invalid Wayland registry request")
                self.registries.add(struct.unpack_from("=I", message, 8)[0])
            if direction != "server" or object_id not in self.registries:
                result.extend(message)
                continue
            if opcode == 0:
                if size < 20:
                    raise ValueError("invalid Wayland global event")
                name, length = struct.unpack_from("=II", message, 8)
                padded = (length + 3) & ~3
                if length < 1 or 20 + padded != size or message[15 + length] != 0:
                    raise ValueError("invalid Wayland global interface string")
                interface = message[16:15 + length].decode("ascii")
                if interface in self.hidden:
                    self.removed.add(name)
                    self.dropped.append({"registry": object_id, "global": name, "interface": interface})
                    continue
            elif opcode == 1:
                if size != 12:
                    raise ValueError("invalid Wayland global removal")
                if struct.unpack_from("=I", message, 8)[0] in self.removed:
                    continue
            result.extend(message)
        return bytes(result)
