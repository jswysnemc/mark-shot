#!/usr/bin/env python3
"""【持续捕获回归】【私有运行器】独立D-Bus与PipeWire验证真实授权复用，不访问用户设备"""

import argparse
from contextlib import ExitStack
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

import wayland_text_sway_session as session

CONFIG = '''context.properties = { core.daemon = true core.name = mark-shot-pw support.dbus = false }
context.spa-libs = { support.* = support/libspa-support }
context.modules = [
    { name = libpipewire-module-protocol-native }
    { name = libpipewire-module-client-node }
    { name = libpipewire-module-metadata }
    { name = libpipewire-module-access args = { access.socket = { mark-shot-pw = unrestricted mark-shot-pw-manager = unrestricted } } }
    { name = libpipewire-module-adapter }
    { name = libpipewire-module-link-factory }
]
'''
CLIENT = '''context.properties = { support.dbus = false }
context.spa-libs = { support.* = support/libspa-support video.convert.* = videoconvert/libspa-videoconvert }
context.modules = [ { name = libpipewire-module-protocol-native } { name = libpipewire-module-client-node } { name = libpipewire-module-adapter } { name = libpipewire-module-metadata } ]
'''


def wait(predicate, message, timeout=5):
    """【持续捕获回归】【条件等待】在timeout内检查predicate，返回有效结果，失败以message报告"""
    deadline = time.monotonic() + timeout
    while True:
        result = predicate()
        if result:
            return result
        if time.monotonic() >= deadline:
            raise AssertionError(message)
        time.sleep(0.03)


def close(process):
    """【持续捕获回归】【进程清理】只回收本次process，不操作任何其他服务，无返回值"""
    if process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=3)


def spawn(stack, command, environment, path):
    """【持续捕获回归】【进程启动】按command与environment启动私有进程，path记录日志，stack负责回收，返回句柄"""
    log = stack.enter_context(path.open("w"))
    process = subprocess.Popen(command, env=environment, stdout=log, stderr=log)
    stack.callback(close, process)
    return process


def objects(environment):
    """【持续捕获回归】【原生对象】查询environment指定私有PipeWire的真实对象，返回JSON对象数组"""
    return json.loads(subprocess.check_output(["pw-dump", "-r", "mark-shot-pw"], env=environment, stderr=subprocess.DEVNULL, timeout=3))


def source_node(environment):
    """【持续捕获回归】【测试来源】返回environment私有测试视频node编号，未建立时返回空值"""
    for value in objects(environment):
        props = value.get("info", {}).get("props", {})
        if value.get("type") == "PipeWire:Interface:Node" and props.get("node.name") == "private-screen":
            return value["id"]
    return None


def link(environment, node):
    """【持续捕获回归】【原生连接】把测试node输出端口接入生产捕获流，只在两个端口实际存在时创建，返回连接状态"""
    values = objects(environment)
    captures = [value["id"] for value in values if value.get("type") == "PipeWire:Interface:Node" and value.get("info", {}).get("props", {}).get("media.category") == "Capture"]
    ports = [(value["id"], value.get("info", {}).get("props", {})) for value in values if value.get("type") == "PipeWire:Interface:Port"]
    output = next((value for value, props in ports if str(props.get("node.id")) == str(node) and props.get("port.direction") == "out"), None)
    input_port = next((value for value, props in ports if props.get("port.direction") == "in" and any(str(props.get("node.id")) == str(capture) for capture in captures)), None)
    if output is None or input_port is None:
        return False
    if any(value.get("type") == "PipeWire:Interface:Link" and value.get("info", {}).get("output-port-id") == output and value.get("info", {}).get("input-port-id") == input_port for value in values):
        return True
    subprocess.run(["pw-link", "-r", "mark-shot-pw", str(output), str(input_port)], env=environment, check=True, capture_output=True, timeout=3)
    return True


def cpu(process):
    """【持续捕获回归】【线程采样】读取process中生产PipeWire线程的实际CPU计数，返回线程编号与计数"""
    for path in Path(f"/proc/{process.pid}/task").iterdir():
        if (path / "comm").read_text().strip() .startswith("mark-shot-pipew"):
            fields = (path / "stat").read_text().split(") ", 1)[1].split()
            return {"tid": int(path.name), "ticks": int(fields[11]) + int(fields[12])}
    return None


def verify(binary, environment, root, node, reject=False, cancel=False, invalid_restore=False, source_cancel=False):
    """【持续捕获回归】【完整协议】按binary执行真实私有Portal和PipeWire会话，reject控制授权失败，返回生命周期证据"""
    root.mkdir()
    if reject:
        (Path(environment["XDG_STATE_HOME"]) / "mark-shot/token").unlink(missing_ok=True)
    trace, ready, result = root / "portal.json", root / "portal.log", root / "result.json"
    with ExitStack() as stack:
        server = spawn(stack, [str(binary), "server", str(node), str(Path(environment["PIPEWIRE_RUNTIME_DIR"]) / "mark-shot-pw"), str(trace), *(["reject"] if reject else []), *(["invalid-restore"] if invalid_restore else []), *(["source-cancel"] if source_cancel else [])], environment, ready)
        wait(lambda: "ready" in ready.read_text(), "the private Portal service did not start")
        log = root / "probe.log"
        probe = spawn(stack, [str(binary), "probe", str(result), *(["reject"] if reject or source_cancel else []), *(["cancel"] if cancel else [])], environment, log)
        if not reject and not source_cancel:
            if cancel:
                wait(lambda: "cancelled" in log.read_text(), "pending frame cancellation did not complete")
            wait(lambda: link(environment, node) if probe.poll() is None else result.exists(), "the real PipeWire capture ports did not connect")
            wait(lambda: "idle" in log.read_text() or probe.poll() is not None, "the production capture did not reach its paused phase", timeout=8)
            assert probe.poll() is None, log.read_text()
            first = cpu(probe)
            assert first is not None, "the production PipeWire thread is missing"
            time.sleep(0.4)
            second = cpu(probe)
            assert first == second, f"paused PipeWire capture keeps working: {first} -> {second}"
        assert probe.wait(timeout=8) == 0, log.read_text()
        counts = json.loads(trace.read_text())
        if reject or source_cancel:
            assert counts == {"server": 1, "create": 1, "select": 1, "close": 1}, counts
        elif invalid_restore:
            assert counts == {"server": 1, "create": 2, "select": 2, "start": 1, "remote": 1, "close": 2}, counts
        else:
            assert counts == {"server": 1, "create": 1, "select": 1, "start": 1, "remote": 1, "close": 1}, counts
        report = json.loads(result.read_text())
        report["actual_portal_calls"] = counts
        if not reject and not source_cancel:
            report["paused_cpu"] = [first, second]
            assert len({sample["threads"] for sample in report["samples"]}) == 1, "frame requests created additional threads"
        assert server.poll() is None
    (root / "evidence.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def run(binary, root):
    """【持续捕获回归】【私有基础设施】按binary和新root创建独立总线与视频源，返回完整证据，不继承用户服务地址"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    runtime = Path(environment["XDG_RUNTIME_DIR"])
    environment.update(PIPEWIRE_RUNTIME_DIR=str(runtime), PIPEWIRE_REMOTE="mark-shot-pw", XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    config = root / "pipewire.conf"
    config.write_text(CONFIG)
    (root / "client.conf").write_text(CLIENT)
    environment["PIPEWIRE_CONFIG_DIR"] = str(root)
    metadata = runtime / "portal-test-runtime.json"
    metadata.write_text(json.dumps({"owner": os.getpid(), "root": str(root)}) + "\n")
    with ExitStack() as stack:
        stack.callback(metadata.unlink)
        bus = subprocess.Popen(["dbus-daemon", "--session", "--nofork", "--print-address=1", f"--address=unix:path={runtime / 'portal-test-bus'}"], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        stack.callback(close, bus)
        environment["DBUS_SESSION_BUS_ADDRESS"] = bus.stdout.readline().decode().strip()
        assert environment["DBUS_SESSION_BUS_ADDRESS"].startswith(f"unix:path={runtime}/")
        daemon = spawn(stack, ["pipewire", "-c", str(config)], environment, root / "pipewire.log")
        wait(lambda: (runtime / "mark-shot-pw").is_socket(), "the private PipeWire daemon did not start")
        producer = spawn(stack, ["gst-launch-1.0", "-q", "videotestsrc", "is-live=true", "pattern=red", "!", "video/x-raw,format=BGRx,width=64,height=32,framerate=30/1", "!", "pipewiresink", "mode=provide", "stream-properties=props,node.name=private-screen,media.class=Video/Source,node.driver=true"], environment, root / "source.log")
        node = wait(lambda: source_node(environment), "the private red video source did not create a node")
        # 1. 【持续捕获回归】【取消授权】先验证来源取消响应，不能在用户取消后继续Start或打开视频远端
        report = {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "node": node,
                  "source_cancel": verify(binary, environment, root / "source-cancel", node, source_cancel=True),
                  "success": verify(binary, environment, root / "success", node),
                  "cancelled": verify(binary, environment, root / "cancelled", node, cancel=True),
                  "invalid_restore": verify(binary, environment, root / "invalid-restore", node, invalid_restore=True),
                  "rejected": verify(binary, environment, root / "rejected", node, True)}
        assert daemon.poll() is None and producer.poll() is None
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    """【持续捕获回归】【命令入口】解析独立binary和root并执行本次私有流程，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
