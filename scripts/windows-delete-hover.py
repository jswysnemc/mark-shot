#!/usr/bin/env python3
"""【Windows删除悬停】【实际流程】在私有显示验证共享按钮颜色、扩大、恢复及原图导出隔离"""

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import time
from types import SimpleNamespace

from PIL import Image, ImageChops
from wayland_delete_hover_checks import check as check_hover
from windows_ocr_actions import click, key, path_w, snapshot, stop, window, xdo
from windows_test_runtime import run


SPEC = importlib.util.spec_from_file_location("selection_reference", Path(__file__).with_name("wayland-annotation-selection.py"))
SELECTION = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SELECTION)


def execute(command, environment, root, log):
    """【Windows删除悬停】【同构建对照】按command运行实际编辑器两次，root保存完整流程与直接导出对照，返回结果"""
    binary, probe = command
    config = root / "config.toml"
    config.write_text('[general]\nto="c"\n[ui]\ntheme="light"\n[notifications]\nenabled=false\n[history]\nenabled=false\n')
    source = root / "source.png"
    Image.new("RGB", (1280, 800), (30, 34, 38)).save(source)
    report = {"binary_sha256": hashlib.sha256(Path(binary).read_bytes()).hexdigest(), "passed": False, "real_windows_validation": False}
    exports = []
    try:
        for baseline in [False, True]:
            process = subprocess.Popen(["wine", binary, "--config", path_w(config, environment), "--fullscreen", "--default-color", "#EF4444", path_w(source, environment)],
                                       env=environment, cwd=root, stdout=log, stderr=log)
            try:
                identity = window(environment, "Mark Shot", root, process)
                time.sleep(0.3)
                key(environment, identity, "r")
                xdo(environment, "mousemove", "--window", identity, 180, 130)
                xdo(environment, "mousedown", 1)
                xdo(environment, "mousemove", "--window", identity, 280, 210)
                xdo(environment, "mouseup", 1)
                key(environment, identity, "v")
                click(environment, identity, 230, 170)
                time.sleep(0.15)
                if not baseline:
                    region = (160, 90, 330, 240)
                    selected = snapshot(environment, root, "selected")
                    pixels = SELECTION.pixels(selected, (282, 92, 327, 127), (239, 68, 68))
                    assert len(pixels) > 80, "Windows selection did not show its delete button"
                    point = (sum(x for x, y in pixels) / len(pixels), sum(y for x, y in pixels) / len(pixels))

                    def move(point):
                        """【Windows删除悬停】【指针投递】按观察到的point移动私有窗口指针，无返回值，不等待静止指针事件"""
                        xdo(environment, "mousemove", "--window", identity, round(point[0]), round(point[1]))
                        time.sleep(0.15)

                    flow = SimpleNamespace(INPUT=SimpleNamespace(move=move), pixels=SELECTION.pixels,
                                           snapshot=lambda root, name: snapshot(environment, root, name))
                    report["hover"] = check_hover(flow, root, selected, point, region)
                    before = snapshot(environment, root, "before-right")
                    xdo(environment, "click", 3)
                    time.sleep(0.15)
                    assert ImageChops.difference(before.crop(region), snapshot(environment, root, "after-right").crop(region)).getbbox() is None
                    xdo(environment, "click", 1)
                    time.sleep(0.15)
                    assert len(SELECTION.pixels(snapshot(environment, root, "deleted"), region, (251, 146, 60))) == 0
                    key(environment, identity, "ctrl+z")
                    move((230, 170))
                    restored = snapshot(environment, root, "undo-restored")
                    assert ImageChops.difference(selected.crop(region), restored.crop(region)).getbbox() is None
                    report["right_button_inert"] = True
                    report["delete_undo_exact"] = True
                key(environment, identity, "ctrl+c")
                assert process.wait(timeout=8) == 0
                output = root / ("baseline.png" if baseline else "export.png")
                subprocess.run(["wine", probe, "image", path_w(output, environment)], env=environment, cwd=root, check=True, timeout=8)
                exports.append(Image.open(output).convert("RGBA"))
            finally:
                stop(process)
        assert exports[0].size == exports[1].size and exports[0].tobytes() == exports[1].tobytes(), "hover/delete history changed original export"
        report["export_exact"] = True
        report["passed"] = True
        return subprocess.CompletedProcess(command, 0)
    except Exception as error:
        report["error"] = str(error)
        snapshot(environment, root, "failure")
        raise
    finally:
        (root / "workflow.json").write_text(json.dumps(report, indent=2) + "\n")


def main():
    """【Windows删除悬停】【隔离入口】读取候选与新root并执行私有测试，输出运行状态，无参数、无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--probe", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    result = run(options.root.resolve(), [str(options.binary.resolve()), str(options.probe.resolve())], execute=execute)
    print(json.dumps(result, indent=2))
    raise SystemExit(result["exit_code"])


if __name__ == "__main__":
    main()
