#!/usr/bin/env python3
"""【Windows识别验证】【真实模型】在私有Wine环境以中英文样本验证Paddle识别结果与原图坐标"""

import argparse
import hashlib
import io
import json
from pathlib import Path
import shutil
import struct
import subprocess
import time
import zipfile

from PIL import Image, ImageDraw, ImageFont
from windows_ocr_actions import path_w
from windows_test_runtime import run

ASSETS = {
    "onnxruntime.dll": "18370c375f07357fa5874344a9d9ac17e6b6fe1eb18b1dd209d79483b4470257",
    "onnxruntime_providers_shared.dll": "599629fa643707defe9156140ae5edd73531f221aa97b7585b1c9bb0a93586f8",
}


def frame(tag, body):
    """【Windows识别验证】【消息帧】按tag与body返回真实worker协议帧，不改变正文"""
    return struct.pack("<IB", len(body) + 1, tag) + body


def string(value):
    """【Windows识别验证】【协议字符串】按value返回UTF-8长度及正文，供Windows路径交接"""
    data = value.encode()
    return struct.pack("<I", len(data)) + data


def lines(data):
    """【Windows识别验证】【结果读取】按data解析有界正文帧，返回行文本及原图矩形，错误结果直接失败"""
    stream = io.BytesIO(data)
    length, tag = struct.unpack("<IB", stream.read(5))
    assert len(data) == length + 4, "incomplete recognition response"
    def read_string():
        """【Windows识别验证】【正文字段】无参数，返回本帧下一字符串"""
        count, = struct.unpack("<I", stream.read(4))
        return stream.read(count).decode()
    if tag == 5:
        raise AssertionError(read_string())
    assert tag == 8, "unexpected recognition response"
    count, = struct.unpack("<I", stream.read(4))
    result = []
    for _ in range(count):
        text = read_string()
        bounds = struct.unpack("<ffff", stream.read(16))
        chars, = struct.unpack("<I", stream.read(4))
        stream.read(chars * 4)
        result.append({"text": text, "bounds": bounds})
    assert stream.tell() == len(data)
    return result


def prepare(root, archive, models):
    """【Windows识别验证】【资源准备】按root建立私有用户资源，校验archive官方摘要后复制模型，返回模型目录"""
    assert hashlib.sha256(archive.read_bytes()).hexdigest() == "abef733dacbe2f571547a7150b479b5cb9cc0df22f96c24983a42cadb1b4f8bc"
    users = [path for path in (root / "wine-prefix/drive_c/users").glob("*/AppData/Roaming")
             if path.parent.parent.name.lower() not in ("default", "public")]
    assert len(users) == 1, "ambiguous private Windows user"
    data = users[0] / "mark-shot"
    installed = data / "runtime/onnxruntime-1.28.0-win-x64/installed"
    installed.mkdir(parents=True)
    with zipfile.ZipFile(archive) as source:
        for name, expected in ASSETS.items():
            matches = [item for item in source.namelist() if item.endswith("/" + name)]
            assert len(matches) == 1
            content = source.read(matches[0])
            assert hashlib.sha256(content).hexdigest() == expected
            (installed / name).write_bytes(content)
    destination = data / "models"
    destination.mkdir()
    for name in ("pp-ocrv5_mobile_det.onnx", "pp-ocrv5_mobile_rec.onnx", "ppocrv5_dict.txt"):
        shutil.copyfile(models / name, destination / name)
    return destination


def execute(command, environment, root, log, archive, models):
    """【Windows识别验证】【样本执行】按候选command、私有environment及资源识别中英文小字，返回状态并保存逐字断言与耗时"""
    destination = prepare(root, archive, models)
    model_paths = [path_w(destination / name, environment) for name in
                   ("pp-ocrv5_mobile_det.onnx", "pp-ocrv5_mobile_rec.onnx", "ppocrv5_dict.txt")]
    load = frame(6, b"".join(string(path) for path in model_paths))
    font_path = subprocess.check_output(["fc-match", "-f", "%{file}", "Noto Sans CJK SC"], text=True)
    report = {"real_windows_validation": False, "cases": [], "passed": False}
    for size in (24, 16, 12):
        expected = ["截图文字识别", "Mark Shot OCR 2026", "本地模型离线识别"]
        image = Image.new("RGB", (480, size * 6 + 40), "white")
        draw = ImageDraw.Draw(image)
        font = ImageFont.truetype(font_path, size)
        for index, text in enumerate(expected):
            draw.text((16, 12 + index * size * 2), text, font=font, fill="black")
        image.save(root / f"fixture-{size}.png")
        recognize = frame(7, struct.pack("<IIff", image.width, image.height, 16.0, 24.0) + image.tobytes())
        started = time.monotonic()
        result = subprocess.run(["wine", command[0], "--pin-recognition-worker"], input=load + recognize,
                                env=environment, cwd=root, stdout=subprocess.PIPE, stderr=log, timeout=60, check=True)
        actual = lines(result.stdout)
        case = {"font_size": size, "elapsed_ms": round((time.monotonic() - started) * 1000, 2), "lines": actual}
        report["cases"].append(case)
        (root / "ocr-fixtures.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
        assert [line["text"] for line in actual] == expected, f"text mismatch for {size}px: {actual}"
        assert all(16 <= item["bounds"][0] < item["bounds"][2] <= image.width + 16
                   and 24 <= item["bounds"][1] < item["bounds"][3] <= image.height + 24 for item in actual)
    report["passed"] = True
    (root / "ocr-fixtures.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return subprocess.CompletedProcess(command, 0)


def main():
    """【Windows识别验证】【私有入口】无参数，从命令行读取候选、资源与新目录，执行验证并返回状态"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--models", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    result = run(options.root.resolve(), [str(options.binary.resolve())],
                 execute=lambda *args: execute(*args, options.archive.resolve(), options.models.resolve()))
    print(json.dumps(result, indent=2))
    raise SystemExit(result["exit_code"])


if __name__ == "__main__":
    main()
