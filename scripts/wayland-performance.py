#!/usr/bin/env python3
"""【Wayland 测试】【性能采样】测量独立桌面的捕获耗时、首次缓冲提交与编辑器空闲资源"""

import argparse
import hashlib
import json
from pathlib import Path
import re
import statistics
import subprocess
import tempfile
import time

from PIL import Image

import wayland_test_session as session


FRAME_TIMING = re.compile(r"\[\s*([\d.]+)ms\].*frames staged \+ flushed")
IDLE_SETTLE_SECONDS = 1.0


def isolated_environment(base, root):
    """【性能测试】【配置隔离】用 base 会话与 root 创建应用目录；返回不包含用户配置的环境"""
    config = root / "config/mark-shot"
    config.mkdir(parents=True)
    (config / "config.toml").write_text('[general]\nto = ""\n[notifications]\nenabled = false\n')
    return dict(base, XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))


def capture_samples(binary, environment, artifacts, count):
    """【性能测试】【捕获耗时】以 binary、environment 无界面捕获 count 次；返回耗时及报告，PNG 证据写入 artifacts"""
    results = []
    for index in range(count):
        destination = artifacts / f"capture-{index}.png"
        start = time.monotonic()
        process = subprocess.run([str(binary), "--all-outputs", "--capture-to", str(destination), "--nonotify"], env=environment, capture_output=True, text=True, check=True)
        elapsed = (time.monotonic() - start) * 1000
        report = json.loads(process.stdout)
        with Image.open(destination) as image:
            if image.format != "PNG" or image.width < 1 or image.height < 1:
                raise RuntimeError("the headless capture did not produce a usable PNG")
            size = list(image.size)
        results.append({"total_ms": elapsed, "size": size, "report": report})
    return results


def wait_frame(process, log, started, timeout=8):
    """【性能测试】【首帧等待】等待 process 向 log 输出提交阶段；started 为父进程起点，返回内部耗时及观察上限"""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        text = log.read_text(errors="replace")
        match = FRAME_TIMING.search(text)
        if match:
            return {"frames_staged_ms": float(match[1]), "parent_observed_ms": (time.monotonic() - started) * 1000}
        if process.poll() is not None:
            raise RuntimeError(f"the editor exited before its first buffer submission: {text}")
        time.sleep(0.005)
    raise RuntimeError(f"the editor did not report its first buffer submission: {log.read_text(errors='replace')}")


def stop_editor(process):
    """【性能测试】【编辑器关闭】通过隔离会话取消 process；返回后进程已退出，输入关闭失败时抛出异常"""
    for _ in range(3):
        if process.poll() is not None:
            return
        session.key("Escape")
        try:
            process.wait(timeout=0.5)
            return
        except subprocess.TimeoutExpired:
            pass
    raise RuntimeError("the idle editor did not close through its cancellation workflow")


def editor_samples(binary, environment, artifacts, count):
    """【性能测试】【编辑器采样】以 binary、environment 启动 count 次；返回首帧与空闲快照，阶段日志写入 artifacts"""
    results = []
    for index in range(count):
        log = artifacts / f"startup-{index}.log"
        with log.open("w") as errors:
            started = time.monotonic()
            process = subprocess.Popen([str(binary), "--fullscreen", "--fullscreen-default-tool", "pen", "--speed", "--nonotify"], env=environment, stdout=subprocess.DEVNULL, stderr=errors)
            try:
                timing = wait_frame(process, log, started)
                session.wait_single_thread(process.pid, timeout=20)
                # 1. 【性能测试】【空闲边界】工作线程退出不等于启动动画和字体合并结束，稳定期不计入首帧耗时
                time.sleep(IDLE_SETTLE_SECONDS)
                before = session.resources(process.pid)
                time.sleep(1)
                after = session.resources(process.pid)
                if after["threads"] != 1 or after["cpu_ticks"] != before["cpu_ticks"]:
                    raise RuntimeError(f"the idle editor retained work: {before} -> {after}")
                stop_editor(process)
                if process.returncode:
                    raise RuntimeError(f"the editor cancellation returned {process.returncode}")
                results.append({**timing, "before": before, "after": after, "exit_code": process.returncode})
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=3)
    return results


def distribution(samples, key):
    """【性能测试】【分布整理】从 samples 的 key 数值返回最小值、中位数与最大值；不推断冷启动性能"""
    values = [sample[key] for sample in samples]
    return {"min": min(values), "median": statistics.median(values), "max": max(values)}


def run(binary, artifacts, count):
    """【性能测试】【完整采样】在现有独立桌面测试 binary 共 count 次；将环境、构建指纹和测量证据保存到 artifacts"""
    artifacts.mkdir(parents=True, exist_ok=True)
    base = session.environment()
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    outputs = json.loads(subprocess.check_output(["niri", "msg", "-j", "outputs"], env=base))
    with tempfile.TemporaryDirectory(prefix="mark-shot-performance-") as temporary:
        root = Path(temporary)
        environment = isolated_environment(base, root)
        captures = capture_samples(binary, environment, artifacts, count)
        editors = editor_samples(binary, environment, artifacts, count)
        if (root / "data/mark-shot/models").exists():
            raise RuntimeError("ordinary capture initialized the OCR model directory")
    if fingerprint != hashlib.sha256(binary.read_bytes()).hexdigest():
        raise RuntimeError("the executable changed during the performance run")
    report = {"binary": str(binary), "binary_bytes": binary.stat().st_size, "sha256": fingerprint, "outputs": outputs, "sample_count": count, "idle_settle_seconds": IDLE_SETTLE_SECONDS, "measurement": "warm local runs; frames_staged_ms starts inside the screenshot workflow and ends after buffer staging and flush; parent_observed_ms is an observation upper bound, not compositor presentation time; idle CPU is sampled for one second after startup threads exit and the explicit UI settling interval", "capture_total_ms": distribution(captures, "total_ms"), "editor_frames_staged_ms": distribution(editors, "frames_staged_ms"), "editor_parent_observed_ms": distribution(editors, "parent_observed_ms"), "idle_rss_kib": distribution([{"rss": item["after"]["rss_kib"]} for item in editors], "rss"), "capture_samples": captures, "editor_samples": editors}
    (artifacts / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({key: report[key] for key in ("binary_bytes", "sha256", "sample_count", "capture_total_ms", "editor_frames_staged_ms", "editor_parent_observed_ms", "idle_rss_kib")}))


def main():
    """【性能测试】【入口】解析二进制、证据目录和次数；次数不足或测试失败时返回非零退出码"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, default=Path("target/wayland-performance"))
    parser.add_argument("--runs", type=int, default=6)
    options = parser.parse_args()
    if options.runs < 3:
        parser.error("at least three samples are required")
    run(options.binary.resolve(), options.artifacts.resolve(), options.runs)


if __name__ == "__main__":
    main()
