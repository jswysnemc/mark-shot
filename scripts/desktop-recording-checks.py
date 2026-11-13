"""【桌面集成回归】【录制菜单】通过隔离命令端点验证生产查询、托盘刷新及控制交付"""

import json
from pathlib import Path
import socketserver
import threading


class RecordingServer:
    """【桌面集成回归】【录制端点】提供与生产协议一致的状态和命令响应，不启动采集设备"""

    def __init__(self, root):
        """【桌面集成回归】【端点建立】按 root 启动回环服务并发布端点，返回初始化后的测试服务"""
        self.path = Path(root) / "screen-recording.json"
        self.token = "private-desktop-recording-test"
        self.lock = threading.Lock()
        self.status = {"window_open": False, "phase": "idle", "elapsed_ms": 0,
                       "detail": "", "output": None, "format": "mp4"}
        self.commands = []
        owner = self

        class Handler(socketserver.BaseRequestHandler):
            """【桌面集成回归】【协议处理】为单次连接读取请求并返回当前状态"""

            def handle(self):
                """【桌面集成回归】【命令响应】无参数，校验令牌并处理暂停、继续、停止请求，无返回值"""
                self.request.settimeout(2)
                data = bytearray()
                while len(data) < 2048:
                    chunk = self.request.recv(2048 - len(data))
                    if not chunk:
                        break
                    data.extend(chunk)
                request = json.loads(data)
                assert request["token"] == owner.token
                action = request["action"]
                with owner.lock:
                    if action != "status":
                        owner.commands.append(action)
                        owner.status["phase"] = {"pause": "paused", "resume": "recording",
                                                 "stop": "saving"}[action]
                    response = json.dumps(owner.status).encode()
                self.request.sendall(response)

        self.server = socketserver.ThreadingTCPServer(("127.0.0.1", 0), Handler)
        self.server.daemon_threads = True
        self.thread = threading.Thread(target=self.server.serve_forever,
                                       kwargs={"poll_interval": 0.05}, daemon=True)
        self.thread.start()
        self.endpoint = {"address": "127.0.0.1:" + str(self.server.server_address[1]),
                         "token": self.token}
        self.restore_endpoint()

    def restore_endpoint(self):
        """【桌面集成回归】【端点恢复】无参数，原子写入有效端点，返回前完成文件替换"""
        pending = self.path.with_suffix(".new")
        pending.write_text(json.dumps(self.endpoint))
        pending.replace(self.path)

    def publish(self, phase, window_open=True):
        """【桌面集成回归】【状态发布】按 phase 与 window_open 更新会话快照，无返回值"""
        with self.lock:
            self.status.update(phase=phase, window_open=window_open)

    def close(self):
        """【桌面集成回归】【服务回收】无参数，关闭测试端口并保留失效端点用于异常退出检查"""
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=3)


def verify(runtime, watcher, menu, wait):
    """【桌面集成回归】【动态录制】按隔离 runtime、watcher、menu 与 wait 检查实际菜单，返回通过的检查名称"""
    import dbus

    interface = "com.canonical.dbusmenu"
    states = runtime.root / "cache/mark-shot/desktop"
    root = next(states.glob("*/status.json")).parent
    registrations = len(watcher.items)

    def recording_menu():
        """【桌面集成回归】【菜单快照】无参数，读取实际 D-Bus 菜单并返回录制标题及操作属性"""
        _, layout = menu.GetLayout(0, -1, [], dbus_interface=interface)
        recording = layout[2][4]
        children = [(int(child[0]), str(child[1]["label"]),
                     bool(child[1].get("enabled", True))) for child in recording[2]]
        return str(recording[1]["label"]), children

    def expect(title, actions):
        """【桌面集成回归】【状态等待】按 title 与 actions 等待生产菜单更新，返回实际操作编号"""
        def matches():
            """【桌面集成回归】【菜单匹配】无参数，返回匹配预期标题、文字与启用状态的实际菜单"""
            actual_title, children = recording_menu()
            return children if actual_title == title and [child[1:] for child in children] == actions else None

        return wait(matches, f"recording menu did not become {title}: {actions}")

    def click(action):
        """【桌面集成回归】【操作点击】按 action 的实际菜单编号触发生产回调，无返回值"""
        menu.Event(action[0], "clicked", dbus.Int32(0), dbus.UInt32(0), dbus_interface=interface)

    expect("屏幕录制", [("开始录制", True)])
    server = RecordingServer(root)
    try:
        # 1. 【桌面集成回归】【准备与倒计时】设置窗口占用时禁止重复启动，倒计时和设备启动阶段只提供取消
        server.publish("idle")
        expect("屏幕录制", [("录制窗口已打开", False)])
        server.publish("countdown")
        expect("录制倒计时", [("取消录制", True)])
        server.publish("starting")
        expect("录制准备中", [("取消录制", True)])
        # 2. 【桌面集成回归】【录制控制】真实托盘点击交付命令，暂停和继续相互替换，停止后禁止重复保存
        server.publish("recording")
        actions = expect("录制中", [("暂停录制", True), ("停止并保存", True)])
        click(actions[0])
        actions = expect("录制已暂停", [("继续录制", True), ("停止并保存", True)])
        click(actions[0])
        actions = expect("录制中", [("暂停录制", True), ("停止并保存", True)])
        click(actions[1])
        expect("录制保存中", [("正在保存", False)])
        with server.lock:
            assert server.commands == ["pause", "resume", "stop"], server.commands
        server.publish("finished", window_open=False)
        expect("屏幕录制", [("开始录制", True)])
        # 3. 【桌面集成回归】【状态恢复】损坏端点时禁用操作，端点修复后恢复录制菜单
        server.path.write_text("invalid endpoint")
        expect("屏幕录制", [("录制状态暂不可用", False)])
        server.restore_endpoint()
        server.publish("recording")
        expect("录制中", [("暂停录制", True), ("停止并保存", True)])
    finally:
        server.close()
    try:
        # 4. 【桌面集成回归】【异常退出】连接失效端点时恢复空闲，录制阶段变化不重新登记托盘
        expect("屏幕录制", [("开始录制", True)])
        assert len(watcher.items) == registrations, "recording changes unnecessarily replaced the tray"
    finally:
        server.path.unlink(missing_ok=True)
    return ["dynamic recording tray menu", "tray pause/resume/stop delivery",
            "recording state error and recovery", "recording process exit recovery"]
