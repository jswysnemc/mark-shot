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
from annotations import config, red_pixels


def run(session, binary):
    """【KDE输入法回归】【完整流程】按 session 和 binary 输入并保存中文，返回浮层、候选和导出结果，退出恢复输入法状态"""
    session.setup(config(session).replace('to=""', 'to=""\noverlay="layer"') + '[history]\nenabled=false\n')
    previous = session.guest("print(subprocess.check_output(['fcitx5-remote'],text=True).strip())")
    pid = session.launch(binary, ["--speed"], overrides={"WAYLAND_DEBUG": "client"})
    try:
        # 1. 【KDE输入法回归】【文字目标】窗口出现后选区并创建文字框，候选出现前记录同一画面
        session.wait(lambda: session.guest(
            "path=Path(values);text=path.read_text() if path.exists() else '';print('ready' if 'get_layer_surface(' in text and '.attach(wl_buffer' in text else '')",
            f"{session.remote}/capture.log"), "截图编辑器未创建 layer-shell 覆盖层")
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
        # 3. 【KDE输入法回归】【可见像素】与候选出现前的同一区域比较，候选被截图浮层遮挡时像素不会改变
        with Image.open(session.root / "before-preedit.png") as before, Image.open(session.root / "candidate.png") as after:
            sx, sy = after.width / 1280, after.height / 800
            x, y, width, height = (candidate[key] for key in ("x", "y", "width", "height"))
            area = tuple(round(value) for value in (x * sx, y * sy, (x + width) * sx, (y + height) * sy))
            difference = ImageChops.difference(before.convert("RGB").crop(area), after.convert("RGB").crop(area))
            changed = sum(max(difference.getpixel((x, y))) > 20
                          for y in range(difference.height) for x in range(difference.width))
            assert changed > difference.width * difference.height * 0.2, "候选窗口存在但画面受到遮挡"
        assert re.search(r'get_layer_surface\([^\n]+, 3, Some\("utility"\)\)', log), "KDE 编辑器未保留原 Overlay 层级"
        assert ".set_layer(2)" in log, "文字输入期间未降低层级避让候选"
        assert "get_toplevel(" not in log, "截图编辑器意外创建普通窗口"
        # 4. 【KDE输入法回归】【中文提交】选择首个候选并结束文字编辑，必须收到中文正文而非仅显示拼音
        session.click(candidate["x"] + 17, candidate["y"] + candidate["height"] // 2)
        session.wait(lambda: 'commit_string, (Some("你")' in session.log("capture"), "输入法未提交中文正文")
        session.key("ctrl+ret")
        session.key("v")
        session.wait(lambda: ".set_layer(3)" in session.log("capture"), "文字输入结束后未恢复 Overlay 层级")
        session.screenshot("chinese-committed")
        # 5. 【KDE输入法回归】【保存结果】保存图像必须保留选区大小和文字正文，候选框不进入导出图像
        session.key("ctrl+s")
        session.wait(lambda: not session.alive(pid), "保存后截图浮层未关闭")
        files = session.wait(lambda: session.files(session.remote + "/output", "[0-9]*.png"), "没有保存中文标注图像")
        output = session.download(files[-1], "chinese-annotation.png")
        annotation_pixels = red_pixels(output)
        assert annotation_pixels > 100, f"保存图像缺少中文正文: {annotation_pixels}"
        with Image.open(output) as image:
            assert all(abs(actual - expected) <= 2 for actual, expected in zip(image.size, (750, 450))), image.size
        return {"layer_shell_retained": True, "layer_restored": True, "preedit_received": True,
                "cursor_rectangle_sent": True, "candidate_visible": True, "changed_pixels": changed,
                "chinese_committed": True, "saved_annotation_pixels": annotation_pixels}
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
