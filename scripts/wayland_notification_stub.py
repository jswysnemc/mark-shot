"""【通知测试】【私有服务】仅在独立 D-Bus 上接收预览，保存实际通知参数供回归检查"""

import asyncio
from contextlib import contextmanager
import json
from pathlib import Path
import subprocess
import sys
import time

DBUS_STRING = "s"
DBUS_UINT32 = "u"
DBUS_INT32 = "i"
DBUS_STRING_ARRAY = "as"
DBUS_HINTS = "a{sv}"
DBUS_SERVER_INFO = "ssss"


@contextmanager
def notification_service(root):
    """【通知测试】【服务生命周期】在 root 启动独立总线和通知服务，返回总线地址，退出只回收本次服务"""
    root.mkdir()
    bus = subprocess.Popen(["dbus-daemon", "--session", "--nofork", "--print-address=1"],
                           stdout=subprocess.PIPE, text=True)
    worker = None
    try:
        address = bus.stdout.readline().strip()
        worker = subprocess.Popen([sys.executable, __file__, address, str(root)])
        deadline = time.monotonic() + 10
        while not (root / "ready").exists():
            if worker.poll() is not None or time.monotonic() > deadline:
                raise RuntimeError("private notification service did not start")
            time.sleep(0.02)
        yield address
    finally:
        for process in (worker, bus):
            if process is not None and process.poll() is None:
                process.terminate()
                process.wait(timeout=5)
        bus.stdout.close()


async def serve(address, root):
    """【通知测试】【协议接收】按 address 导出通知接口，将接收到的预览尺寸写入 root，无返回值"""
    from dbus_next.aio import MessageBus
    from dbus_next.service import ServiceInterface, method

    class Notifications(ServiceInterface):
        """【通知测试】【接口实现】声明 mako 的图标预览能力，不创建任何桌面表面"""

        def __init__(self):
            """【通知测试】【接口初始化】注册标准接口名称，无参数、无返回值"""
            super().__init__("org.freedesktop.Notifications")

        @method()
        def GetCapabilities(self) -> DBUS_STRING_ARRAY:
            """【通知测试】【能力查询】无参数，返回正文标记能力"""
            return ["body", "body-markup"]

        @method()
        def GetServerInformation(self) -> DBUS_SERVER_INFO:
            """【通知测试】【服务查询】无参数，返回与 mako 一致的预览选择条件"""
            return ["mako", "test", "1", "1.2"]

        @method()
        def Notify(self, app_name: DBUS_STRING, replaces_id: DBUS_UINT32, app_icon: DBUS_STRING, summary: DBUS_STRING,
                   body: DBUS_STRING, actions: DBUS_STRING_ARRAY, hints: DBUS_HINTS, expire_timeout: DBUS_INT32) -> DBUS_UINT32:
            """【通知测试】【通知接收】接收标准通知参数及 hints，记录图像尺寸与字节数，返回通知编号"""
            image = hints.get("image-data")
            record = {"app": app_name, "summary": summary, "hints": list(hints)}
            if image:
                values = image.value
                record.update(width=values[0], height=values[1], bytes=len(values[6]))
            with (root / "notifications.jsonl").open("a") as stream:
                stream.write(json.dumps(record) + "\n")
            return 1

    bus = await MessageBus(bus_address=address).connect()
    bus.export("/org/freedesktop/Notifications", Notifications())
    await bus.request_name("org.freedesktop.Notifications")
    (root / "ready").touch()
    await asyncio.Event().wait()


if __name__ == "__main__":
    asyncio.run(serve(sys.argv[1], Path(sys.argv[2])))
