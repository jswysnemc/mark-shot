#!/usr/bin/env python3
"""【选区尺寸对照】【只读采集】编译原版字段、面板和实际 Rust 几何，保存输入与布局指纹"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


def fingerprint(path):
    """【选区尺寸对照】【来源指纹】按 path 返回 SHA-256，不修改文件"""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def geometry(original, root):
    """【选区尺寸对照】【生产几何】从 original 直接编译现有几何源码到 root，返回静态库，不使用旧构建缓存"""
    types = original / "crates/shot-geometry/src/types.rs"
    selection = original / "crates/shot-geometry/src/selection.rs"
    wrapper = root / "geometry_reference.rs"
    wrapper.write_text(f'''#![allow(dead_code)]
#[path = {json.dumps(str(types))}] mod types;
pub use types::{{Point,RectF,Size}};
#[path = {json.dumps(str(selection))}] mod selection;
/// 【选区尺寸对照】【创建入口】传递 a、b、ratio、size 到生产函数，返回原始矩形
#[unsafe(no_mangle)] pub extern "C" fn shot_selection_create(a:Point,b:Point,ratio:f64,size:Size)->RectF{{selection::create(a,b,ratio,size)}}
/// 【选区尺寸对照】【调整入口】传递 before、handle、adjusted、ratio、size，返回原始矩形
#[unsafe(no_mangle)] pub extern "C" fn shot_selection_adjust(before:RectF,handle:i32,adjusted:RectF,ratio:f64,size:Size)->RectF{{selection::adjust(before,handle.into(),adjusted,ratio,size)}}
/// 【选区尺寸对照】【尺寸入口】传递 current、requested、image，返回原始矩形
#[unsafe(no_mangle)] pub extern "C" fn shot_selection_resize(current:RectF,requested:Size,image:Size)->RectF{{selection::resize(current,requested,image)}}
/// 【选区尺寸对照】【联动入口】传递 value、ratio、from_width，返回原始整数尺寸
#[unsafe(no_mangle)] pub extern "C" fn shot_selection_side(value:i32,ratio:f64,from_width:i32)->i32{{selection::related_side(value,ratio,from_width!=0)}}
''')
    library = root / "libselection_reference.a"
    subprocess.run(["rustc", "--edition=2024", "--crate-type=staticlib", "-O", str(wrapper), "-o", str(library)], check=True)
    return library


def collect(original, root, field_navigation=False, field_clicks=False, field_native_pointer=False):
    """【选区尺寸对照】【实际采集】按 original、root 与三种字段标记选择生产探针，返回固定英文及九十六 DPI 的来源指纹"""
    root.mkdir(parents=True, exist_ok=False)
    project = Path(__file__).resolve().parents[1]
    source = original / "qt/src"
    field_navigation |= field_clicks
    fixture = project / "tests/fixtures/qt-selection-size" / ("field_native_pointer.cpp" if field_native_pointer else "field_navigation.cpp" if field_navigation else "main.cpp")
    harness_before = fingerprint(fixture)
    helpers = [fixture.parent / name for name in ("field_probe.cpp", "field_click_sequences.cpp")] if field_navigation else [fixture.parent / "field_probe.cpp"] if field_native_pointer else []
    harness_sources = {path.name: fingerprint(path) for path in [fixture, *helpers, *([fixture.parent / "field_probe.h"] if field_navigation or field_native_pointer else [])]}
    names = ["selection_aspect/selection_size_panel.cpp", "selection_aspect/selection_size_panel.h",
             "selection_aspect/selection_size_field.cpp", "selection_aspect/selection_size_field.h",
             "selection_aspect/selection_ratio_popup.cpp", "selection_aspect/selection_ratio_popup.h",
             "selection_aspect/selection_hud_layout.cpp", "selection_aspect/selection_hud_layout.h",
             "selection_aspect/selection_hud_painter.cpp", "selection_aspect/selection_hud_painter.h",
             "selection_aspect/selection_hud_colors.cpp", "selection_aspect/selection_hud_colors.h",
             "selection_aspect/selection_aspect_ratio.cpp", "selection_aspect/selection_aspect_ratio.h",
             "ui/theme.cpp", "ui/theme.h", "ui/toolbar_palette.cpp", "ui/toolbar_palette.h"]
    paths = {name: source / name for name in names}
    paths.update({name: original / name for name in ["crates/shot-geometry/src/types.rs", "crates/shot-geometry/src/selection.rs", "qt/bridge/geometry_bridge.h"]})
    before = {name: fingerprint(path) for name, path in paths.items()}
    library = geometry(original, root)
    generated = []
    for name in ["selection_size_panel", "selection_size_field", "selection_ratio_popup"]:
        destination = root / f"moc_{name}.cpp"
        subprocess.run(["/usr/lib/qt6/moc", str(source / f"selection_aspect/{name}.h"), "-o", str(destination)], check=True)
        generated.append(destination)
    flags = subprocess.check_output(["pkg-config", "--cflags", "--libs", "Qt6Widgets"], text=True).split()
    if field_navigation or field_native_pointer:
        version = subprocess.check_output(["pkg-config", "--modversion", "Qt6Widgets"], text=True).strip()
        headers = Path(subprocess.check_output(["pkg-config", "--variable=includedir", "Qt6Widgets"], text=True).strip())
        for module in ["QtCore", "QtGui", "QtWidgets"]:
            flags.extend(["-I" + str(headers / module / version), "-I" + str(headers / module / version / module)])
    binary = root / "reference"
    with (root / "build.log").open("w") as log:
        subprocess.run(["c++", "-std=c++17", "-fPIC", str(fixture), *(str(path) for path in helpers), *(str(source / n) for n in names if n.endswith(".cpp")),
                        *(str(n) for n in generated), "-I" + str(source), "-I" + str(original / "qt/bridge"),
                        str(library), *flags, "-ldl", "-lpthread", "-lm", "-o", str(binary)], stdout=log, stderr=log, check=True)
    output = root / "reference.json"
    process = subprocess.run([str(binary), str(output), *(["--click-sequences"] if field_clicks else [])], env=dict(os.environ, QT_QPA_PLATFORM="offscreen", QT_FONT_DPI="96"), capture_output=True, text=True, check=True)
    (root / "run.log").write_text(process.stderr)
    if before != {name: fingerprint(path) for name, path in paths.items()} or harness_before != fingerprint(fixture):
        raise AssertionError("original selection size sources changed")
    if harness_sources != {name: fingerprint(fixture.parent / name) for name in harness_sources}:
        raise AssertionError("selection size probe sources changed")
    data = json.loads(output.read_text())
    index = {"sources": before, "harness": harness_before, "harness_sources": harness_sources, "binary": fingerprint(binary), "reference": fingerprint(output),
             "rust_reference_library": fingerprint(library), "qt": data["qt"], "font": data["font"], "dpi": data["dpi"],
             "counts": {name: len(data[name]) for name in ["validation", "states", "creations", "adjustments", "linked", "resized", "placements", "aligned_exports", "keys", "mouse", "sequences"] if name in data}}
    index["renders"] = {path.name: fingerprint(path) for path in sorted((root / "render").glob("*.png"))}
    (root / "index.json").write_text(json.dumps(index, indent=2) + "\n")
    return index


def main():
    """【选区尺寸对照】【执行入口】解析只读原版和新输出目录，打印采集指纹，返回空值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--original", type=Path, default=Path.home() / "workspace/mark-shot")
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--field-navigation", action="store_true")
    parser.add_argument("--field-clicks", action="store_true")
    parser.add_argument("--field-native-pointer", action="store_true")
    options = parser.parse_args()
    print(json.dumps(collect(options.original.resolve(), options.root.resolve(), options.field_navigation, options.field_clicks, options.field_native_pointer), indent=2))


if __name__ == "__main__":
    main()
