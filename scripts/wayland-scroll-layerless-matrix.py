#!/usr/bin/env python3
"""【普通控制回归】【私有矩阵】隔离代理与原生页面，验证完整长截图动作和实际窗口资源"""

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path

import wayland_scroll_layerless_cases as cases

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("matrix_layerless_runtime", PROJECT / "scripts/wayland-scroll-layerless.py")
RUNTIME = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RUNTIME)
SUPPORT = RUNTIME.SUPPORT


def run(binary, page_binary, root):
    """【普通控制回归】【矩阵入口】按 binary、page_binary 与新 root 执行真实协议缺失的主流程，返回指纹和实际结果"""
    root.mkdir(parents=True, exist_ok=False)
    environment = SUPPORT.session.environment()
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"), MARK_SHOT_LANG="english", MARK_SHOT_LOG="info", DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/layerless-matrix")
    previous = dict(os.environ)
    os.environ.update(environment)
    try:
        with SUPPORT.session.input_context(), SUPPORT.FLOW.NAV.SIZE.pointer_context():
            SUPPORT.session.sway.keyboard_ready()
            with RUNTIME.proxy(environment, root) as name, SUPPORT.page(page_binary, environment, root) as (_page, trace):
                app_environment = dict(environment, WAYLAND_DISPLAY=name)
                results = [cases.verify(SUPPORT, binary, app_environment, root / label, trace, horizontal) for label, horizontal in [("vertical", False), ("horizontal", True)]]
                saved = cases.save_cancel(SUPPORT, binary, app_environment, root / "save-cancel", trace)
        proxy = json.loads((root / "proxy/result.json").read_text())
        assert proxy["dropped"] and proxy["received_fds"] > 0 and proxy["received_fds"] == proxy["sent_fds"]
        assert not proxy["errors"], "the protocol proxy reported a forwarding failure"
        result = {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "cases": results, "save_cancel": saved, "protocol_filter": proxy}
        (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        return result
    finally:
        os.environ.clear()
        os.environ.update(previous)


def main():
    """【普通控制回归】【命令入口】解析独立候选、页面和新证据目录并执行私有矩阵，无参数、无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--page", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(run(options.binary.resolve(), options.page.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
