"""管理贴图回归测试使用的独立配置、D-Bus、显示服务和输入进程。"""

from contextlib import ExitStack
import json
import os
from pathlib import Path
import resource
import select
import signal
import subprocess
import time


def isolated_environment(root):
    """在 root 创建隔离目录，返回供 D-Bus 及其子进程共同使用的环境变量。"""
    env = os.environ.copy()
    for key in ("DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET", "NIRI_SOCKET", "SWAYSOCK",
                "HYPRLAND_INSTANCE_SIGNATURE", "DBUS_SESSION_BUS_ADDRESS", "DBUS_STARTER_ADDRESS",
                "DBUS_STARTER_BUS_TYPE", "XDG_ACTIVATION_TOKEN", "WAYLAND_DEBUG",
                "QT_PLUGIN_PATH", "QT_QPA_PLATFORM", "QT_WAYLAND_SHELL_INTEGRATION",
                "QT_SCALE_FACTOR", "QT_SCREEN_SCALE_FACTORS", "QT_AUTO_SCREEN_SCALE_FACTOR",
                "QT_FONT_DPI", "AQ_BACKENDS", "WLR_BACKENDS"):
        env.pop(key, None)
    for key, folder in (("XDG_RUNTIME_DIR", "r"), ("XDG_CONFIG_HOME", "config"),
                        ("XDG_CACHE_HOME", "cache"), ("XDG_DATA_HOME", "data"), ("TMPDIR", "tmp")):
        path = root / folder
        path.mkdir(mode=0o700)
        env[key] = str(path)
    # gvfs 会在运行目录挂载 FUSE，会话结束后残留的挂载点会让临时目录无法删除
    env.update(GIO_USE_VFS="local", GVFS_DISABLE_FUSE="1")
    env.update(WLR_RENDERER_ALLOW_SOFTWARE="1", HYPRLAND_NO_SD_NOTIFY="1", HYPRLAND_NO_SD_VARS="1",
               XDG_SESSION_TYPE="wayland", XDG_CURRENT_DESKTOP="Hyprland",
               XDG_SESSION_DESKTOP="Hyprland", DESKTOP_SESSION="hyprland",
               QT_QPA_PLATFORMTHEME="generic", QT_STYLE_OVERRIDE="Fusion",
               QT_NO_XDG_DESKTOP_PORTAL="1", MARK_SHOT_HYPRLAND_TEST_RUNTIME=str(root / "r"))
    return env


def parent_render_node():
    """选择虚拟父显示使用的渲染节点，返回节点路径；没有可选节点时返回 None，交给 KWin 默认选择。"""
    override = os.environ.get("MARK_SHOT_HYPRLAND_TEST_RENDER_NODE")
    if override:
        return override
    # NVIDIA 专有驱动无法为 Hyprland 的 HEADLESS 输出分配 GBM 缓冲，多显卡时改用其他显卡
    for node in sorted(Path("/dev/dri").glob("renderD*")):
        driver = Path("/sys/class/drm") / node.name / "device/driver"
        if driver.exists() and driver.resolve().name != "nvidia":
            return str(node)
    return None


def stop_process_group(process):
    """终止 process 所属的本次测试进程组并等待回收，无返回值。"""
    try:
        os.killpg(process.pid, signal.SIGTERM)
        process.wait(timeout=3)
    except (ProcessLookupError, subprocess.TimeoutExpired):
        pass
    finally:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.wait(timeout=3)


def run_isolated(args, script, root, arguments):
    """使用 args 配置在 root 隔离运行 script 和 arguments，返回测试退出码。"""
    import sys

    # 1. 【Hyprland测试】【会话隔离】在启动 D-Bus 前设置环境，覆盖其激活的服务
    env = isolated_environment(root)
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    command = ["dbus-run-session", "--", sys.executable, str(script), *arguments, "--inside-session"]
    log_path = args.artifacts / "session.log"
    with log_path.open("w") as log:
        process = subprocess.Popen(command, env=env, stderr=log, start_new_session=True)
        try:
            result = process.wait(timeout=55)
        except subprocess.TimeoutExpired:
            print("FAIL isolated Hyprland check timed out", flush=True)
            result = 1
        finally:
            stop_process_group(process)
    if result:
        print("\n".join(log_path.read_text().splitlines()[-15:]), file=sys.stderr)
    return result


def wait_until(callback, description, timeout=10):
    """等待 callback 返回非空值；超出 timeout 秒时以 description 报错，否则返回该值。"""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        value = callback()
        if value:
            return value
        time.sleep(0.025)
    raise RuntimeError(f"Timed out waiting for {description}")


def pinned_layers(value, pid):
    """递归收集 value 中属于 pid 的贴图 surface，返回几何记录列表。"""
    if isinstance(value, dict):
        if value.get("namespace") in ("dock", "mark-shot-pinned-drag-preview") and value.get("pid") == pid:
            return [value]
        return [item for child in value.values() for item in pinned_layers(child, pid)]
    if isinstance(value, list):
        return [item for child in value for item in pinned_layers(child, pid)]
    return []


class HyprlandTestSession:
    """持有本次隔离测试的显示服务、应用和虚拟指针。"""

    def __init__(self, args):
        """根据 args 保存测试路径并核实隔离运行目录，无返回值。"""
        runtime = os.environ.get("MARK_SHOT_HYPRLAND_TEST_RUNTIME")
        if not runtime or runtime != os.environ.get("XDG_RUNTIME_DIR"):
            raise RuntimeError("Use the test launcher to create an isolated session")
        self.root = Path(runtime).parent
        if not self.root.name.startswith("mshypr-"):
            raise RuntimeError("Refusing to connect outside the isolated test directory")
        self.args = args
        self.env = os.environ.copy()
        self.processes = []
        self.files = ExitStack()
        self.instance = None
        self.monitor = None
        self.app = None
        self.pointer = None

    def __enter__(self):
        """启动独立显示服务，失败时清理进程，成功时返回当前会话。"""
        try:
            self.start_display()
            return self
        except BaseException:
            self.__exit__(None, None, None)
            raise

    def __exit__(self, exception_type, exception, traceback):
        """按逆序释放会话资源；异常参数由上下文管理器提供，不吞掉异常。"""
        for process in reversed(self.processes):
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=2)
        self.files.close()

    def spawn(self, name, command, env=None, interactive=False):
        """以 name 记录 command 日志，使用 env 启动进程；interactive 开启管道，返回进程。"""
        log = self.files.enter_context((self.args.artifacts / f"{name}.log").open("w"))
        process = subprocess.Popen(command, env=env or self.env,
                                   stdin=subprocess.PIPE if interactive else subprocess.DEVNULL,
                                   stdout=subprocess.PIPE if interactive else log,
                                   stderr=log, text=True, bufsize=1)
        self.processes.append(process)
        return process

    def control(self, *arguments):
        """向当前隔离 Hyprland 发送 arguments，返回标准输出；失败时抛出异常。"""
        return subprocess.check_output([self.args.hyprctl, "-i", self.instance, *arguments],
                                       env=self.env, text=True, stderr=subprocess.PIPE, timeout=5)

    def query(self, *arguments):
        """执行 arguments 指定的只读查询，返回 Hyprland 的 JSON 数据。"""
        return json.loads(self.control("-j", *arguments))

    def start_display(self):
        """创建虚拟父显示和 Hyprland 测试输出，保存显示参数，无返回值。"""
        config = self.root / "hyprland.conf"
        config.write_text(f"monitor = ,2880x1800@120,0x0,{self.args.scale}\n"
                          "debug {\n  disable_logs = false\n  enable_stdout_logs = true\n}\n"
                          "animations {\n  enabled = false\n}\n"
                          "misc {\n  disable_hyprland_logo = true\n  disable_splash_rendering = true\n}\n"
                          "general {\n  border_size = 0\n  gaps_in = 0\n  gaps_out = 0\n}\n")

        # 1. 【Hyprland测试】【显示隔离】虚拟 KWin 提供 allocator，不连接用户的桌面或输入设备
        parent_env = dict(self.env, KWIN_COMPOSE="O2")
        render_node = parent_render_node()
        if render_node:
            parent_env["KWIN_RENDER_NODES"] = render_node
        self.spawn("parent", [self.args.kwin, "--virtual", "--socket", "kwin-test", "--width", "1280",
                              "--height", "900", "--no-lockscreen", "--no-global-shortcuts",
                              "--no-kactivities"], parent_env)
        wait_until(lambda: (self.root / "r/kwin-test").is_socket(), "virtual parent display")
        self.env.update(WAYLAND_DISPLAY="kwin-test", LIBSEAT_BACKEND="seatd",
                        SEATD_SOCK=str(self.root / "r/no-seat.socket"))
        compositor = self.spawn("hyprland", [self.args.hyprland, "--config", str(config)])

        def signature():
            """返回本次 Hyprland 的实例编号；启动失败时抛出异常。"""
            if compositor.poll() is not None:
                raise RuntimeError("Hyprland exited; inspect hyprland.log")
            directory = self.root / "r/hypr"
            return next((p.name for p in directory.iterdir() if (p / ".socket.sock").is_socket()), None) \
                if directory.is_dir() else None

        self.instance = wait_until(signature, "isolated Hyprland instance")
        socket = wait_until(lambda: next((p.name for p in (self.root / "r").iterdir()
                                          if p.name.startswith("wayland-") and p.is_socket()), None),
                            "Hyprland Wayland socket")
        self.env.update(WAYLAND_DISPLAY=socket, HYPRLAND_INSTANCE_SIGNATURE=self.instance,
                        QT_QPA_PLATFORM="wayland")

        # 2. 【Hyprland测试】【输出配置】输入只绑定 HEADLESS 输出，父窗口输出移至测试范围外
        self.control("output", "create", "headless")
        monitor = wait_until(lambda: next((m for m in self.query("monitors")
                                            if m["name"].startswith("HEADLESS")), None), "headless output")
        for other in self.query("monitors"):
            if other["name"] != monitor["name"]:
                self.control("keyword", "monitor", f"{other['name']},preferred,3000x0,1")
        self.control("keyword", "monitor", f"{monitor['name']},2880x1800@120,0x0,{self.args.scale}")
        time.sleep(0.3)
        self.monitor = next(m for m in self.query("monitors") if m["name"] == monitor["name"])
        if not self.monitor["width"] or not self.monitor["height"]:
            raise RuntimeError("Hyprland could not allocate the headless output; set "
                               "MARK_SHOT_HYPRLAND_TEST_RENDER_NODE to a render node with GBM support")
        if abs(self.monitor["scale"] - self.args.scale) > 0.01:
            raise RuntimeError("The compositor did not apply the requested output scale")
        self.control("dispatch", "movecursor", "700 450")
        (self.args.artifacts / "monitor.json").write_text(json.dumps(self.monitor, indent=2) + "\n")
        (self.args.artifacts / "hyprland-version.json").write_text(json.dumps(self.query("version"), indent=2) + "\n")

    def start_app(self, fixture):
        """启动正式应用并贴上 fixture 图片，等待 layer surface 就绪，无返回值。"""
        config = self.root / "config/mark-shot"
        config.mkdir(exist_ok=True)
        (config / "config.json").write_text(json.dumps({
            "pinnedWindow": {"alwaysOnTop": True, "autoOcr": False},
        }))
        app_env = dict(self.env, QT_PLUGIN_PATH=str(self.args.binary.parent))
        if self.args.protocol_log:
            app_env["WAYLAND_DEBUG"] = "client"
        self.app = self.spawn("app", [str(self.args.binary), "--pin-image", str(fixture), "--debug",
                                      "--debug-log", str(self.args.artifacts / "app-debug.log")], app_env)
        wait_until(self.layer, "pinned image layer surface")

    def layers(self):
        """返回本次应用的原始贴图和拖动预览列表；应用退出时抛出异常。"""
        if self.app.poll() is not None:
            raise RuntimeError("Mark Shot exited; inspect app.log")
        return pinned_layers(self.query("layers"), self.app.pid)

    def layer(self):
        """返回正在显示的贴图几何，拖动时优先选择预览；尚未映射时返回 None。"""
        layers = self.layers()
        previews = [layer for layer in layers if layer["namespace"] == "mark-shot-pinned-drag-preview"]
        if len(previews) == 1:
            return previews[0]
        return layers[0] if len(layers) == 1 else None

    def settled_layer(self):
        """等待原窗口接管显示且预览全部回收，返回原窗口几何；超时则报错。"""
        def original_only():
            """仅存在一个原始贴图时返回该记录，否则返回 None。"""
            layers = self.layers()
            return layers[0] if len(layers) == 1 and layers[0]["namespace"] == "dock" else None

        return wait_until(original_only, "drag preview cleanup and original image handoff", timeout=3)

    def start_pointer(self):
        """创建绑定测试输出的虚拟指针，等待协议初始化完成，无返回值。"""
        width = self.monitor["width"] / self.monitor["scale"]
        height = self.monitor["height"] / self.monitor["scale"]
        self.pointer = self.spawn("pointer", [str(self.args.pointer), str(width), str(height),
                                              self.monitor["name"]], interactive=True)
        self.expect_pointer_reply("ready")

    def expect_pointer_reply(self, expected):
        """等待指针返回 expected 文本；超时或协议错误时抛出异常，无返回值。"""
        if not select.select([self.pointer.stdout], [], [], 3)[0]:
            raise RuntimeError("Virtual pointer reply timed out")
        if self.pointer.stdout.readline().strip() != expected:
            raise RuntimeError("Virtual pointer failed; inspect pointer.log")

    def send(self, command):
        """发送 command 输入并等待合成器确认；只向本次虚拟指针写入，无返回值。"""
        self.pointer.stdin.write(command + "\n")
        self.pointer.stdin.flush()
        self.expect_pointer_reply("ok")
