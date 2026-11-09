"""【全局快捷键回归】【系统占用】在私有X11显示抓取一个真实按键以验证冲突报告"""

import ctypes


class OccupiedKey:
    """【全局快捷键回归】【资源持有】持有独立X11连接，关闭时自动释放抓取"""

    def __init__(self, display, symbol):
        """【全局快捷键回归】【按键占用】按display和symbol注册不带修饰键的抓取，无返回值"""
        self.library = ctypes.CDLL("libX11.so.6")
        self.library.XOpenDisplay.argtypes = [ctypes.c_char_p]
        self.library.XOpenDisplay.restype = ctypes.c_void_p
        self.library.XDefaultRootWindow.argtypes = [ctypes.c_void_p]
        self.library.XDefaultRootWindow.restype = ctypes.c_ulong
        self.library.XKeysymToKeycode.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
        self.library.XKeysymToKeycode.restype = ctypes.c_uint
        self.library.XGrabKey.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_uint, ctypes.c_ulong, ctypes.c_int, ctypes.c_int, ctypes.c_int]
        self.library.XSync.argtypes = [ctypes.c_void_p, ctypes.c_int]
        self.library.XCloseDisplay.argtypes = [ctypes.c_void_p]
        self.connection = self.library.XOpenDisplay(display.encode())
        assert self.connection, "private X11 display unavailable"
        code = self.library.XKeysymToKeycode(self.connection, symbol)
        root = self.library.XDefaultRootWindow(self.connection)
        self.library.XGrabKey(self.connection, code, 0, root, 0, 1, 1)
        self.library.XSync(self.connection, 0)

    def close(self):
        """【全局快捷键回归】【释放占用】无参数，关闭本夹具拥有的连接，无返回值"""
        self.library.XCloseDisplay(self.connection)
