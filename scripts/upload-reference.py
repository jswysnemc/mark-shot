#!/usr/bin/env python3
"""【上传对照】【只读采集】完整 Qt 函数与内置辅助脚本对照，所有命令和文件均属于独立探针"""

import argparse
import hashlib
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import subprocess

def fingerprint(path):
    """【上传对照】【内容指纹】按 path 返回 SHA-256，不修改源文件"""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def function(text, signature):
    """【上传对照】【完整提取】从 text 返回 signature 对应完整函数，不改写生产代码，边界缺失直接失败"""
    start = text.index(signature)
    end = text.index("\n}\n", start) + 3
    return text[start:end]


def collect(original, root):
    """【上传对照】【生产采集】从 original 提取完整函数并在 root 独立执行，返回真实 Qt 与 helper 状态和指纹"""
    root.mkdir(parents=True, exist_ok=False)
    project = Path(__file__).resolve().parents[1]
    sources = {name: original / "qt" / name for name in (
        "src/shot_window_upload.cpp", "src/shot_window_config.cpp", "src/shot_window_internal.cpp",
        "src/shot_window_internal.h", "src/config_value.cpp", "src/shell_command.cpp", "scripts/mark-shot-upload")}
    before = {name: fingerprint(path) for name, path in sources.items()}
    harness = project / "tests/fixtures/qt-upload-reference/main.cpp"
    text = harness.read_text()
    internal = sources["src/shot_window_internal.cpp"].read_text()
    header = sources["src/shot_window_internal.h"].read_text()
    start = header.index("struct UploadConfig {")
    config_type = header[start:header.index("\n};", start) + 3]
    values = {
        "PRODUCTION_CONFIG_TYPE": config_type,
        "PRODUCTION_OBJECT_READER": function(sources["src/config_value.cpp"].read_text(), "QJsonObject firstObjectValue(const QJsonObject &object, const QStringList &keys)"),
        "PRODUCTION_CONFIG": function(sources["src/shot_window_config.cpp"].read_text(), "UploadConfig uploadConfig()"),
        "PRODUCTION_QUOTING": "\n".join(function(internal, signature) for signature in ("QString shellQuote(", "bool replaceExtensionImagePlaceholders(")),
        "PRODUCTION_UPLOAD": function(sources["src/shot_window_upload.cpp"].read_text(), "void ShotWindow::uploadSelection()"),
    }
    for name, value in values.items():
        text = text.replace(f"/* {name} */", value)
    main = root / "main.cpp"
    main.write_text(text)
    receiver = root / "receiver.py"
    receiver.write_text("""import json,pathlib,sys,time
kind=sys.argv[1]
assert pathlib.Path(sys.argv[2]).read_bytes()==b'isolated-upload-image'
if kind=='timeout': time.sleep(30)
if kind=='failed': raise SystemExit(4)
if kind=='plain': print('  https://images.example/plain  \\nignored')
elif kind=='plain-prefix': print('HTTPS://')
elif kind=='json-local': print(json.dumps({'url':'file:///tmp/returned-image.png'}))
elif kind=='empty': print(json.dumps({'url':''}))
else: print(json.dumps({'url':'https://images.example/json'}))
""")
    flags = subprocess.check_output(["pkg-config", "--cflags", "--libs", "Qt6Widgets"], text=True).split()
    binary = root / "reference"
    with (root / "build.log").open("w") as log:
        subprocess.run(["c++", "-std=c++17", "-fPIC", str(main), str(sources["src/shell_command.cpp"]), "-I" + str(original / "qt/src"), *flags, "-o", str(binary)], stdout=log, stderr=log, check=True)
    environment = dict(os.environ, QT_QPA_PLATFORM="offscreen", SHELL="/bin/sh")
    for key in ("TOKEN", "A", "MARK_SHOT_UPLOAD_COMMAND", "MARK_SHOT_UPLOAD_TIMEOUT_MS"):
        environment.pop(key, None)
    result = subprocess.run([str(binary), str(receiver), str(root / "reference.json")], env=environment, capture_output=True, text=True, check=True)
    (root / "run.log").write_text(result.stderr)
    state = json.loads((root / "reference.json").read_text())
    loader = importlib.machinery.SourceFileLoader("isolated_upload_helper", str(sources["scripts/mark-shot-upload"]))
    module = importlib.util.module_from_spec(importlib.util.spec_from_loader(loader.name, loader))
    loader.exec_module(module)
    paths = [("data.0.url", {"data": [{"url": "https://example.test/array"}]}),
             ("data.-1.url", {"data": [{"url": "https://example.test/array"}]}),
             ("data.8.url", {"data": []}), ("data.value", {"data": {"value": 7}}),
             ("url", {"url": "https://example.test/top"}), ("", {"url": "https://example.test/top"})]
    state["extract"] = [{"path": path, "input": value, "output": module.extract_url(value, path)} for path, value in paths]
    fields = [{}, {"album": "local", "token": "test-value"}, {"label": "中文\nvalue"}]
    state["multipart"] = []
    for values in fields:
        body, boundary = module.build_multipart(values, {"image": ("screenshot.png", bytes([1, 2, 3]), "image/png")})
        state["multipart"].append({"fields": values, "boundary": boundary, "body": body.hex()})
    if len(state["configs"]) != 12 or len(state["results"]) != 8:
        raise AssertionError("original upload matrix is incomplete")
    if before != {name: fingerprint(path) for name, path in sources.items()}:
        raise AssertionError("original sources changed during collection")
    (root / "reference.json").write_text(json.dumps(state, ensure_ascii=False, indent=2) + "\n")
    index = {"sources": before, "harness": fingerprint(harness), "generated_source": fingerprint(main),
             "binary": fingerprint(binary), "reference": fingerprint(root / "reference.json"), "qt": state["qt"]}
    (root / "index.json").write_text(json.dumps(index, indent=2) + "\n")
    return index


def main():
    """【上传对照】【命令入口】解析 original、root 并执行只读采集，返回验证索引"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--original", type=Path, default=Path.home() / "workspace/mark-shot")
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(collect(options.original.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
