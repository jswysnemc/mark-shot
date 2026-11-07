#!/usr/bin/env python3
"""【选区尺寸对照】【真实输入法】连接生产 Qt 字段与私有输入法客户端，记录原生提示及用途"""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess

import wayland_text_format_session as session
from wayland_input_method_transport import InputMethod, build

PROJECT = Path(__file__).resolve().parents[1]


def run(binary, root):
    """【选区尺寸对照】【原生采集】在已校验私有会话运行 binary，root 保存协议与提示，返回真实字段状态"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(QT_QPA_PLATFORM="wayland", QT_IM_MODULE="wayland", QT_FONT_DPI="96", WAYLAND_DEBUG="client", XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/qt-selection-test")
    client = build(root / "client", PROJECT / "target/test-tools/input-method/input-method-unstable-v2.xml")
    with session.input_context(), (root / "qt.log").open("w") as log:
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        method = InputMethod(client, environment, root / "client")
        process = subprocess.Popen([str(binary), "--interactive"], env=environment, stdout=subprocess.DEVNULL, stderr=log)
        try:
            result = {"field": method.state("640"), "qt_binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest()}
            session.screenshot(root / "field.png")
            (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
            return result
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)
            method.close()


def main():
    """【选区尺寸对照】【执行入口】解析生产 Qt binary 和新 root，打印实际协议值，无参数以外返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
