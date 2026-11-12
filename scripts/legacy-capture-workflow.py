#!/usr/bin/env python3
"""【历史问题回归】【分数缩放】在私有双屏 Sway 比较原生截图与独立图形窗口，不连接用户桌面"""

import argparse
import json
from pathlib import Path
import subprocess
import sys
import time

from PIL import Image, ImageChops
from wayland_headless_runtime import HeadlessRuntime


def fixture(identity):
    """【历史问题回归】【独立画面】按 identity 创建 GTK 全屏四色窗口，无返回值，内容不依赖截图实现"""
    import gi
    gi.require_version("Gtk", "3.0")
    from gi.repository import Gtk

    window = Gtk.Window(title=f"legacy-capture-fixture-{identity}")
    area = Gtk.DrawingArea()

    def draw(widget, context):
        """【历史问题回归】【四角标记】按 widget 尺寸向 context 绘制不同象限，覆盖屏幕最外侧边缘"""
        width, height = widget.get_allocated_width(), widget.get_allocated_height()
        colors = [(0.7, 0.2, 0.3), (0.2, 0.6, 0.4), (0.2, 0.3, 0.8), (0.8, 0.7, 0.2)]
        for index in range(4):
            context.set_source_rgb(*colors[(index + identity) % 4])
            context.rectangle(index % 2 * width / 2, index // 2 * height / 2, width / 2, height / 2)
            context.fill()

    area.connect("draw", draw)
    window.add(area)
    window.connect("destroy", Gtk.main_quit)
    window.fullscreen()
    window.show_all()
    Gtk.main()


def verify(binary, artifacts, sway):
    """【历史问题回归】【像素核对】按 binary、artifacts、sway 捕获两块输出并比较环境变量变化，返回证据摘要"""
    artifacts.mkdir(parents=True, exist_ok=False)
    config = artifacts / "config.toml"
    config.write_text('[history]\nenabled=false\n[notifications]\nenabled=false\n[startup]\ninclude_cursor=false\n')
    checks = []
    with HeadlessRuntime(artifacts / "runtime", sway) as environment:
        environment.update(GDK_BACKEND="wayland", NO_AT_BRIDGE="1")
        # 1. 【历史问题回归】【环境隔离】截图采用私有协议，不继承外部后端或 Qt 缩放覆盖
        for name in ("MARK_SHOT_CAPTURE", "MARK_SHOT_BACKEND", "QT_FONT_DPI", "QT_SCALE_FACTOR", "GDK_SCALE"):
            environment.pop(name, None)
        processes = []
        with (artifacts / "fixtures.log").open("w") as log:
            try:
                for identity in range(2):
                    process = subprocess.Popen([sys.executable, str(Path(__file__).resolve()), "--fixture", str(identity)],
                                               env=environment, stdout=log, stderr=log)
                    processes.append(process)
                    time.sleep(0.5)
                    subprocess.run([str(sway.resolve().parent / "swaymsg"),
                                    f'[title="legacy-capture-fixture-{identity}"] move container to output HEADLESS-{identity + 1}'],
                                   env=environment, check=True, stdout=log, stderr=log, timeout=3)
                time.sleep(0.3)
                for number, scale in [(1, "1.25"), (2, "1.5")]:
                    name = f"HEADLESS-{number}"
                    reference = artifacts / f"{name}-grim.png"
                    subprocess.run(["grim", "-o", name, "-s", scale, str(reference)], env=environment, check=True, timeout=3)
                    expected = Image.open(reference).convert("RGB")
                    assert expected.size == (1200, 900)
                    colors = expected.getcolors(maxcolors=4)
                    assert colors is not None and len(colors) == 4, "fixture did not cover the output"
                    for label, changes in [("baseline", {}), ("qt-font", {"QT_FONT_DPI": "128"}),
                                           ("qt-scale", {"QT_FONT_DPI": "192", "QT_SCALE_FACTOR": "2", "GDK_SCALE": "2"})]:
                        path = artifacts / f"{name}-{label}.png"
                        result = subprocess.run([str(binary), "--config", str(config), "--display", name,
                                                 "--capture-to", str(path), "--nonotify"],
                                                env=dict(environment, **changes), capture_output=True, timeout=10, check=True)
                        actual = Image.open(path).convert("RGB")
                        assert actual.size == expected.size, (name, label, actual.size)
                        assert ImageChops.difference(expected, actual).getbbox() is None, (name, label)
                        checks.append({"output": name, "scale": scale, "case": label,
                                       "size": actual.size, "report": json.loads(result.stdout)})
            finally:
                for process in processes:
                    if process.poll() is None:
                        process.terminate()
                        process.wait(timeout=3)
    return {"checks": checks, "pixel_exact": True}


def main():
    """【历史问题回归】【命令入口】无参数，解析构建与证据目录并执行独立画面或验证流程"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--fixture", type=int)
    parser.add_argument("--binary", type=Path, default=Path("target/release/mark-shot"))
    parser.add_argument("--sway", type=Path, default=Path("target/test-tools/sway/usr/bin/sway"))
    parser.add_argument("--artifacts", type=Path, default=Path("target/legacy-capture"))
    options = parser.parse_args()
    if options.fixture is not None:
        fixture(options.fixture)
        return
    result = verify(options.binary.resolve(), options.artifacts.resolve(), options.sway)
    (options.artifacts / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result))


if __name__ == "__main__":
    main()
