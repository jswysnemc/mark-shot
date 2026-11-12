"""【跨平台性能】【窗口像素】通过私有 X11 连接发送真实测试输入并观察服务器像素。"""

import time

from Xlib import X, display
from Xlib.ext import xtest  # noqa: F401


class Pixels:
    """【跨平台性能】【像素观察】只连接调用方提供的私有 display_name，不使用默认桌面。"""

    def __init__(self, display_name):
        """【跨平台性能】【连接创建】按 display_name 打开服务器连接，保存根窗口，无返回值。"""
        self.connection = display.Display(display_name)
        self.root = self.connection.screen().root

    def close(self):
        """【跨平台性能】【连接关闭】关闭本次观察连接，不退出其他客户端，无参数与返回值。"""
        self.connection.close()

    def background(self, size):
        """【跨平台性能】【对照背景】按 size 填充中灰桌面，使选区内外像素具有可验证差异，无返回值。"""
        color = self.connection.screen().default_colormap.alloc_color(32768, 32768, 32768).pixel
        # 1. 【跨平台性能】【背景可见】Wine 桌面窗口可能覆盖根窗口，独立背景窗口确保捕获像素具有统一颜色
        self.backdrop = self.root.create_window(0, 0, size[0], size[1], 0, self.connection.screen().root_depth,
                                               X.InputOutput, X.CopyFromParent,
                                               background_pixel=color, override_redirect=True)
        self.backdrop.map()
        self.connection.sync()

    def move(self, x, y):
        """【跨平台性能】【测试移动】按 x、y 发送绝对坐标 XTest 输入并交付服务器，无返回值。"""
        self.connection.xtest_fake_input(X.MotionNotify, root=self.root.id, x=round(x), y=round(y))
        self.connection.flush()

    def button(self, pressed):
        """【跨平台性能】【测试按键】按 pressed 发送鼠标左键按下或释放，无返回值。"""
        self.connection.xtest_fake_input(X.ButtonPress if pressed else X.ButtonRelease, 1)
        self.connection.flush()

    def read(self, region):
        """【跨平台性能】【局部读取】按 region 返回根窗口可见矩形的原生字节，不包含硬件鼠标图案。"""
        return self.root.get_image(*region, X.ZPixmap, 0xffffffff).data

    def changed(self, region, before, started, process, timeout=2):
        """【跨平台性能】【更新等待】按参数等待 region 像素改变，返回观察毫秒与新字节；程序退出或 timeout 内无更新直接失败。"""
        while time.monotonic() - started < timeout:
            pixels = self.read(region)
            if pixels != before:
                return (time.monotonic() - started) * 1000, pixels
            if process.poll() is not None:
                raise RuntimeError("editor exited before updating observed pixels")
            time.sleep(.001)
        raise RuntimeError(f"observed window pixels did not change within {timeout}s: {region}")
