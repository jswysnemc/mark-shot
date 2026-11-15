"""【KDE虚拟机】【交互回归】在运行中的 Plasma 虚拟机复现截图交互并保存独立证据"""

import argparse
import json
from pathlib import Path
import time

from checks import visible_selection
from session import Session

CONFIG = '''[general]
to=""
[desktop]
tray_enabled=false
shortcuts_enabled=false
[notifications]
enabled=false
[history]
enabled=false
'''


def rectangle(session, binary, overrides, shell="auto"):
    """【KDE虚拟机】【矩形复现】按 session、binary、overrides、shell 框选后切换矩形，返回结果；失败也回收进程"""
    session.setup(CONFIG.replace('to=""', f'to=""\noverlay="{shell}"'))
    pid = session.launch(binary, ["--speed"], overrides=overrides)
    try:
        time.sleep(1)
        assert session.alive(pid), session.log("capture")
        session.screenshot("initial")
        session.drag((150, 100), (950, 550))
        visible_selection(session.screenshot("selection"))
        session.key("r")
        time.sleep(0.5)
        visible_selection(session.screenshot("rectangle-tool"))
        session.drag((300, 240), (600, 400))
        visible_selection(session.screenshot("rectangle-drawn"))
        return {"rectangle_tool_kept_selection": True, "binary": binary}
    finally:
        session.stop(pid)
        session.log("capture")


def main():
    """【KDE虚拟机】【命令入口】读取目标、程序与证据目录，执行矩形检查，写入可审查结果，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--domain", default="arch-kde")
    parser.add_argument("--host", required=True)
    parser.add_argument("--binary", default="/tmp/mark-shot-kde-current")
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--override", action="append", default=[])
    parser.add_argument("--overlay-shell", choices=("auto", "layer", "window"), default="auto")
    parser.add_argument("--case", choices=("rectangle", "annotations", "windows", "recording", "scrolling", "desktop", "window_detection"), default="rectangle")
    options = parser.parse_args()
    session = Session(options.domain, options.host, options.root)
    overrides = dict(value.split("=", 1) for value in options.override)
    try:
        if options.case != "rectangle":
            from importlib import import_module
            run = import_module(options.case).run
            result = run(session, options.binary, overrides)
        else:
            result = rectangle(session, options.binary, overrides, options.overlay_shell)
    except Exception as error:
        result = {"error": str(error), "binary": options.binary}
        raise
    finally:
        (session.root / "result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
