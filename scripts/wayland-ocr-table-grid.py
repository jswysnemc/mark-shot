#!/usr/bin/env python3
"""【OCR 表格网格】【真实验证】仅在私有 Wayland 检查表格像素、固定尺寸与编辑导出，不初始化模型或外部服务"""

import argparse
import hashlib
import json
from pathlib import Path
import select
import subprocess
import time

import wayland_text_format_session as session
from wayland_ocr_translation_checks import idle, popup
from wayland_ocr_window_layout import tab_point, table_cell_point, table_point, table_reference
from wayland_text_pointer import click, pointer_context
from wayland_window_input_session import ROWS, frame, geometry, screenshot


def pixels(image, window, reference, selected, theme):
    """【OCR 表格网格】【像素断言】按 image、window、reference 比较 selected 单元格及 theme 配色，返回实际像素记录，不比较抗锯齿文字"""
    table, viewport = reference["table"], reference["viewport"]
    left, top = window["x"] + table[0], window["y"] + table[1]
    cell = next(cell for cell in reference["cells"] if (cell["row"], cell["column"]) == selected)
    x, y = left + viewport[0] + cell["rect"][0], top + viewport[1] + cell["rect"][1]
    colors = reference["palette"]
    checks = {
        "selected_fill": ((x + 1, y + 1), tuple(colors["accent"][:3])),
        "header_surface": ((left + viewport[0] + 2, top + 3), tuple(colors["alternate"][:3])),
        "empty_body": ((left + viewport[0] + 2, top + viewport[1] + 100), tuple(colors["base"][:3])),
        "vertical_grid": ((x + cell["rect"][2], y + 2), (203, 213, 225) if theme == "light" else (51, 65, 85)),
        "horizontal_grid": ((x + 2, y + cell["rect"][3]), (203, 213, 225) if theme == "light" else (51, 65, 85)),
    }
    results = {}
    for name, (point, expected) in checks.items():
        actual = image.getpixel(tuple(round(value) for value in point))
        assert actual == expected, f"{theme} {name}: expected {expected}, observed {actual} at {point}"
        results[name] = {"point": point, "actual": actual, "expected": expected}
    # 1. 【OCR 表格网格】【字形存在】每格必须绘出正文像素，色块和边线正确不能替代文字验证
    glyphs = {}
    for cell in reference["cells"]:
        row, column = cell["row"], cell["column"]
        rect = cell["rect"]
        x, y = left + viewport[0] + rect[0], top + viewport[1] + rect[1]
        background = tuple(colors["accent"][:3] if (row, column) == selected else colors["base"][:3])
        count = sum(image.getpixel((px, py)) != background
                    for py in range(round(y + 2), round(y + rect[3] - 2))
                    for px in range(round(x + 3), round(x + rect[2] - 3)))
        assert count > 10, f"{theme} cell ({row}, {column}) contains only {count} text pixels"
        glyphs[f"{row},{column}"] = count
    results["cell_text_pixels"] = glyphs
    return results


def reference(theme):
    """【OCR 表格网格】【主题记录】按 theme 返回真实原版 760 像素内容表格的状态，不修改冻结记录"""
    if theme == "dark":
        return table_reference(760)
    fixture = Path(__file__).resolve().parents[1] / "tests/fixtures/qt-ocr-table-grid.json"
    return next(case["states"][0]["state"] for case in json.loads(fixture.read_text())["cases"]
                if case["theme"] == theme and case["width"] == 760 and case["kind"] == "content")


def run(binary, root, theme):
    """【OCR 表格网格】【完整场景】用 binary、root、theme 执行真实选择、编辑、菜单导出和空闲检查，保存结果，无返回值"""
    root.mkdir(parents=True, exist_ok=False)
    environment = session.environment()
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"), MARK_SHOT_LANG="en")
    config = root / "config/mark-shot/config.toml"
    config.parent.mkdir(parents=True)
    config.write_text("[history]\nenabled = false\n[notifications]\nenabled = false\n[theme]\nbackground = "
                      + json.dumps("#F8FAFC" if theme == "light" else "#0F1117E6") + "\n")
    expected = reference(theme)
    log = root / "process.log"
    before_hash = hashlib.sha256(binary.read_bytes()).hexdigest()
    with session.input_context(), pointer_context(), log.open("w") as output:
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        process = subprocess.Popen([str(binary), "--ocr-result", "--nonotify"], env=dict(environment, WAYLAND_DEBUG="client"),
                                   stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=output)
        try:
            process.stdin.write(frame())
            process.stdin.close()
            assert select.select([process.stdout], [], [], 8)[0] and process.stdout.readline() == b"\n"
            window = geometry(process.pid, environment)
            click(tab_point(window, "Table"))
            # 1. 【OCR 表格网格】【真实选择】在原版单元格中心选择，验证填充和边线，不从内部状态推断像素
            click(table_cell_point(window, 1, 0))
            time.sleep(0.15)
            observed = pixels(screenshot(root, "selected-table"), window, expected, (1, 0), theme)
            # 2. 【OCR 表格网格】【稳定编辑】确认较长内容后仍按初始几何选择其他单元格，表格复制保持独立菜单
            session.key("F2")
            session.key("ctrl+a")
            session.paste_text("Changed value")
            session.key("Return")
            click(table_cell_point(window, 1, 1))
            time.sleep(0.1)
            changed = pixels(screenshot(root, "changed-table"), window, expected, (1, 1), theme)
            click(table_point(window))
            popup(log, 1)
            session.key("Home")
            session.key("Return")
            rows = [row.copy() for row in ROWS]
            rows[1][0] = "Changed value"
            tsv = "\n".join("\t".join(row) for row in rows)
            session.wait_clipboard(environment, tsv)
            (root / "table.tsv").write_text(tsv)
            resources = idle(process.pid, log)
            assert not (root / "data/mark-shot/models").exists()
            session.key("Escape")
            assert process.wait(timeout=4) == 0
            assert hashlib.sha256(binary.read_bytes()).hexdigest() == before_hash
            (root / "result.json").write_text(json.dumps({"sha256": before_hash, "theme": theme,
                "compositor": "sway" if session.backend() is session.sway else "niri", "selected_pixels": observed,
                "edited_stable_geometry_pixels": changed, "edited_tsv": tsv, "resources": resources,
                "no_ocr_or_external_service": True, "window_process_exited": True}, indent=2) + "\n")
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)


def main():
    """【OCR 表格网格】【入口】解析二进制、私有目录和主题后执行真实场景，失败保留非零状态，无参数"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--theme", choices=["dark", "light"], default="dark")
    args = parser.parse_args()
    run(args.binary.resolve(), args.root.resolve(), args.theme)


if __name__ == "__main__":
    main()
