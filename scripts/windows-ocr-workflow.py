#!/usr/bin/env python3
"""【Windows识别回归】【真实模型】在新Wine与不可见Xvfb中执行首次下载、内置推理和独立结果窗口"""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time

from wayland_model_locator import download_position
from windows_ocr_actions import clipboard_text, copy_all, key, model_assets, path_w, snapshot, stop, visible, window, xdo
from windows_ocr_result_checks import result_checks
from windows_test_runtime import run


ROOT = Path(__file__).resolve().parents[1]
ASSETS = {
    "pp-ocrv5_mobile_det.onnx": (4_826_518, "1eb7b4f7ab657ebd1c66d5f79bca7497f29768a2e3c15e52daecbba1a8e4a039"),
    "pp-ocrv5_mobile_rec.onnx": (16_562_373, "243a0f06d826761323e9045e9b113ab2c191c3aa50565585e628300b8eda0224"),
    "ppocrv5_dict.txt": (74_012, "d1979e9f794c464c0d2e0b70a7fe14dd978e9dc644c0e71f14158cdf8342af1b"),
}
EXPECTED = ("Wayland Native Screenshot", "Mark Shot OCR 2026", "Local text recognition")


def wait_download(environment, root, process, application_log):
    """【Windows识别回归】【下载完成】等待真实process下载三项固定资源，核对大小与摘要，返回清单和耗时，下载失败保存画面"""
    started = time.monotonic()
    deadline = started + 180
    while time.monotonic() < deadline:
        assets = model_assets(root)
        if ASSETS.keys() <= assets.keys():
            for name, (size, digest) in ASSETS.items():
                assert assets[name]["bytes"] == size and assets[name]["sha256"] == digest, f"invalid installed model: {name}"
            assert not any(name.endswith(".part") for name in assets), "download left partial model files"
            return {"assets": assets, "seconds": time.monotonic() - started, "real_remote_download": True}
        text = application_log.read_text() if application_log.exists() else ""
        if "download failed" in text.lower() or "下载失败" in text or process.poll() is not None:
            snapshot(environment, root, "download-failure")
            raise AssertionError(f"first Windows download failed: {text[-3000:]}, exit={process.poll()}")
        time.sleep(0.1)
    snapshot(environment, root, "download-timeout")
    raise AssertionError("first Windows model download exceeded 180 seconds")


def execute(command, environment, root, log):
    """【Windows识别回归】【同构建流程】用command中的应用和probe执行下载及结果编辑，逐项保存root中的证据并返回完成状态"""
    binary, probe, *fonts = command
    environment = dict(environment, LANG="en_US.UTF-8", LC_ALL="en_US.UTF-8")
    config_path = root / "config.toml"
    config_path.write_text('[general]\nto=""\n[ui]\ntheme="light"\nlanguage="en"\n[notifications]\nenabled=false\n[history]\nenabled=false\n[ocr]\nresult_panel=true\n')
    config = path_w(config_path, environment)
    application_log = root / "application.log"
    report = {"binary_sha256": hashlib.sha256(Path(binary).read_bytes()).hexdigest(), "cases": {},
              "real_windows_validation": False, "passed": False}
    if fonts:
        destination = root / "wine-prefix/drive_c/windows/Fonts"
        destination.mkdir(exist_ok=True)
        report["test_fonts"] = []
        for source in map(Path, fonts):
            shutil.copyfile(source, destination / source.name)
            report["test_fonts"].append({"name": source.name, "bytes": source.stat().st_size,
                                         "sha256": hashlib.sha256(source.read_bytes()).hexdigest()})
    processes = []
    try:
        # 1. 【Windows识别回归】【首次引导】只启动实际应用，不预置模型、不改写模型URL，右键与空闲不能开始下载
        process = subprocess.Popen(["wine", binary, "--config", config, "--nonotify", "--debug", "--debug-log",
                                    path_w(application_log, environment), "--ocr", path_w(ROOT / "tests/fixtures/ocr.png", environment)],
                                   env=environment, cwd=root, stdout=log, stderr=log)
        processes.append(process)
        editor = window(environment, "Mark Shot", root, process)
        time.sleep(0.25)
        guide = snapshot(environment, root, "first-use-light")
        position = download_position(guide, index=4)
        assert not model_assets(root), "first use downloaded a model without a selection"
        xdo(environment, "windowfocus", "--sync", editor)
        xdo(environment, "mousemove", "--sync", *position)
        xdo(environment, "click", 3)
        time.sleep(0.2)
        assert not model_assets(root), "right click started model download"
        snapshot(environment, root, "right-click-dismissed")
        key(environment, editor, "p")
        key(environment, editor, "o")
        time.sleep(0.2)
        position = download_position(snapshot(environment, root, "first-use-reopened"), index=4)
        xdo(environment, "mousemove", "--sync", *position)
        xdo(environment, "click", 1)
        report["cases"]["download"] = wait_download(environment, root, process, application_log)
        (root / "workflow.json").write_text(json.dumps(report, indent=2) + "\n")

        # 2. 【Windows识别回归】【实际推理】下载完成自动继续，等待独立结果并验证三行原文，编辑器退出后仍可操作
        identity = window(environment, "OCR Result", root, timeout=60)
        assert process.wait(timeout=8) == 0, "recognition editor did not exit after result handoff"
        assert not visible(environment, "Mark Shot"), "recognition editor remained after result handoff"
        recognized = copy_all(probe, environment, root, identity, "recognized")
        assert all(line in recognized for line in EXPECTED), recognized
        snapshot(environment, root, "recognized-result")
        report["cases"]["recognition"] = {"text": recognized, "editor_exited": True, "native_result_window": True}
        checks = result_checks(binary, probe, environment, root, identity, recognized)
        report["cases"]["result"] = checks
        key(environment, identity, "Escape")
        deadline = time.monotonic() + 5
        while visible(environment, "OCR Result") and time.monotonic() < deadline:
            time.sleep(0.05)
        assert not visible(environment, "OCR Result"), "result window remained after close"
        assert clipboard_text(probe, environment, root, "after-result-close") == checks["edited_text"], "closing result discarded Unicode clipboard"

        # 3. 【Windows识别回归】【缓存复用】同一候选再次识别本地文件，不显示模型引导，结果仍包含原文
        process = subprocess.Popen(["wine", binary, "--config", config, "--nonotify", "--ocr",
                                    path_w(ROOT / "tests/fixtures/ocr.png", environment)], env=environment, cwd=root, stdout=log, stderr=log)
        processes.append(process)
        identity = window(environment, "OCR Result", root, timeout=60)
        assert process.wait(timeout=8) == 0
        copy_all(probe, environment, root, identity, "cached-recognized", recognized)
        key(environment, identity, "Escape")
        assert model_assets(root) == report["cases"]["download"]["assets"], "cached OCR changed model files"
        report["cases"]["cached_recognition"] = True
        report["passed"] = True
        return subprocess.CompletedProcess(command, 0)
    except Exception as error:
        report["error"] = str(error)
        snapshot(environment, root, "failure")
        raise
    finally:
        (root / "workflow.json").write_text(json.dumps(report, indent=2) + "\n")
        for process in processes:
            stop(process)


def main():
    """【Windows识别回归】【私有入口】读取binary、probe和新root，执行真实流程并输出状态，无参数、无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--probe", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--font", action="append", type=Path, default=[])
    options = parser.parse_args()
    command = [str(options.binary.resolve()), str(options.probe.resolve()), *(str(font.resolve()) for font in options.font)]
    report = run(options.root.resolve(), command, execute=execute)
    print(json.dumps(report, indent=2))
    raise SystemExit(report["exit_code"])


if __name__ == "__main__":
    main()
