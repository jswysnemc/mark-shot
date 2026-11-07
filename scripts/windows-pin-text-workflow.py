#!/usr/bin/env python3
"""【Windows文本贴图】【真实流程】私有Wine中验证Unicode文本粘贴及空文本拒绝，保留完整证据"""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

from PIL import Image
from windows_ocr_actions import key, path_w, set_clipboard, snapshot, stop, visible, window
from windows_pin_checks import copied
from windows_pin_text_checks import paste_text
from windows_test_runtime import run


def execute(command, environment, root, log):
    """【Windows文本贴图】【同构建验证】按command启动实际父贴图并验证文本子窗，root保存报告，返回完成码并回收所有本次资源"""
    binary, probe, *fonts = command
    config = root / "config.toml"
    config.write_text('[ui]\ntheme="light"\nlanguage="en"\n[notifications]\nenabled=false\n[history]\nenabled=false\n')
    report = {"binary_sha256": hashlib.sha256(Path(binary).read_bytes()).hexdigest(),
              "cases": {}, "passed": False, "real_windows_validation": False, "test_fonts": []}
    for source in map(Path, fonts):
        shutil.copyfile(source, root / "wine-prefix/drive_c/windows/Fonts" / source.name)
        report["test_fonts"].append({"name": source.name, "sha256": hashlib.sha256(source.read_bytes()).hexdigest()})
    image = Image.new("RGBA", (240, 160), (70, 110, 150, 255))
    source = root / "parent.png"
    image.save(source)
    process = subprocess.Popen(["wine", binary, "--config", path_w(config, environment), "--pin", path_w(source, environment)],
                               env=environment, cwd=root, stdout=log, stderr=log)
    try:
        parent = window(environment, "Pinned Mark Shot", root, process)
        for name, text in [("english", "Clipboard text\n\nThird line"), ("unicode", "中文文本贴图\n标点：测试，保留换行\nSupplementary: 𠮷")]:
            report["cases"][name] = paste_text(binary, probe, environment, root, parent, text, name)
        copied(probe, environment, root, parent, "parent-after-text", image)
        # 1. 【Windows文本贴图】【空文本拒绝】直接执行同一入口并等待失败，不能仅凭未出现窗口判定处理完成
        set_clipboard(binary, environment, root, " \n\t")
        result = subprocess.run(["wine", binary, "--config", path_w(config, environment), "--pin-clipboard"],
                                env=environment, cwd=root, capture_output=True, text=True, timeout=20)
        (root / "empty-result.txt").write_text(result.stdout + result.stderr)
        assert result.returncode != 0 and "no image or text" in result.stderr, "blank text was not rejected explicitly"
        assert visible(environment, "Pinned Mark Shot") == [parent]
        report["cases"]["empty"] = {"exit_code": result.returncode, "no_new_window": True, "explicit_error": True}
        key(environment, parent, "Escape")
        assert process.wait(timeout=8) == 0
        report["passed"] = True
        return subprocess.CompletedProcess(command, 0)
    except Exception as error:
        report["error"] = str(error)
        snapshot(environment, root, "failure")
        raise
    finally:
        stop(process)
        (root / "workflow.json").write_text(json.dumps(report, indent=2) + "\n")


def main():
    """【Windows文本贴图】【隔离入口】读取binary、probe、新root与可选测试字体，返回进程状态，无参数"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--probe", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--font", type=Path, action="append", default=[])
    options = parser.parse_args()
    report = run(options.root.resolve(), [str(path.resolve()) for path in [options.binary, options.probe, *options.font]], execute=execute)
    print(json.dumps(report, indent=2))
    raise SystemExit(report["exit_code"])


if __name__ == "__main__":
    main()
