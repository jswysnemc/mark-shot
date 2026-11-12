"""【截图性能】【对照测量】在私有 Niri 同尺寸空桌面依次测量两个 release 程序，不操作真实桌面。"""

import argparse
import hashlib
import importlib.util
import json
import re
import subprocess
import sys
import time
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
from wayland_test_runtime import TestRuntime

spec = importlib.util.spec_from_file_location("recording_benchmark", SCRIPTS / "recording/benchmark.py")
benchmark = importlib.util.module_from_spec(spec)
spec.loader.exec_module(benchmark)


def mappings(pid):
    """【截图性能】【映射归因】按 pid 返回各内存映射的 RSS/PSS 和私有页，单位 KiB。"""
    items = []
    current = None
    for line in Path(f"/proc/{pid}/smaps").read_text().splitlines():
        if re.match(r"^[0-9a-f]+-[0-9a-f]+ ", line):
            fields = line.split(maxsplit=5)
            current = {"range": fields[0], "name": fields[5] if len(fields) > 5 else "[anonymous]"}
            items.append(current)
        elif current is not None:
            key, _, value = line.partition(":")
            if key in ("Size", "Rss", "Pss", "Private_Clean", "Private_Dirty", "Shared_Clean", "Shared_Dirty", "SwapPss"):
                current[key] = int(value.split()[0])
    return items


def output_size(env, size):
    """【截图性能】【显示尺寸】按私有 env 和 size 调整嵌套窗口，等待合成器确认实际像素尺寸。"""
    window = subprocess.check_output(["xdotool", "search", "--name", "niri"], env=env, text=True).splitlines()[0]
    subprocess.run(["xdotool", "windowsize", window, str(size[0]), str(size[1])], env=env, check=True)
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        output = json.loads(subprocess.check_output(["niri", "msg", "-j", "outputs"], env=env))["winit"]["logical"]
        if (output["width"], output["height"]) == size:
            return
        time.sleep(.05)
    raise RuntimeError("private output did not reach requested resolution")


def run_case(env, args, root, name, seconds):
    """【截图性能】【稳态采样】按 env/args 启动独立实例，预热两秒后测量 seconds，证据写入 root/name。"""
    with (root / f"{name}.log").open("w") as log:
        process = subprocess.Popen(args, env=env, stdout=log, stderr=log)
        try:
            time.sleep(2)
            if process.poll() is not None:
                raise RuntimeError(f"{name} exited; see {root / (name + '.log')}")
            report = benchmark.measure(process.pid, seconds)
            report["command"] = args
            report["mappings"] = mappings(process.pid)
            subprocess.run(["grim", "-o", "winit", str(root / f"{name}.png")], env=env, check=True)
            (root / f"{name}.json").write_text(json.dumps(report, indent=2))
            summary = {key: value for key, value in report.items() if key not in ("samples", "mappings", "command")}
            print(json.dumps({"case": name, **summary}), flush=True)
            return summary
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)
            time.sleep(.25)


def main():
    """【截图性能】【命令入口】读取二进制和证据目录参数，返回相同分辨率下起始与全屏状态的测量。"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--mark-shot", type=Path, required=True)
    parser.add_argument("--babry", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--seconds", type=float, default=15)
    parser.add_argument("--label", default="baseline")
    args = parser.parse_args()
    root = args.root.resolve()
    if root.exists():
        parser.error("use a new private evidence directory")
    binaries = {"mark-shot": args.mark_shot.resolve(), "babry": args.babry.resolve()}
    summary = {"label": args.label, "binaries": {name: {"path": str(path), "size": path.stat().st_size,
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest()} for name, path in binaries.items()}, "cases": {}}
    with TestRuntime(root, 1.0) as env:
        config = root / "mark.toml"
        config.write_text('[desktop]\ntray_enabled=false\nshortcuts_enabled=false\n[ui]\nlanguage="zh-CN"\n[startup]\nselection_loupe=false\n')
        for size in [(1270, 720), (1920, 1080), (3840, 2160)]:
            output_size(env, size)
            for mode in ("region", "screen"):
                for name, binary in binaries.items():
                    command = [str(binary)]
                    if name == "mark-shot":
                        command += ["--config", str(config)]
                        if mode == "screen":
                            command += ["--fullscreen"]
                    else:
                        command += ["--standalone", "--" + mode]
                    case = f"{name}-{size[0]}x{size[1]}-{mode}"
                    summary["cases"][case] = run_case(env, command, root, case, args.seconds)
        (root / "summary.json").write_text(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
