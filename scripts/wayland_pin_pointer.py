"""【贴图回归】【全局指针】复用仓库协议客户端，虚拟指针不绑定单一输出"""

from contextlib import contextmanager
import os
from wayland_virtual_pointer import VirtualPointer


class GlobalPointer(VirtualPointer):
    """【贴图回归】【跨输出设备】使用私有合成器全局逻辑坐标，不做单输出变换"""

    def __init__(self):
        """【贴图回归】【设备绑定】创建不绑定单一输出的协议指针，无参数、无返回值"""
        super().__init__(bind_output=False)

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
