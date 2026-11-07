#!/usr/bin/env python3
"""【单实例多输出回归】【延迟替换】仅在私有双输出 Sway 中验证范围切换与实际交付"""

import argparse
import hashlib
import io
import json
from pathlib import Path
import re
import time

from PIL import Image
import wayland_single_instance_checks as checks
import wayland_single_instance_session as runtime
import wayland_text_format_session as session
from wayland_headless_runtime import HeadlessRuntime
from wayland_pin_pointer import GlobalPointer, private_environment


def run(binary, root, sway):
    """【单实例多输出回归】【独立流程】按 binary、root、sway 建立两种比例的私有输出，返回真实范围及 PNG 尺寸证据"""
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    reports = []
    with HeadlessRuntime(root / "runtime", sway) as environment:
        source = json.loads((root / "runtime/runtime.json").read_text())
        metadata = {"compositor": "sway", "sway_pid": source["sway_pid"],
                    "sway_socket": environment["SWAYSOCK"], "wayland_display": environment["WAYLAND_DISPLAY"],
                    "outputs": {output["name"]: {"logical": dict(output["rect"], scale=output["scale"], transform=output["transform"]),
                                                  "modes": [output["current_mode"]], "current_mode": 0} for output in source["outputs"]}}
        (Path(environment["XDG_RUNTIME_DIR"]) / "runtime.json").write_text(json.dumps(metadata, indent=2) + "\n")
        with private_environment(environment), session.input_context(), GlobalPointer() as pointer:
            session.sway.keyboard_ready()
            pointer.move((100, 100))
            time.sleep(0.08)
            for name, initial, replacement, count, size in [
                ("current-to-all", [], ["--all-outputs"], 2, (2640, 1080)),
                ("all-to-current", ["--all-outputs"], [], 1, (1200, 900)),
            ]:
                with runtime.launch(binary, root, name, ["--delay", "1.5", "--fullscreen", *initial]) as first:
                    path = runtime.wait_endpoint(first)
                    assert ".damage_buffer(" not in first.log.read_text()
                    with runtime.launch(binary, root, name + "-replace", ["--delay", "0.15", "--fullscreen", *replacement]) as second:
                        elapsed = checks.forwarded(second)
                    first.wait_present()
                    idle = checks.idle(first)
                    layers = re.findall(r'get_layer_surface\([^\n]+Some\("mark-shot"\)\)', first.log.read_text())
                    assert len(layers) == count, (layers, count)
                    session.key("Return")
                    assert first.process.wait(timeout=5) == 0
                    with Image.open(io.BytesIO(first.output.read_bytes())) as image:
                        assert image.size == size, (image.size, size)
                    assert not path.exists()
                    reports.append({"name": name, "forward_wait_seconds": elapsed, "actual_overlay_surfaces": len(layers),
                                    "png_size": list(size), "idle": idle})
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == fingerprint
    result = {"binary_sha256": fingerprint, "compositor": "sway", "outputs": metadata["outputs"], "cases": reports,
              "limits": ["mixed-output scope and dimensions verified; mixed-output pixel resampling is outside this check"]}
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    """【单实例多输出回归】【命令入口】解析固定 binary、root 与 sway，仅运行私有会话，输出两个场景的实际范围"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--sway", type=Path, default=Path("target/test-tools/sway/usr/bin/sway"))
    options = parser.parse_args()
    root = options.root.resolve()
    root.mkdir(parents=True, exist_ok=False, mode=0o700)
    result = run(options.binary.resolve(), root, options.sway.resolve())
    print(json.dumps({"binary_sha256": result["binary_sha256"], "cases": [{key: value for key, value in case.items() if key != "idle"}
                                                                                     for case in result["cases"]]}, indent=2))


if __name__ == "__main__":
    main()
