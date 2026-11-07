#!/usr/bin/env python3
"""【工具内存对照】【原生矩阵】私有4K输出中执行两版同一工具手势，保留稳定占用、峰值与退出资源"""

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import time

import wayland_text_format_session as session
from wayland_memory_metrics import MemorySampler
from wayland_memory_tool_actions import CASES, QT_SHORTCUTS, SHORTCUTS, UNMATCHED_QT_TOOLS, draw
from wayland_text_pointer import pointer_context


SPEC = importlib.util.spec_from_file_location("memory_tools_support", Path(__file__).with_name("wayland-memory-baseline.py"))
SUPPORT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SUPPORT)


def tool_case(binary, implementation, root, tool):
    """【工具内存对照】【单个工具】按binary和implementation选择tool，root保存真实采样与画面，返回资源统计和正文变化，无降低质量的测试选项"""
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    flags = ["--fullscreen"]
    with SUPPORT.launch(binary, implementation, root, flags) as (process, log):
        with MemorySampler(process, root / "samples.jsonl") as sampler:
            main_frame_seconds = SUPPORT.presented(process, log, implementation)
            # 1. 【工具内存对照】【明确工具】按各版真实快捷键激活同一工具，不能把默认工具或无对应工具的手势当作比较样本
            shortcut = (QT_SHORTCUTS if implementation == "qt" else SHORTCUTS)[tool]
            session.key(shortcut)
            time.sleep(0.25)
            SUPPORT.stable(sampler, "tool-ready", 0.7)
            before = SUPPORT.raw_capture()
            before.save(root / "before.png")
            sampler.mark("draw")
            drawing = draw(SUPPORT, before, root / "drawn.png", tool)
            SUPPORT.stable(sampler, "annotation-stable", 1.2)
            sampler.mark("cancel")
            session.key("Escape")
            assert process.wait(timeout=5) == 0, log.read_text()[-2000:]
            SUPPORT.stable(sampler, "exited", 0.15)
        result = sampler.report()
    assert not result["remaining_processes"], "the tool left measured processes after cancellation"
    assert fingerprint == hashlib.sha256(binary.read_bytes()).hexdigest(), "measured candidate changed"
    result.update(binary_sha256=fingerprint, binary_bytes=binary.stat().st_size, main_frame_seconds=main_frame_seconds,
                  drawing=drawing, exit_code=process.returncode, tool_shortcut=shortcut)
    return result


def compare(applications):
    """【工具内存对照】【一半门槛】按applications检查稳定RSS、PSS、文件缓存和观测及内核峰值，返回逐项结论"""
    qt, rust = (applications[name] for name in ("qt", "rust"))
    result = {}
    for key in ("peak_rss_kib", "peak_pss_kib", "main_hwm_kib", "peak_rss_with_unmapped_original_kib", "peak_pss_with_unmapped_original_kib"):
        result[key] = {"qt": qt[key], "rust": rust[key], "below_half": rust[key] * 2 < qt[key]}
    for phase in ("tool-ready", "annotation-stable"):
        for key in ("median_rss_kib", "median_pss_kib", "median_rss_with_unmapped_original_kib", "median_pss_with_unmapped_original_kib"):
            first, second = (app["phases"][phase][key] for app in (qt, rust))
            result[f"{phase}.{key}"] = {"qt": first, "rust": second, "below_half": second * 2 < first}
    return result


def run(qt, rust, page, root, tools):
    """【工具内存对照】【同场景入口】按qt、rust、page在私有输出执行tools，root保存独立失败证据，返回完整报告，不以单个工具代表全部工具"""
    root.mkdir(parents=True, exist_ok=False)
    report = {"status": "incomplete", "tools": [], "unmatched_qt_tools": list(UNMATCHED_QT_TOOLS)}
    with session.input_context(), pointer_context():
        session.sway.keyboard_ready()
        with SUPPORT.SCROLL.page(page, session.environment(), root) as (_page, _trace):
            for tool in tools:
                case_root = root / tool
                case_root.mkdir()
                applications = {}
                for implementation, binary in (("qt", qt), ("rust", rust)):
                    applications[implementation] = tool_case(binary, implementation, case_root / implementation, tool)
                comparisons = compare(applications)
                case = {"tool": tool, "applications": applications, "comparisons": comparisons,
                        "passed": all(item["below_half"] for item in comparisons.values())}
                report["tools"].append(case)
                (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
                print(json.dumps({"tool": tool, "passed": case["passed"], "comparisons": comparisons}), flush=True)
    report["passed"] = all(case["passed"] for case in report["tools"])
    report["status"] = "complete"
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    """【工具内存对照】【命令入口】解析两版二进制、页面、私有输出和工具集合，返回严格矩阵验收状态，无参数"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--qt", type=Path, required=True)
    parser.add_argument("--rust", type=Path, required=True)
    parser.add_argument("--page", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--tools", nargs="+", choices=CASES, default=list(CASES))
    options = parser.parse_args()
    report = run(options.qt.resolve(), options.rust.resolve(), options.page.resolve(), options.root.resolve(), options.tools)
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
