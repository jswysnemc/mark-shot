#!/usr/bin/env python3
"""【Wayland 测试】【箭头曲线】验证原生头部、线型、多控制点、属性历史、原图和剪贴板，仅操作隔离桌面"""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib

from PIL import Image, ImageDraw

import wayland_test_session as session
from wayland_arrow_checks import HEADS, PATTERNS, check_shaft, export, geometry, idle, quiet, ready, red_near
from wayland_arrow_scenarios import cancel, choose, click, controls, create_edit_curve, draw, edit_width, existing_styles, key_tool, line_patterns, matrix, multiple_controls, panel_lifecycle


def persistence(binary, environment, temporary, artifacts, source_path, source, layout):
    """【箭头测试】【偏好重开】以 binary、environment 重开 source_path，验证独立宽度及样式；temporary、artifacts、source、layout 提供输出和像素断言"""
    destination = temporary / "reload-export"
    with (artifacts / "reload.log").open("w") as log:
        process = subprocess.Popen([str(binary), "--fullscreen", "--fullscreen-default-tool", "arrow", "--to", "cs", "--output", str(destination), str(source_path)], env=environment, stderr=log)
        try:
            ready(process, artifacts / "reload-baseline.png")
            draw(layout["reload_start"], layout["reload_end"])
            key_tool("line")
            draw(layout["reload_line_start"], layout["reload_line_end"])
            image = quiet(process, artifacts / "reload-defaults.png", layout)
            start, end = layout["reload_start"], layout["reload_end"]
            check_shaft(image, start, end, "dashed", margin=65)
            check_shaft(image, layout["reload_line_start"], layout["reload_line_end"], "dotted")
            if not red_near(image, (start[0] + 45, start[1] - 28), radius=5) or not red_near(image, (end[0] - 45, end[1] - 28), radius=5):
                raise RuntimeError("the reloaded bidirectional KDE head or its nine-pixel width was not visible")
            export(process, environment, destination, source, artifacts, "reload")
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)


def run(binary, artifacts):
    """【箭头测试】【完整回归】以 binary 执行独立样式、骨架和导出流程；artifacts 保存当前构建指纹，不初始化识别模型"""
    environment = session.environment()
    artifacts.mkdir(parents=True, exist_ok=True)
    fingerprint = hashlib.sha256(binary.read_bytes()).hexdigest()
    with tempfile.TemporaryDirectory(prefix="mark-shot-arrow-test-") as directory:
        temporary = Path(directory)
        config, state = temporary / "config/mark-shot", temporary / "state/mark-shot"
        config.mkdir(parents=True)
        state.mkdir(parents=True)
        (config / "config.toml").write_text('[general]\nto = "cs"\n[notifications]\nenabled = false\n[theme]\nborder = "#60717FFF"\nborder_on_light = "#60717FFF"\n[tools]\ncolor = "#FF4D4D80"\n[tools.arrow]\nwidth = 3.0\nstyle = "fletched"\nline_style = "solid"\n[tools.line]\nwidth = 3.0\nstyle = "solid"\n')
        (state / "tools.toml").write_text('magnifier = false\n[stroke_widths]\nnumbered = 11.0\nfreehand = 7.0\n')
        environment.update(XDG_CONFIG_HOME=str(temporary / "config"), XDG_DATA_HOME=str(temporary / "data"), XDG_STATE_HOME=str(temporary / "state"))
        source = Image.new("RGBA", (1600, 900), (30, 34, 38, 255))
        painter = ImageDraw.Draw(source)
        painter.rectangle((0, 0, 1599, 7), fill=(0, 0, 0, 0))
        painter.rectangle((20, 20, 42, 42), fill=(255, 0, 0, 128))
        source_path = temporary / "source.png"
        source.save(source_path)
        layout = geometry(environment, source)
        destination = temporary / "export"
        with (artifacts / "process.log").open("w") as log:
            process = subprocess.Popen([str(binary), "--fullscreen", "--fullscreen-default-tool", "arrow", "--to", "cs", "--output", str(destination), str(source_path)], env=environment, stderr=log)
            try:
                # 1. 【箭头测试】【真实样式】以真实属性面板和控制点生成全部直线、曲线及普通线型
                ready(process, artifacts / "baseline.png")
                matrix(process, artifacts, layout)
                panel_lifecycle(process, artifacts, layout)
                create_edit_curve(process, artifacts, layout)
                controls(process, artifacts, layout)
                multiple_controls(process, artifacts, layout)
                cancel(process, artifacts, layout, existing=True)
                cancel(process, artifacts, layout)
                key_tool("pick")
                middle = ((layout["edit_start"][0] + layout["edit_end"][0]) / 2, layout["edit_start"][1] - 40)
                click(middle)
                edit_width(process, artifacts, layout, True, 9)
                key_tool("arrow")
                existing_styles(process, artifacts, layout)
                line_patterns(process, artifacts, layout)
                choose(process, artifacts, layout, "head", 3, "future-head")
                choose(process, artifacts, layout, "pattern", 1, "future-pattern")
                resources = idle(process, artifacts, "editor")

                # 2. 【箭头测试】【原图输出】退出后读取图片剪贴板，校验原始分辨率、透明度和未修改像素
                export(process, environment, destination, source, artifacts, "saved")
                saved = tomllib.loads((state / "tools.toml").read_text())
                expected = {"arrow_style": "bidirectional_kde", "arrow_line_style": "dashed", "line_style": "dotted"}
                if any(saved.get(key) != value for key, value in expected.items()):
                    raise RuntimeError(f"the independent styles did not persist: {saved}")
                if any(saved["stroke_widths"][key] != value for key, value in {"arrow": 9, "line": 7, "numbered": 11, "freehand": 7}.items()):
                    raise RuntimeError(f"line widths did not persist independently: {saved}")
                if any(key in saved for key in ("controls", "line_session", "selected_point")):
                    raise RuntimeError("temporary skeleton geometry leaked into tool preferences")
                shutil.copyfile(state / "tools.toml", artifacts / "tools.toml")

                # 3. 【箭头测试】【重开隔离】第二会话验证实际默认样式，最后核对构建与模型目录
                persistence(binary, environment, temporary, artifacts, source_path, source, layout)
                if (temporary / "data/mark-shot/models").exists():
                    raise RuntimeError("ordinary arrow editing initialized the OCR model directory")
                if fingerprint != hashlib.sha256(binary.read_bytes()).hexdigest():
                    raise RuntimeError("the executable changed during the arrow regression")
                result = {"sha256": fingerprint, "binary_bytes": binary.stat().st_size, "heads": HEADS, "line_styles": PATTERNS, "arrow_matrix": 24, "source_size": source.size, "widths": {"arrow": 9, "line": 7}, "popover_lifecycle": True, "double_click_controls": True, "multiple_controls": True, "control_deletion_history": True, "external_controls": True, "proportional_resize": True, "snapped_rotation": True, "existing_styles_keep_future_defaults": True, "half_transparent_seams": True, "draft_and_drag_cancellation": True, "preference_reload": True, "persistent_clipboard": True, "resources": resources}
                (artifacts / "result.json").write_text(json.dumps(result, indent=2) + "\n")
                print("Wayland arrow heads, line styles, curves, controls, history, idle and native export checks passed")
            finally:
                application_log = state / "mark-shot.log"
                if application_log.exists():
                    shutil.copyfile(application_log, artifacts / "application.log")
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=3)


def main():
    """【箭头测试】【入口】解析二进制和证据目录并执行回归；失败时返回非零退出码"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--artifacts", type=Path, default=Path("target/wayland-arrow"))
    options = parser.parse_args()
    run(options.binary.resolve(), options.artifacts.resolve())


if __name__ == "__main__":
    main()
