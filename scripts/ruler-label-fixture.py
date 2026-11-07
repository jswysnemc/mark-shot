#!/usr/bin/env python3
"""【尺子说明对照】【固定样本】校验完整采集指纹并选取普通回归所需场景"""

import argparse
import hashlib
import json
from pathlib import Path


def digest(path):
    """【尺子说明对照】【文件指纹】按path返回文件SHA-256，无修改"""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def extract(root, destination):
    """【尺子说明对照】【样本提取】按root校验可靠采集并写入destination，返回场景数量"""
    index = json.loads((root / "index.json").read_text())
    reference = root / "reference.json"
    assert digest(reference) == index["reference"]
    values = json.loads(reference.read_text())
    assert len(values["scenes"]) == index["scenes"] == 2880
    # 1. 【尺子说明对照】【覆盖选择】每组倍率、截图比例和笔宽保留两个绘制目标，轮换上下文与几何组合
    serials = {0, 7, 388, 389}
    for group in range(48):
        first = group * 60 + (group % 3) * 20 + (group % 10) * 2
        serials.update((first, first + 1))
    scenes = [values["scenes"][serial] for serial in sorted(serials)]
    result = {"provenance": index, "scenes": scenes}
    destination.write_text(
        '{"provenance":' + json.dumps(result["provenance"], separators=(",", ":"))
        + ',"scenes":[\n' + ",\n".join(json.dumps(scene, separators=(",", ":")) for scene in scenes) + "\n]}\n"
    )
    return len(scenes)


def main():
    """【尺子说明对照】【入口】解析采集目录与固定样本路径并执行提取，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--destination", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps({"scenes": extract(args.root, args.destination), "fixture": digest(args.destination)}))


if __name__ == "__main__":
    main()
