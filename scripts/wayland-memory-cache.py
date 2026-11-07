#!/usr/bin/env python3
"""【内存对照】【原件写入】直接打开已验证大图，复现原件文件写入峰值与退出清理"""

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path

from PIL import Image
import wayland_text_format_session as session
from wayland_memory_metrics import MemorySampler


SPEC = importlib.util.spec_from_file_location("memory_cache_support", Path(__file__).with_name("wayland-memory-baseline.py"))
SUPPORT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SUPPORT)


def run(binary, image, root, limit_kib):
    """【内存对照】【原件入口】按binary打开image，root保存连续样本，limit_kib约束文件缓存，返回完整报告，不生成期望像素"""
    root.mkdir(parents=True, exist_ok=False)
    with Image.open(image) as source:
        expected_bytes = source.width * source.height * 4
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    with session.input_context():
        session.sway.keyboard_ready()
        with SUPPORT.launch(binary, "rust", root / "editor", [str(image)]) as (process, log):
            with MemorySampler(process, root / "editor/samples.jsonl", interval=0.005) as sampler:
                SUPPORT.presented(process, log)
                SUPPORT.stable(sampler, "editor-stable", 0.25)
                sources = sampler.source_files["editor-stable"]
                assert sum(source["bytes"] for source in sources if source["kind"] == "original") == expected_bytes, "the original was not cached"
                SUPPORT.raw_capture().save(root / "editor.png")
                sampler.mark("cancel")
                session.key("Escape")
                assert process.wait(timeout=5) == 0, log.read_text()[-2000:]
                SUPPORT.stable(sampler, "exited", 0.1)
            result = sampler.report()
    assert not result["remaining_processes"], "image editing left measured processes"
    assert result["original_source_files"]["exited"] == [], "image editing retained original descriptors"
    assert fingerprint == hashlib.sha256(binary.read_bytes()).hexdigest(), "measured candidate changed"
    peak = result["peak_original_file_resident_kib"]
    result.update(binary_sha256=fingerprint, expected_original_bytes=expected_bytes,
                  original_file_cache_limit_kib=limit_kib, original_file_cache_peak_kib=peak,
                  passed=peak <= limit_kib)
    (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main():
    """【内存对照】【命令入口】解析固定二进制、原图、私有输出和文件缓存预算，返回严格写入峰值验收状态，无参数"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--image", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--limit-kib", type=int, default=8192)
    options = parser.parse_args()
    result = run(options.binary.resolve(), options.image.resolve(), options.root.resolve(), options.limit_kib)
    print(json.dumps({key: result[key] for key in ("binary_sha256", "original_file_cache_peak_kib", "original_file_cache_limit_kib", "passed")}, indent=2))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
