#!/usr/bin/env python3
"""【外部动作对照】【只读采集】提取完整原版函数并编译 Qt 探针，保存真实参数和侧边列表，不操作用户桌面"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


def fingerprint(path):
    """【外部动作对照】【源码指纹】读取 path 并返回 SHA-256，不修改文件"""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def function(text, signature):
    """【外部动作对照】【完整提取】从 text 获取 signature 对应完整顶层函数，返回未修改源码，缺少边界直接失败"""
    start = text.index(signature)
    end = text.index("\n}\n", start) + 3
    return text[start:end]


def collect(original, root):
    """【外部动作对照】【生产采集】从 original 提取并在 root 编译函数，返回原版实际状态；外围目录和命令接收端保持隔离"""
    root.mkdir(parents=True, exist_ok=False)
    project = Path(__file__).resolve().parents[1]
    sources = {name: original / "qt/src" / name for name in (
        "shot_window_internal.cpp", "shot_window_modes.cpp", "shot_window_side_panels.cpp", "ui/toolbar_panel_style.cpp", "ui/toolbar_palette.cpp", "ui/toolbar_palette.h")}
    before = {name: fingerprint(path) for name, path in sources.items()}
    functions = []
    internal = sources["shot_window_internal.cpp"].read_text()
    for signature in ("QString desktopEntryValue(", "bool desktopEntryBool(", "bool desktopEntrySupportsImage(", "QStringList expandDesktopExec("):
        functions.append(function(internal, signature))
    modes = sources["shot_window_modes.cpp"].read_text()
    for signature in ("QVector<ShotWindow::DesktopApp> ShotWindow::imageDesktopApps()", "QVector<ShotWindow::ExtensionCommand> ShotWindow::extensionCommands("):
        functions.append(function(modes, signature))
    functions.insert(3, function(internal, "bool extensionCommandUsesImagePlaceholder("))
    panels = sources["shot_window_side_panels.cpp"].read_text()
    for signature in ("void ShotWindow::updateOpenWithPanel()", "void ShotWindow::updateOpenWithPanelGeometry()", "void ShotWindow::updateExtensionPanel()", "void ShotWindow::updateExtensionPanelGeometry()", "QRect ShotWindow::sidePanelGeometry("):
        functions.append(function(panels, signature))
    palette = function(sources["ui/toolbar_palette.cpp"].read_text(), "QString cssColor(")
    style = function(sources["ui/toolbar_panel_style.cpp"].read_text(), "QString openWithPanelStyleSheet(")
    functions.insert(0, "namespace markshot::ui {\n" + palette + "\n}\nnamespace markshot::theme { using markshot::ui::ToolbarPalette; using markshot::ui::cssColor;\n" + style + "\n}")
    harness = project / "tests/fixtures/qt-external-actions-reference/main.cpp"
    main = root / "main.cpp"
    main.write_text(harness.read_text().replace("/* PRODUCTION_FUNCTIONS */", "\n\n".join(functions)))
    flags = subprocess.check_output(["pkg-config", "--cflags", "--libs", "Qt6Widgets"], text=True).split()
    binary = root / "reference"
    with (root / "build.log").open("w") as log:
        subprocess.run(["c++", "-std=c++17", "-fPIC", str(main), "-I" + str(original / "qt/src"), *flags, "-o", str(binary)], stdout=log, stderr=log, check=True)
    run = subprocess.run([str(binary), str(root / "images")], env=dict(os.environ, QT_QPA_PLATFORM="offscreen", MARK_SHOT_LANG="english"), capture_output=True, text=True, check=True)
    (root / "run.log").write_text(run.stderr)
    state = json.loads((root / "images/reference.json").read_text())
    if len(state["panels"]) != 8 or len(state["exec"]) != 6:
        raise AssertionError("original function matrix was incomplete")
    if before != {name: fingerprint(path) for name, path in sources.items()}:
        raise AssertionError("reference source changed during collection")
    index = {"sources": before, "harness": fingerprint(harness), "generated_source": fingerprint(main), "binary": fingerprint(binary), "qt": state["qt"], "panels": 8, "commands": 6}
    (root / "index.json").write_text(json.dumps(index, ensure_ascii=False, indent=2) + "\n")
    return index


def main():
    """【外部动作对照】【命令入口】解析 original、root 并采集独立 Qt 状态，返回验证索引"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--original", type=Path, default=Path.home() / "workspace/mark-shot")
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(collect(options.original.resolve(), options.root.resolve()), indent=2))


if __name__ == "__main__":
    main()
