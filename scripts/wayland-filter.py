#!/usr/bin/env python3
"""【Wayland 测试】【原图滤镜】在私有桌面验证五模式、真实属性输入、几何历史、偏好和独立像素导出"""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib

from wayland_filter_checks import export, idle, ready
from wayland_filter_fixture import geometry, read_commits, source_image
from wayland_filter_gestures import cancel, controls, deletion, empty_creation
from wayland_filter_reference import MODES
from wayland_filter_scenarios import create, future_default, large_region, matrix, panel_lifecycle, property_history
import wayland_test_session as session


def launch(binary, environment, destination, source_path, log):
    """【滤镜测试】【应用启动】以 binary、environment 打开 source_path；destination 指定保存目录，log 接收标准错误，返回子进程"""
    return subprocess.Popen([str(binary), "--fullscreen", "--fullscreen-default-tool", "redact", "--to", "cs", "--output", str(destination), str(source_path)], env=environment, stdout=subprocess.DEVNULL, stderr=log)


def persistence(binary, environment, temporary, artifacts, source_path, source, layout, application_log):
    """【滤镜测试】【偏好重开】以 binary、environment 重开 source_path，核对实际默认模式、强度和透明度；其余参数提供隔离目录和参考，返回资源记录"""
    destination = temporary / "reload-export"
    with (artifacts / "reload-process.log").open("w") as log:
        process = launch(binary, environment, destination, source_path, log)
        try:
            ready(process, artifacts / "reload-baseline.png")
            annotation = create(process, artifacts, layout, application_log, layout["reload_start"], layout["reload_end"], "pixelate", 42, "reload-default", alpha=255)
            resources = idle(process, artifacts, "reload-editor")
            export(process, environment, destination, source, [annotation], artifacts, "reload")
            return resources
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)


def configuration(temporary, environment):
    """【滤镜测试】【配置隔离】在 temporary 创建工具配置及兼容偏好；返回覆盖 environment 的私有环境和状态目录"""
    config, state = temporary / "config/mark-shot", temporary / "state/mark-shot"
    config.mkdir(parents=True)
    state.mkdir(parents=True)
    (config / "config.toml").write_text('[general]\nto = "cs"\n[notifications]\nenabled = false\n[theme]\nborder = "#60717FFF"\nborder_on_light = "#60717FFF"\n[tools]\ncolor = "#FF4D4DFF"\n[tools.redact]\nmode = "pixelate"\nstrength = 14.0\nstep = 1.0\n')
    (state / "tools.toml").write_text('magnifier = false\npixelate = false\nfilter_mode = "pixelate"\nstrength = 14.0\n[stroke_widths]\nnumbered = 11.0\nfreehand = 7.0\n')
    return dict(environment, XDG_CONFIG_HOME=str(temporary / "config"), XDG_DATA_HOME=str(temporary / "data"), XDG_STATE_HOME=str(temporary / "state")), state


def preferences(state, artifacts):
    """【滤镜测试】【偏好核对】验证 state 保存的独立默认值、不再写入旧开关且没有几何临时状态；向 artifacts 复制配置，无返回值"""
    path = state / "tools.toml"
    saved = tomllib.loads(path.read_text())
    if saved.get("filter_mode") != "pixelate" or saved.get("strength") != 42:
        raise AssertionError(f"existing mode edits changed subsequent defaults or strength did not synchronize: {saved}")
    # 1. 【滤镜测试】【颜色隔离】原版 applyPropertyColor 只修改选中标注，不把属性取色的 alpha 写回默认色
    if saved.get("color", "").upper() != "#FF4D4D":
        raise AssertionError(f"editing selected filter opacity overwrote the default color: {saved}")
    if any(saved["stroke_widths"].get(key) != value for key, value in {"numbered": 11, "freehand": 7}.items()):
        raise AssertionError(f"filter edits changed unrelated stroke widths: {saved}")
    if any(key in saved for key in ("pixelate", "rotation", "filter_geometry", "filters")):
        raise AssertionError(f"obsolete or session-only filter state leaked into tool preferences: {saved}")
    shutil.copyfile(path, artifacts / "tools.toml")


def run(binary, artifacts):
    """【滤镜测试】【完整回归】在私有桌面执行 binary 全部滤镜场景；artifacts 记录当前构建指纹、失败证据与精确输出，不初始化识别模型"""
    environment = session.environment()
    artifacts.mkdir(parents=True, exist_ok=True)
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    with tempfile.TemporaryDirectory(prefix="mark-shot-filter-test-") as directory:
        temporary = Path(directory)
        environment, state = configuration(temporary, environment)
        application_log = state / "mark-shot.log"
        source = source_image()
        source_path = temporary / "source.png"
        source.save(source_path)
        shutil.copyfile(source_path, artifacts / "source.png")
        destination = temporary / "export"
        with (artifacts / "process.log").open("w") as log:
            process = launch(binary, environment, destination, source_path, log)
            try:
                # 1. 【滤镜测试】【真实属性】五模式乘三个强度全部由真实字段与弹层创建，不用日志替代输入预期
                baseline = ready(process, artifacts / "baseline.png")
                layout = geometry(environment, source, baseline.size)
                (artifacts / "layout.json").write_text(json.dumps(layout, indent=2) + "\n")
                annotations = matrix(process, artifacts, layout, application_log)
                popover_resources = panel_lifecycle(process, artifacts, layout)
                underlay, annotation, base = large_region(process, artifacts, layout, application_log)
                annotations.extend((underlay, annotation))
                property_history(process, artifacts, layout, annotation)
                controls(process, artifacts, layout, annotation, base)
                cancel(process, artifacts, layout, annotation, existing=True)
                cancel(process, artifacts, layout, annotation)
                empty_creation(process, artifacts, layout)
                deletion(process, artifacts, layout, annotation, base)
                annotations.append(future_default(process, artifacts, layout, application_log))
                if len(read_commits(application_log)) != len(annotations):
                    raise AssertionError("cancelled gestures or zero-area clicks produced hidden filter commits")
                resources = idle(process, artifacts, "editor")

                # 2. 【滤镜测试】【原图像素】逐像素核对全部模式、双向接缝、冻结源与透明覆盖，并在进程退出后读取图片剪贴板
                export(process, environment, destination, source, annotations, artifacts, "saved")
                preferences(state, artifacts)
                reload_resources = persistence(binary, environment, temporary, artifacts, source_path, source, layout, application_log)
                if (temporary / "data/mark-shot/models").exists():
                    raise AssertionError("ordinary filter editing initialized the OCR model directory")
                if fingerprint != hashlib.sha256(binary.read_bytes()).hexdigest():
                    raise AssertionError("the executable changed during the native filter regression")
                left, top, right, bottom = annotation["native"]
                result = {
                    "sha256": fingerprint,
                    "binary_bytes": binary.stat().st_size,
                    "modes": MODES,
                    "strengths": [4, 14, 48],
                    "matrix_count": 15,
                    "source_size": source.size,
                    "native_large_size": [right - left, bottom - top],
                    "native_tile_seams": {"vertical": left + 512, "horizontal": top + 512},
                    "native_pixels_compared": source.width * source.height,
                    "rgb_encoding_tolerance": 1,
                    "alpha_tolerance": 0,
                    "frozen_source_ignores_earlier_annotations": True,
                    "popover_lifecycle": True,
                    "real_clipboard_strength": True,
                    "real_alpha_track": True,
                    "selected_opacity_keeps_future_defaults": True,
                    "strength_range": True,
                    "single_transaction_property_history": True,
                    "eight_resize_handles": True,
                    "free_and_proportional_resize": True,
                    "free_rotation_degrees": 32,
                    "snapped_rotation_degrees": 45,
                    "draft_and_existing_cancellation": True,
                    "deletion_history": True,
                    "existing_modes_keep_future_defaults": True,
                    "existing_strength_syncs_defaults": True,
                    "preference_reload": True,
                    "persistent_clipboard_after_exit": True,
                    "ocr_directory_untouched": True,
                    "resources": {"editor": resources, "popover": popover_resources, "reload": reload_resources},
                }
                (artifacts / "result.json").write_text(json.dumps(result, indent=2) + "\n")
                print("Wayland native filter modes, properties, gestures, history, idle and independent pixel export checks passed")
            finally:
                if application_log.exists():
                    shutil.copyfile(application_log, artifacts / "application.log")
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=3)


def main():
    """【滤镜测试】【入口】解析二进制与证据目录并执行回归；失败时返回非零退出码"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, default=Path("target/wayland-filter"))
    options = parser.parse_args()
    run(options.binary.resolve(), options.artifacts.resolve())


if __name__ == "__main__":
    main()
