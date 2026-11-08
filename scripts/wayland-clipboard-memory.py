#!/usr/bin/env python3
"""【图片剪贴板回归】【真实协议】在私有输出验证 ext/wlr、重复粘贴、KDE MIME 和接管释放"""

import argparse
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import selectors
import socket
import subprocess
import threading

from wayland_layer_shell_filter import Relay
from wayland_memory_metrics import MemorySampler


@contextmanager
def connection(hidden):
    """【图片剪贴板回归】【协议隔离】隐藏 hidden 接口并转发唯一测试连接，返回客户端套接字及转发器，退出关闭全部私有描述符"""
    client, endpoint = socket.socketpair()
    upstream = socket.socket(socket.AF_UNIX)
    upstream.connect(str(Path(os.environ["XDG_RUNTIME_DIR"]) / os.environ["WAYLAND_DISPLAY"]))
    relay = Relay()
    relay.add(endpoint, upstream, hidden)
    worker = threading.Thread(target=relay.run)
    worker.start()
    try:
        yield client, relay
    finally:
        client.close()
        relay.stop()
        worker.join(timeout=5)
        assert not worker.is_alive(), "clipboard protocol relay did not stop"
        relay.close()


def digest(path):
    """【图片剪贴板回归】【文件校验】分块计算 path 的 SHA-256，返回摘要，不完整读入图片"""
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def check(options, protocol, primary=False):
    """【图片剪贴板回归】【协议流程】按 options、protocol、primary 启动辅助进程并验证两次完整粘贴，返回内存及协议证据"""
    root = options.root / (protocol + ("-primary" if primary else ""))
    root.mkdir()
    hidden = ["ext_data_control_manager_v1"] if protocol == "wlr" else []
    with connection(hidden) as (client, relay), options.png.open("rb") as source, (root / "client.log").open("w") as log:
        environment = dict(os.environ, WAYLAND_SOCKET=str(client.fileno()), WAYLAND_DEBUG="client",
                           XDG_CONFIG_HOME=str(root / "config"), XDG_CACHE_HOME=str(root / "cache"),
                           XDG_STATE_HOME=str(root / "state"), XDG_DATA_HOME=str(root / "data"))
        command = [str(options.binary), "--clipboard-daemon"] + (["primary"] if primary else [])
        process = subprocess.Popen(command, stdin=source, stdout=subprocess.PIPE, stderr=log,
                                   env=environment, pass_fds=(client.fileno(),))
        try:
            with MemorySampler(process, root / "samples.jsonl", interval=0.015) as sampler:
                with selectors.DefaultSelector() as ready:
                    ready.register(process.stdout, selectors.EVENT_READ)
                    assert ready.select(30), "clipboard helper readiness timed out"
                assert process.stdout.readline() == b"\n", (root / "client.log").read_text()[-2000:]
                sampler.mark("serving")
                selection = ["--primary"] if primary else []
                types = subprocess.check_output(["wl-paste", *selection, "--list-types"], timeout=5).decode().splitlines()
                assert "image/png" in types and "x-kde-force-image-copy" in types
                for index in range(2):
                    target = root / f"paste-{index}.png"
                    with target.open("wb") as stream:
                        subprocess.run(["wl-paste", *selection, "--type", "image/png"], stdout=stream, check=True, timeout=30)
                    assert digest(target) == digest(options.png), "clipboard changed the original PNG bytes"
                assert subprocess.check_output(["wl-paste", *selection, "--type", "x-kde-force-image-copy"], timeout=5) == b""
                sampler.mark("takeover")
                subprocess.run(["wl-copy", *selection, "--clear"], check=True)
                assert process.wait(timeout=5) == 0, "clipboard helper did not exit cleanly"
            report = sampler.report()
            report.update(protocol=protocol, primary=primary, bytes=options.png.stat().st_size,
                          all_bytes_equal=True, helper_exited=True)
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)
            process.stdout.close()
    report["relay"] = relay.report()
    assert not report["relay"]["errors"], report["relay"]
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    text = (root / "client.log").read_text()
    expected = "zwlr_data_control_manager_v1" if protocol == "wlr" else "ext_data_control_manager_v1"
    assert f'Some("{expected}")' in text, f"{protocol} manager was not bound"
    assert report["peak_rss_with_unmapped_original_kib"] <= options.max_mib * 1024, "clipboard helper memory exceeds budget"
    return {"protocol": protocol, "primary": primary,
            "peak_mib": report["peak_rss_with_unmapped_original_kib"] / 1024, "all_bytes_equal": True}


def main():
    """【图片剪贴板回归】【命令入口】解析测试二进制、原图和结果目录，输出四个协议场景的验证结果，无参数"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--png", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--max-mib", type=float, default=48)
    options = parser.parse_args()
    options.binary, options.png, options.root = options.binary.resolve(), options.png.resolve(), options.root.resolve()
    options.root.mkdir(parents=True, exist_ok=False)
    results = [check(options, protocol, primary) for protocol in ("ext", "wlr") for primary in (False, True)]
    (options.root / "result.json").write_text(json.dumps(results, indent=2) + "\n")
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
