#!/usr/bin/env python3
"""【KDE输入法回归】【候选可见】在已配置 Fcitx5 Rime 的测试虚拟机中验证浮层配置不会遮挡候选框"""

import argparse
import json
from pathlib import Path
import re
import sys
import time

from PIL import Image, ImageChops

sys.path.insert(0, str(Path(__file__).resolve().parent / "kde_vm"))
from session import Session


def run(session, binary):
    """【KDE输入法回归】【完整流程】按 session 和 binary 输入 ni，返回协议与候选像素结果，退出恢复输入法状态"""
    session.setup('[general]\nto=""\noverlay="layer"\n[desktop]\ntray_enabled=false\nshortcuts_enabled=false\n[history]\nenabled=false\n[notifications]\nenabled=false\n')
    previous = session.guest("print(subprocess.check_output(['fcitx5-remote'],text=True).strip())")
    pid = session.launch(binary, ["--speed"], overrides={"WAYLAND_DEBUG": "client"})
    try:
        # 1. 【KDE输入法回归】【文字目标】窗口出现后选区并创建文字框，候选出现前记录同一画面
        session.wait(lambda: any(window["app"].startswith("mark-shot") for window in session.windows()), "截图编辑器未出现")
        session.drag((150, 150), (900, 600))
        session.key("t")
        session.click(300, 300)
        session.screenshot("before-preedit")
        session.guest("subprocess.run(['fcitx5-remote','-o'],check=True)")
        session.key("n")
        session.key("i")
        time.sleep(0.4)
        # 2. 【KDE输入法回归】【候选位置】候选窗口必须在光标下方，协议焦点和预编辑必须同时到达
        candidate = session.wait(lambda: next((window for window in session.windows()
            if window["app"] == "kwin_wayland" and 280 <= window["x"] <= 340
            and 320 <= window["y"] <= 390 and window["width"] < 600), None), "输入法候选窗口未出现")
        session.screenshot("candidate")
        log = session.log("capture")
        assert 'preedit_string, (Some("ni")' in log, "程序未收到 ni 预编辑"
        assert "set_cursor_rectangle" in log, "程序未提供候选坐标"
        assert not re.search(r"get_layer_surface\([^\n]+Some\(\"utility\"", log), "KDE 编辑器仍使用遮挡候选的浮层"
        # 3. 【KDE输入法回归】【可见像素】与候选出现前的同一区域比较，候选被截图浮层遮挡时像素不会改变
        with Image.open(session.root / "before-preedit.png") as before, Image.open(session.root / "candidate.png") as after:
            sx, sy = after.width / 1280, after.height / 800
            x, y, width, height = (candidate[key] for key in ("x", "y", "width", "height"))
            area = tuple(round(value) for value in (x * sx, y * sy, (x + width) * sx, (y + height) * sy))
            difference = ImageChops.difference(before.convert("RGB").crop(area), after.convert("RGB").crop(area))
            changed = sum(max(difference.getpixel((x, y))) > 20
                          for y in range(difference.height) for x in range(difference.width))
            assert changed > difference.width * difference.height * 0.2, "候选窗口存在但画面受到遮挡"
        return {"preedit_received": True, "cursor_rectangle_sent": True, "candidate_visible": True, "changed_pixels": changed}
    finally:
        session.stop(pid)
        session.guest("subprocess.run(['fcitx5-remote',values],check=True)", "-o" if previous == "2" else "-c")
        session.log("capture")


def main():
    """【KDE输入法回归】【入口】读取虚拟机和程序参数，执行回归并保存结果，无参数、无返回值"""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--domain", default="arch-kde")
    parser.add_argument("--host", required=True)
    parser.add_argument("--binary", required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    session = Session(options.domain, options.host, options.root)
    result = run(session, options.binary)
    (session.root / "result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(result, ensure_ascii=False))


if __name__ == "__main__":
    main()
