"""【截图历史测试】【应用会话】配置隔离、辅助窗口启动和完整截图等待"""

import json
import subprocess
import time

import wayland_history_checks as checks
import wayland_test_session as session


def launch(binary, environment, config, arguments, log):
    """【截图历史测试】【进程启动】以 binary、environment、config 执行 arguments；日志写入 log，返回进程"""
    with log.open("w") as output:
        return subprocess.Popen([str(binary), "--config", str(config), "--nonotify", *map(str, arguments)], env=environment, stdout=subprocess.DEVNULL, stderr=output)


def wait_exit(process):
    """【截图历史测试】【进程结束】等待 process 正常退出；失败时报告实际退出码"""
    process.wait(timeout=8)
    if process.returncode:
        raise AssertionError(f"application exited with {process.returncode}")


def wait_removed(identifier):
    """【截图历史测试】【窗口结束】等待私有 identifier 窗口消失；不终止其他桌面的进程"""
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        if not any(item["id"] == identifier for item in session.windows()):
            return
        time.sleep(0.04)
    raise AssertionError("the private window did not close")


def wait_editor(process, artifact):
    """【截图历史测试】【编辑器等待】等待 process 的普通窗口完成启动，截图写入 artifact；返回窗口"""
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        matching = [item for item in session.windows() if item["pid"] == process.pid]
        if matching:
            session.wait_single_thread(process.pid, timeout=15)
            time.sleep(0.3)
            session.screenshot(artifact)
            return matching[0]
        if process.poll() is not None:
            raise AssertionError("image editor exited before mapping")
        time.sleep(0.04)
    raise AssertionError("image editor did not appear")


def capture_image(binary, environment, config, source, destination, artifacts, name, label=None):
    """【截图历史测试】【完成图片】编辑 source 后真实复制和保存到 destination；返回实际导出 PNG"""
    process = launch(binary, environment, config, ["--fullscreen", "--fullscreen-default-tool", "text" if label else "select", "--to", "cs", "--output", destination, source], artifacts / f"{name}-process.log")
    try:
        wait_editor(process, artifacts / f"{name}-editor.png")
        if label:
            session.control("click", "--logical", 330, 250)
            session.paste_text(label)
            session.control("click", "--logical", 880, 450)
            time.sleep(0.15)
        session.key("ctrl+c")
        wait_exit(process)
        pngs = list(destination.rglob("*.png"))
        if len(pngs) != 1:
            raise AssertionError("image capture did not save exactly one PNG")
        checks.clipboard(environment, pngs[0], artifacts / f"{name}-clipboard.png")
        return pngs[0]
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=3)


def headless(binary, environment, config, destination):
    """【截图历史测试】【无界面捕获】捕获私有桌面到 destination，返回原版 JSON 报告"""
    process = subprocess.run([str(binary), "--config", str(config), "--nonotify", "--all-outputs", "--capture-to", str(destination)], env=environment, capture_output=True, text=True, check=True)
    return json.loads(process.stdout)


def configuration(artifacts):
    """【截图历史测试】【目录隔离】在 artifacts 创建独立应用目录与覆盖配置；返回环境、配置、历史和默认配置路径"""
    root = artifacts / "app"
    root.mkdir()
    home = root / "home"
    home.mkdir()
    config_dir = root / "config/mark-shot"
    config_dir.mkdir(parents=True)
    default = config_dir / "config.toml"
    default.write_text('[history]\nenabled = false\nlimit = 1\n# default config remains unchanged\n')
    history = root / "history"
    config = root / "override.toml"
    config.write_text(f'''# preserved history preferences
[general]
to = "c"
overlay = "window"
[notifications]
enabled = false
[pinned]
always_on_top = false
[history]
enabled = true
index_annotations = false
limit = 50
max_size_mib = 256
directory = {json.dumps(str(history))}
[tools.marker]
width = 13.0
[custom]
preserve = 17
''')
    environment = dict(session.environment(), HOME=str(home), XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    return environment, config, history, default


