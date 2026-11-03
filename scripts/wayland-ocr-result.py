#!/usr/bin/env python3
"""【Wayland 测试】【OCR 结果】验证原生结果编辑、页面保留、表格复制和识别进程释放"""

import argparse
import csv
import io
import json
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import time

from PIL import Image

from wayland_model_locator import download_position
import wayland_test_session as session


ROOT = Path(__file__).resolve().parents[1]
ROWS = [["Item", "Count", "Total"], ["Paper", "12", "36"], ["Ink", "2", "24"]]
SOURCE = "\n".join("\t".join(row) for row in ROWS)


def frame():
    """【结果测试】【固定文档】创建表格文字、位置与 PNG 原图的二进制帧；不调用识别模型"""
    lines = [{"text": text, "bounds": [38 + column * 220, 32 + row * 66, max(26, len(text) * 15), 24]}
             for row, values in enumerate(ROWS) for column, text in enumerate(values)]
    header = json.dumps({"text": SOURCE, "lines": lines}).encode()
    png = (ROOT / "tests/fixtures/ocr-table.png").read_bytes()
    return b"MSOCR001" + struct.pack("<II", len(header), len(png)) + header + png


def launch_result(binary, env, artifacts):
    """【结果测试】【原生启动】使用 binary 和 env 打开固定结果窗口；把诊断写入 artifacts 并返回进程"""
    log = (artifacts / "result-process.log").open("w")
    process = subprocess.Popen([str(binary), "--ocr-result", "--nonotify"], stdin=subprocess.PIPE,
                               stdout=subprocess.DEVNULL, stderr=log, env=env)
    log.close()
    process.stdin.write(frame())
    process.stdin.close()
    return process


def paste(text):
    """【结果测试】【粘贴输入】将 text 粘贴到当前编辑区；等待读取剪贴板和排版完成"""
    session.control("type", text)
    time.sleep(0.15)


def expected_csv(rows):
    """【结果测试】【CSV 预期】使用标准 CSV 写入器编码 rows；返回无末尾行结束符的文本"""
    output = io.StringIO(newline="")
    csv.writer(output).writerows(rows)
    return output.getvalue().removesuffix("\r\n")


def table_formats(env, window, artifacts):
    """【结果测试】【表格操作】修改 window 中单元格并验证四种导出；env 为隔离环境，artifacts 保存结果"""
    session.click(window, 160, 56)
    session.click(window, 82, 155)
    session.key("ctrl+a")
    edited = 'A|B, "quoted"\n新值'
    paste(edited)
    rows = [row.copy() for row in ROWS]
    rows[1][0] = edited
    tsv = '\n'.join('\t'.join('"' + value.replace('"', '""') + '"' if any(char in value for char in '\t\r\n"') else value for value in row) for row in rows)
    session.copy_all(env, tsv)
    (artifacts / "table.tsv").write_text(tsv)
    for index, suffix in [(2, "csv"), (1, "md"), (3, "html")]:
        session.control("copy", "clipboard sentinel")
        session.click(window, window["size"][0] - 44, 56)
        session.click(window, window["size"][0] - 98, 92 + index * 30)
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline:
            text = session.read_clipboard(env)
            if text != "clipboard sentinel":
                break
            time.sleep(0.03)
        else:
            raise RuntimeError(f"copying {suffix} did not update the clipboard")
        (artifacts / f"table.{suffix}").write_text(text)
        if suffix == "csv" and text != expected_csv(rows):
            raise RuntimeError("CSV export differs from the standard CSV encoder")
        if suffix == "md" and not all(part in text for part in ('A\\|B, &quot;quoted&quot;<br>新值', '| --- | --- | --- |')):
            raise RuntimeError("Markdown lost escaping or the header separator")
        if suffix == "html" and not all(part in text for part in ('<table>', 'A|B, &quot;quoted&quot;<br>新值')):
            raise RuntimeError("HTML lost escaped cell text")
    session.screenshot(artifacts / "table.png")
    return tsv


def editable_result(binary, env, artifacts):
    """【结果测试】【编辑回归】验证 binary 的 Unicode 编辑、撤销、页签、窄窗口和表格；输出证据到 artifacts"""
    process = launch_result(binary, env, artifacts)
    try:
        window = session.size_window(session.wait_window(process.pid))
        session.copy_all(env, SOURCE)
        session.key("ctrl+a")
        edited = "Edited source\n中文内容"
        paste(edited)
        session.copy_all(env, edited)
        session.key("ctrl+z")
        session.copy_all(env, SOURCE)
        session.key("ctrl+shift+z")
        session.copy_all(env, edited)
        session.key("ctrl+a")
        session.key("ctrl+x")
        session.key("ctrl+z")
        session.copy_all(env, edited)
        session.key("ctrl+shift+z")
        session.key("ctrl+v")
        time.sleep(0.1)
        session.copy_all(env, edited)
        session.click(window, 98, 56)
        session.screenshot(artifacts / "image.png")
        session.click(window, window["size"][0] - 44, 56)
        copied_png = subprocess.check_output(["wl-paste", "--type", "image/png", "--no-newline"], env=env)
        if copied_png != (ROOT / "tests/fixtures/ocr-table.png").read_bytes():
            raise RuntimeError("image preview copied different source pixels")
        session.click(window, 36, 56)
        session.copy_all(env, edited)
        window = session.size_window(window, 360, 280)
        session.copy_all(env, edited)
        session.screenshot(artifacts / "narrow.png")
        window = session.size_window(window)
        tsv = table_formats(env, window, artifacts)
        session.click(window, 98, 56)
        session.click(window, window["size"][0] - 118, 56)
        session.click(window, 160, 56)
        session.copy_all(env, tsv)
        session.key("Escape")
        if process.wait(timeout=3) != 0:
            raise RuntimeError("the native result window exited with an error")
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=3)


def copy_models(models, destination):
    """【结果测试】【模型准备】将 models 中已验证资源复制到 destination；测试不重新下载模型"""
    for name in ("pp-ocrv5_mobile_det.onnx", "pp-ocrv5_mobile_rec.onnx", "ppocrv5_dict.txt"):
        shutil.copyfile(models / name, destination / name)


def actual_ocr(binary, models, base_env, artifacts, first_use=False, table_result=False):
    """【结果测试】【真实识别】用 binary、models 验证结果与释放；first_use 检查引导，table_result 检查表格，artifacts 保存证据"""
    artifacts.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="mark-shot-result-test-") as temporary:
        root = Path(temporary)
        config = root / "config/mark-shot"
        data = root / "data/mark-shot/models"
        config.mkdir(parents=True)
        data.mkdir(parents=True)
        if not first_use:
            copy_models(models, data)
        (config / "config.toml").write_text('[general]\nto = "s"\n[notifications]\nenabled = false\n')
        env = dict(base_env, XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
        if first_use:
            env.update(LANG="en_US.UTF-8", LC_ALL="en_US.UTF-8")
        with (artifacts / "ocr-process.log").open("w") as log:
            fixture = ROOT / "tests/fixtures" / ("ocr-table.png" if table_result else "ocr.png")
            process = subprocess.Popen([str(binary), "--ocr", str(fixture)], env=env, stdout=subprocess.DEVNULL, stderr=log)
        window = None
        try:
            if first_use:
                time.sleep(0.25)
                if list(data.iterdir()):
                    raise RuntimeError("first use downloaded a model without consent")
                session.screenshot(artifacts / "first-use.png")
                copy_models(models, data)
                with Image.open(artifacts / "first-use.png") as screenshot:
                    position = download_position(screenshot)
                session.control("click", "--logical", *position)
            window = session.wait_window()
            if window["pid"] == process.pid:
                raise RuntimeError("the OCR engine was not separated from the result window")
            if process.wait(timeout=3) != 0:
                raise RuntimeError("the recognition process failed during handoff")
            window = session.size_window(window)
            time.sleep(0.2)
            before = session.resources(window["pid"])
            time.sleep(1)
            after = session.resources(window["pid"])
            after["idle_cpu_ticks"] = after["cpu_ticks"] - before["cpu_ticks"]
            (artifacts / "resources.json").write_text(json.dumps(after, indent=2))
            if after["threads"] != 1 or after["idle_cpu_ticks"] > 2:
                raise RuntimeError(f"the idle result window retained workers or busy-polled: {after}")
            session.control("copy", "clipboard sentinel")
            session.key("ctrl+shift+c")
            text = session.read_clipboard(env)
            expected = ("Item", "Paper", "Ink") if table_result else ("Wayland Native Screenshot", "Mark Shot OCR 2026", "Local text recognition")
            for line in expected:
                if line not in text:
                    raise RuntimeError(f"the real result omitted {line!r}")
            (artifacts / "recognized.txt").write_text(text)
            if table_result:
                session.click(window, 160, 56)
                text = session.copy_all(env, SOURCE)
                (artifacts / "table.tsv").write_text(text)
            session.screenshot(artifacts / "recognized.png")
            session.key("Escape")
            time.sleep(0.1)
            if any(item["id"] == window["id"] for item in session.windows()):
                raise RuntimeError("the result window remained after closing")
            if session.read_clipboard(env) != text:
                raise RuntimeError("closing the result window discarded copied text")
            print(json.dumps(after))
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)
            if window and any(item["id"] == window["id"] for item in session.windows()):
                session.control("close", window["id"])
            application_log = root / "state/mark-shot/mark-shot.log"
            if application_log.exists():
                shutil.copyfile(application_log, artifacts / "application.log")


def main():
    """【结果测试】【入口】解析构建产物、模型和输出目录；执行编辑与真实识别两组回归"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--models", type=Path, required=True)
    parser.add_argument("--artifacts", type=Path, default=Path("target/wayland-ocr-result"))
    options = parser.parse_args()
    artifacts = options.artifacts.resolve()
    artifacts.mkdir(parents=True, exist_ok=True)
    env = session.environment()
    with tempfile.TemporaryDirectory(prefix="mark-shot-result-ui-") as temporary:
        test_env = dict(env, XDG_CONFIG_HOME=str(Path(temporary) / "config"), XDG_STATE_HOME=str(Path(temporary) / "state"))
        editable_result(options.binary.resolve(), test_env, artifacts)
    actual_ocr(options.binary.resolve(), options.models.resolve(), env, artifacts / "cached")
    actual_ocr(options.binary.resolve(), options.models.resolve(), env, artifacts / "first-use", first_use=True)
    actual_ocr(options.binary.resolve(), options.models.resolve(), env, artifacts / "table-ocr", table_result=True)
    print("Native OCR result regression passed")


if __name__ == "__main__":
    main()
