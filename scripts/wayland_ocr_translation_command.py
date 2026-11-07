#!/usr/bin/env python3
"""【OCR 翻译回归】【本地命令】记录实际输入和所属进程，仅提供可控成功、失败及阻塞结果，不访问网络"""

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def execute(root, input_path, target):
    """【OCR 翻译回归】【任务执行】按 root 模式处理 input_path 与 target，返回退出状态；先原子记录资源，再等待或返回结果"""
    mode = (root / "mode").read_text().strip()
    source = json.loads(input_path.read_text())
    child = subprocess.Popen(["/bin/sleep", "60"]) if mode == "hold" else None
    record = {"pid": os.getpid(), "child": child.pid if child else None,
              "input": str(input_path), "source": source, "target": target, "mode": mode}
    requests = root / "requests"
    requests.mkdir(exist_ok=True)
    name = str(time.time_ns())
    temporary = requests / (name + ".tmp")
    temporary.write_text(json.dumps(record))
    temporary.replace(requests / (name + ".json"))
    if mode in ("hold", "wait"):
        time.sleep(60)
    elif mode == "error":
        sys.stderr.write("private translation fixture failure")
        return 7
    else:
        time.sleep(0.15)
    text = "\n".join(token["text"] for token in source["tokens"])
    sys.stdout.write(json.dumps({"tokens": [{"text": "translated " + text}]}))
    return 0


def main():
    """【OCR 翻译回归】【命令入口】解析私有 root、input 与 target 后执行，返回命令状态，无参数"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--target", required=True)
    args = parser.parse_args()
    return execute(args.root, args.input, args.target)


if __name__ == "__main__":
    raise SystemExit(main())
