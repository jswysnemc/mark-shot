"""【贴图回归】【全局指针】复用 niri-use 协议客户端，虚拟指针不绑定单一输出"""

from contextlib import contextmanager
import os
from pathlib import Path
import struct
import sys

sys.path.insert(0, str(Path.home() / ".claude/skills/niri-use/src"))
from niri_pointer import VirtualPointer, VP_DESTROY  # noqa: E402


class GlobalPointer(VirtualPointer):
    """【贴图回归】【跨输出设备】使用私有合成器全局逻辑坐标，不做单输出变换"""

    def _setup(self):
        """【贴图回归】【设备绑定】先发现座席，再创建无输出绑定的指针；继承协议客户端的往返校验"""
        super()._setup()
        self._send(self.vp, VP_DESTROY)
        self.vp = self._nid()
        self._send(self.mgr, 0, struct.pack("=II", self.seat, self.vp))
        self._roundtrip()

    def move(self, point):
        """【贴图回归】【全局移动】将 point 发往已校验的 1760×720 私有输出布局，无返回值"""
        self.move_abs(point[0], point[1], 1760, 720)


@contextmanager
def private_environment(environment):
    """【贴图回归】【环境隔离】暂时设置 environment 供协议客户端连接，结束后恢复原环境"""
    previous = dict(os.environ)
    os.environ.clear()
    os.environ.update(environment)
    try:
        yield
    finally:
        os.environ.clear()
        os.environ.update(previous)
