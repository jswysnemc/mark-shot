"""【桌面集成回归】【托盘宿主】仅在私有D-Bus上提供真实StatusNotifierWatcher协议端点"""

import dbus
import dbus.service
import threading
from dbus.mainloop.glib import DBusGMainLoop
from gi.repository import GLib


class Watcher(dbus.service.Object):
    """【桌面集成回归】【注册记录】持有测试服务名称、注册条目和私有事件循环"""

    def __init__(self):
        """【桌面集成回归】【宿主启动】无参数，在当前私有总线启动协议宿主，无返回值"""
        DBusGMainLoop(set_as_default=True)
        self.bus = dbus.SessionBus()
        self.owner = dbus.service.BusName("org.kde.StatusNotifierWatcher", bus=self.bus)
        super().__init__(self.owner, "/StatusNotifierWatcher")
        self.items = []
        self.loop = GLib.MainLoop()
        self.thread = threading.Thread(target=self.loop.run, daemon=True)
        self.thread.start()

    @dbus.service.method("org.kde.StatusNotifierWatcher", in_signature="s", out_signature="", sender_keyword="sender")
    def RegisterStatusNotifierItem(self, service, sender=None):
        """【桌面集成回归】【实际注册】按service与sender保存服务地址并广播注册，无返回值"""
        identity = (sender, service) if service.startswith("/") else (service, "/StatusNotifierItem")
        self.items.append(identity)
        self.StatusNotifierItemRegistered(identity[0] + identity[1])

    @dbus.service.signal("org.kde.StatusNotifierWatcher", signature="s")
    def StatusNotifierItemRegistered(self, service):
        """【桌面集成回归】【注册信号】按service广播系统托盘注册事件，无返回值"""

    @dbus.service.method("org.freedesktop.DBus.Properties", in_signature="ss", out_signature="v")
    def Get(self, interface, name):
        """【桌面集成回归】【属性读取】按interface和name返回宿主标准属性"""
        return self.GetAll(interface)[name]

    @dbus.service.method("org.freedesktop.DBus.Properties", in_signature="s", out_signature="a{sv}")
    def GetAll(self, interface):
        """【桌面集成回归】【宿主属性】按interface返回可用宿主和已注册服务，不读取真实桌面"""
        assert interface == "org.kde.StatusNotifierWatcher"
        return {"RegisteredStatusNotifierItems": dbus.Array([name + path for name, path in self.items], signature="s"), "IsStatusNotifierHostRegistered": dbus.Boolean(True), "ProtocolVersion": dbus.Int32(0)}

    def close(self):
        """【桌面集成回归】【宿主关闭】无参数，停止本测试事件线程并释放服务对象，无返回值"""
        self.loop.quit()
        self.thread.join(timeout=3)
        self.remove_from_connection()
        self.bus.release_name("org.kde.StatusNotifierWatcher")
