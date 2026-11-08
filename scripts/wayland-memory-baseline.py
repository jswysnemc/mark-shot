#!/usr/bin/env python3
"""【内存对照】【原生流程】在私有输出执行相同 Qt/Rust 场景并保存不可变指纹和失败证据"""

import argparse
from contextlib import contextmanager
import hashlib
import importlib.util
import json
import os
import re
from pathlib import Path
import subprocess
import time

from PIL import ImageChops
import wayland_text_format_session as session
from wayland_text_pointer import pointer_context
from wayland_memory_metrics import MemorySampler
from wayland_scroll_raw_capture import capture as raw_capture

PROJECT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("memory_scroll_support", PROJECT / "scripts/wayland-scroll-capture.py")
SCROLL = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SCROLL)


@contextmanager
def launch(binary, implementation, root, flags, input_bytes=None):
    """【内存对照】【启动】按 binary、implementation、root 和 flags 创建隔离进程，可选 input_bytes 提供快照；返回进程与日志"""
    root.mkdir(mode=0o700)
    environment = dict(session.environment(), XDG_CONFIG_HOME=str(root / "config"),
                       XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"),
                       MARK_SHOT_CONFIG_DIR=str(root / "config"), MARK_SHOT_LANG="english",
                       WAYLAND_DEBUG="client", QT_QPA_PLATFORM="wayland", QT_NO_XDG_DESKTOP_PORTAL="1",
                       DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/memory-test")
    if implementation == "rust":
        config = root / "config.toml"
        config.write_text('[general]\noverlay="layer"\nto=""\n[history]\nenabled=false\n[notifications]\nenabled=false\n[region_detection]\nenabled=false\n[window_detection]\nenabled=false\n')
        command = [str(binary), "--config", str(config), "--nonotify", *flags]
    else:
        config = root / "config"
        config.mkdir()
        (config / "config.json").write_text(json.dumps({"captureHistory": {"enabled": False},
                                                       "windowDetection": {"enabled": False},
                                                       "regionDetection": {"enabled": False},
                                                       "ui": {"language": "english", "theme": "light"},
                                                       "shortcuts": {"actions": {"scroll": "Ctrl+Shift+S"}}}))
        command = [str(binary), "--capture", "--no-debug", *flags]
    path = root / "client.log"
    with path.open("w") as log, (root / "output.png").open("wb") as output:
        process = subprocess.Popen(command, env=environment, stdout=output, stderr=log,
                                   stdin=subprocess.PIPE if input_bytes is not None else subprocess.DEVNULL)
        try:
            if input_bytes is not None:
                process.stdin.write(input_bytes)
                process.stdin.close()
            yield process, path
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=3)


def presented(process, log, implementation="rust", size=(3840, 2160)):
    """【内存对照】【首帧】等待process对应implementation及size的真实主窗口和键盘焦点，log保留协议，返回实际首帧秒数，排除探测表面"""
    started = time.monotonic()

    def ready():
        """【内存对照】【协议条件】检查本次进程完整表面附着与键盘进入，无参数，返回是否满足"""
        assert process.poll() is None, log.read_text()[-2000:]
        text = log.read_text()
        return initial_frame_ready(text, implementation, size)

    while not ready():
        assert time.monotonic() - started < 30, "the measured application did not present its actual main frame"
        time.sleep(0.04)
    elapsed = time.monotonic() - started
    time.sleep(0.3)
    return elapsed


def stable(sampler, phase, seconds=1.2):
    """【内存对照】【稳定采样】按 sampler 标记 phase 并等待 seconds，不修改应用状态，无返回值"""
    sampler.mark(phase)
    time.sleep(seconds)
    sampler.mappings()


def result_frame_ready(text):
    """【内存对照】【结果首帧】按 text 确认普通编辑窗口或旧版全屏编辑表面提交真实前景，返回是否完成"""
    if image_editor_ready(text):
        return True
    parents = re.findall(r'get_layer_surface\([^\n]*wl_surface@(\d+)[^\n]*Some\("(?:mark-shot|utility)"\)', text)
    buffers = set(re.findall(r'create_buffer\(wl_buffer@(\d+),\s*\d+,\s*3840,\s*2160,\s*15360,', text))
    for parent in parents:
        attachments = list(re.finditer(rf'wl_surface@{parent}\.attach\(wl_buffer@(\d+),', text))
        complete = [item for item in attachments if item.group(1) in buffers]
        if len(complete) >= 2 and f'wl_surface@{parent}.commit()' in text[complete[-1].end():]:
            return True
    return False


def image_editor_ready(text):
    """【内存对照】【图片窗口】按 text 追踪编辑窗口角色与表面，返回是否已提交内容并取得键盘焦点"""
    for role in re.findall(r'xdg_toplevel@(\d+)\.set_app_id\(Some\("mark-shot-editor"\)\)', text):
        shell = re.search(rf'xdg_surface@(\d+)\.get_toplevel\(xdg_toplevel@{role}\)', text)
        if not shell:
            continue
        surface = re.search(rf'get_xdg_surface\(xdg_surface@{shell.group(1)}, wl_surface@(\d+)\)', text)
        if not surface:
            continue
        parent = surface.group(1)
        attached = list(re.finditer(rf'wl_surface@{parent}\.attach\(wl_buffer@\d+,', text))
        entered = re.search(rf'wl_keyboard@\d+\.enter(?:,\s*|)\([^\n]*wl_surface@{parent},', text)
        if len(attached) >= 2 and entered and f'wl_surface@{parent}.commit()' in text[attached[-1].end():]:
            return True
    return False


def initial_frame_ready(text, implementation, size=(3840, 2160)):
    """【内存对照】【主表面条件】按text、implementation和size要求真实前景、提交及同表面键盘焦点，返回是否满足，探测表面不参与"""
    if implementation == "rust" and image_editor_ready(text):
        return True
    buffers = set(re.findall(rf'create_buffer\((?:new id )?wl_buffer([@#]\d+),\s*\d+,\s*{size[0]},\s*{size[1]},\s*{size[0] * 4},', text))
    parents = set(re.findall(r'wl_surface([@#]\d+)\.attach\(wl_buffer[@#]\d+,', text))
    if implementation == "rust":
        roles = set("@" + parent for parent in re.findall(r'get_layer_surface\([^\n]*wl_surface@(\d+)[^\n]*Some\("(?:mark-shot|utility)"\)', text))
        parents &= roles
    for parent in parents:
        attached = [item for item in re.finditer(rf'wl_surface{re.escape(parent)}\.attach\(wl_buffer([@#]\d+),', text)
                    if item.group(1) in buffers]
        entered = re.search(rf'wl_keyboard[@#]\d+\.enter(?:,\s*|)\([^\n]*wl_surface{re.escape(parent)},', text)
        if len(attached) >= 3 and entered and f'wl_surface{parent}.commit()' in text[attached[-1].end():]:
            return True
    return False


def panel_controls(image, implementation):
    """【内存对照】【操作位置】从 image 右侧实际保存按钮定位 implementation 控制栏，返回暂停和标注中心"""
    strip = image.crop((3280, 0, 3840, 2160))
    if implementation == "rust":
        controls = SCROLL.controls(strip)
        return {key: (point[0] + 3280, point[1]) for key, point in controls.items()}
    red, green, blue = strip.split()
    mask = ImageChops.multiply(red.point(lambda value: 255 if value == 215 else 0),
                              green.point(lambda value: 255 if value == 236 else 0))
    mask = ImageChops.multiply(mask, blue.point(lambda value: 255 if value == 235 else 0))
    bounds = mask.getbbox()
    assert bounds and bounds[2] - bounds[0] > 20, "the Qt scroll panel has no visible save control"
    x, y = 3280 + (bounds[0] + bounds[2]) / 2, (bounds[1] + bounds[3]) / 2
    return {"annotate": (x - 51, y), "pause": (x - 102, y)}


def ordinary(binary, implementation, root, reference):
    """【内存对照】【普通截图】用 binary 执行 implementation 全屏编辑，root 保存样本；返回内存与 reference 原图核对结果"""
    with launch(binary, implementation, root, ["--fullscreen"]) as (process, log):
        with MemorySampler(process, root / "samples.jsonl") as sampler:
            main_frame_seconds = presented(process, log, implementation)
            stable(sampler, "editor-stable")
            shown = raw_capture()
            shown.save(root / "editor.png")
            bounds = (400, 400, reference.width - 400, reference.height - 400)
            assert ImageChops.difference(reference.crop(bounds), shown.crop(bounds)).getbbox() is None, "ordinary editor changed source pixels"
            sampler.mark("cancel")
            session.key("Escape")
            assert process.wait(timeout=5) == 0, log.read_text()[-2000:]
            stable(sampler, "exited", 0.25)
        result = sampler.report()
        assert not result["remaining_processes"], "ordinary screenshot left measured processes after cancellation"
    restored = raw_capture()
    restored.save(root / "restored.png")
    assert ImageChops.difference(reference, restored).getbbox() is None, "ordinary cancel did not restore the page"
    result.update(source_region_equal=True, cancel_restores_page=True, exit_code=process.returncode,
                  main_frame_seconds=main_frame_seconds)
    return result


def scroll_capture(binary, implementation, root, reference, trace, steps, assert_shm=False):
    """【内存对照】【长截图】按两版入口截取相同区域，trace 核对 steps 次滚动，assert_shm 检查共享预算；root 保存证据，返回统计"""
    region = (80, 80, 3280, 1880)
    flags = ["-S"] if implementation == "rust" else []
    with launch(binary, implementation, root, flags) as (process, log):
        with MemorySampler(process, root / "samples.jsonl") as sampler:
            main_frame_seconds = presented(process, log, implementation)
            SCROLL.INPUT.drag(region[:2], region[2:])
            if implementation == "qt":
                session.key("ctrl+shift+s")
            time.sleep(0.7)
            stable(sampler, "scroll-first")
            SCROLL.INPUT.move((400, 400))
            pointer = SCROLL.INPUT.CURRENT.get()[0]
            sampler.mark("scroll-growth")
            for index in range(steps):
                assert process.poll() is None, log.read_text()[-2000:]
                pointer.scroll(dy=10.0)
                SCROLL.wait(lambda: SCROLL.origin(trace) == (0, (index + 1) * 48), "the measured scroll page lost input")
                time.sleep(0.18)
            stable(sampler, "scroll-stable")
            shown = raw_capture()
            shown.save(root / "scroll.png")
            controls = panel_controls(shown, implementation)
            sampler.mark("annotate-handoff")
            offset = len(log.read_text())
            SCROLL.INPUT.click(controls["annotate"])
            if implementation == "qt":
                node = SCROLL.wait(lambda: next((node for node in SCROLL.tree(session.environment())
                                               if node.get("pid") == process.pid and node.get("app_id")), None),
                                   "the Qt scroll result editor did not map")
                SCROLL.command(session.environment(), f'[con_id="{node["id"]}"] focus')
                SCROLL.command(session.environment(), f'[con_id="{node["id"]}"] fullscreen enable')
            else:
                SCROLL.wait(lambda: result_frame_ready(log.read_text()[offset:]), "the Rust result editor did not submit its actual frame")
            time.sleep(0.7)
            stable(sampler, "result-editor-stable")
            raw_capture().save(root / "result-editor.png")
            sampler.mark("cancel")
            session.key("Escape")
            assert process.wait(timeout=5) == 0, log.read_text()[-2000:]
            stable(sampler, "exited", 0.25)
        result = sampler.report()
        assert not result["remaining_processes"], "scroll screenshot left measured processes after cancellation"
        if implementation == "rust":
            mappings = json.loads((root / "mappings-scroll-first.json").read_text())
            shared = sum(mapping["size_kib"] * 1024 for mapping in mappings
                         if "/memfd:smithay-client-toolkit" in mapping["path"])
            result["scroll_shm_bytes"] = shared
            result["scroll_shm_within_foreground_budget"] = shared <= 3840 * 2160 * 4 + 256 * 1024
            result["scroll_shm_within_panel_budget"] = 0 < shared <= 4 * 1024 * 1024
            if assert_shm:
                assert result["scroll_shm_within_panel_budget"], "scroll controls still reserve a full-screen buffer"
            sources = result["original_source_files"].get("result-editor-stable", [])
            result["original_source_disk_bytes"] = sum(source["bytes"] for source in sources if source["kind"] == "original")
            result["frozen_source_disk_bytes"] = sum(source["bytes"] for source in sources if source["kind"] == "frozen")
            result["original_source_file_resident_kib"] = sum(source["resident_file_kib"] for source in sources if source["kind"] == "original")
            result["frozen_source_file_resident_kib"] = sum(source["resident_file_kib"] for source in sources if source["kind"] == "frozen")
            result["original_source_resident_kib"] = sum(mapping["rss_kib"] for mapping in json.loads((root / "mappings-result-editor-stable.json").read_text())
                                                        if "/mark-shot/image-source/" in mapping["path"])
    result.update(page_origin=list(SCROLL.origin(trace)), expected_size=[3200, 1800 + steps * 48], exit_code=process.returncode,
                  main_frame_seconds=main_frame_seconds)
    SCROLL.INPUT.click((400, 400))
    session.key("Home")
    time.sleep(0.25)
    restored = raw_capture()
    restored.save(root / "restored.png")
    assert ImageChops.difference(reference, restored).getbbox() is None, "scroll cancel did not restore the page"
    result["cancel_restores_page"] = True
    return result


def run(options):
    """【内存对照】【入口】按 options 在已校验的私有 4K 输出运行两版；返回阶段对照和验收结论，保留失败现场"""
    root = options.root.resolve()
    root.mkdir(parents=True, exist_ok=False)
    metadata = json.loads((Path(os.environ["XDG_RUNTIME_DIR"]) / "runtime.json").read_text())
    output = metadata["outputs"]["HEADLESS-1"]
    assert output["logical"]["width"] == 3840 and output["logical"]["height"] == 2160 and output["logical"]["scale"] == 1
    report = {"output": output, "scenario": options.scenario, "applications": {}}
    with session.input_context(), pointer_context():
        session.sway.keyboard_ready()
        with SCROLL.page(options.page.resolve(), session.environment(), root) as (_page, trace):
            reference = raw_capture()
            reference.save(root / "source.png")
            for implementation, binary in (("qt", options.qt.resolve()), ("rust", options.rust.resolve())):
                fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
                if options.scenario == "4k-fullscreen-editor":
                    result = ordinary(binary, implementation, root / implementation, reference)
                else:
                    result = scroll_capture(binary, implementation, root / implementation, reference, trace, options.steps, options.assert_scroll_shm)
                assert fingerprint == hashlib.sha256(binary.read_bytes()).hexdigest(), "measured binary changed"
                result.update(binary=str(binary), binary_sha256=fingerprint, binary_bytes=binary.stat().st_size)
                report["applications"][implementation] = result
                (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    qt, rust = (report["applications"][name] for name in ("qt", "rust"))
    comparisons = {}
    for key in ("peak_rss_kib", "peak_pss_kib", "main_hwm_kib",
                "peak_rss_with_unmapped_original_kib", "peak_pss_with_unmapped_original_kib"):
        comparisons[key] = {"qt": qt[key], "rust": rust[key], "ratio": rust[key] / qt[key], "below_half": rust[key] * 2 < qt[key]}
    phases = ["editor-stable"] if options.scenario == "4k-fullscreen-editor" else ["scroll-first", "scroll-stable", "result-editor-stable"]
    for phase in phases:
        for key in ("median_rss_kib", "median_pss_kib",
                    "median_rss_with_unmapped_original_kib", "median_pss_with_unmapped_original_kib"):
            first, second = (app["phases"][phase][key] for app in (qt, rust))
            comparisons[f"{phase}.{key}"] = {"qt": first, "rust": second, "ratio": second / first, "below_half": second * 2 < first}
    report.update(comparisons=comparisons, passed=all(value["below_half"] for value in comparisons.values()))
    (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def main():
    """【内存对照】【命令行】解析两版二进制、私有页面和新目录，无参数，返回严格内存验收退出状态"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--qt", type=Path, required=True)
    parser.add_argument("--rust", type=Path, required=True)
    parser.add_argument("--page", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--scenario", choices=("4k-fullscreen-editor", "4k-scroll"), default="4k-fullscreen-editor")
    parser.add_argument("--steps", type=int, default=128)
    parser.add_argument("--assert-scroll-shm", action="store_true")
    options = parser.parse_args()
    report = run(options)
    print(json.dumps({"scenario": report["scenario"], "comparisons": report["comparisons"], "passed": report["passed"]}, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
