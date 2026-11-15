"""【KDE虚拟机】【标注验证】检查实际工具输入、保存和剪贴板交付"""

import json
import time

from PIL import Image, ImageChops
from checks import visible_selection


def config(session):
    """【KDE虚拟机】【验证配置】按 session 返回独立保存目录与固定界面配置文本，不改变用户设置"""
    return '''[general]
to=""
[desktop]
tray_enabled=false
shortcuts_enabled=false
[notifications]
enabled=false
[history]
enabled=false
[ui]
language="zh-CN"
theme="light"
[startup]
selection_loupe=false
[save]
directory=''' + json.dumps(session.remote + "/output") + '\nmonth_format=""\nfilename_format="%H-%M-%S-%f"\n'


def red_pixels(path):
    """【KDE虚拟机】【正文像素】按 path 返回红色标注像素数，排除桌面底图及控制框颜色"""
    with Image.open(path) as image:
        return sum(1 for r, g, b in image.convert("RGB").get_flattened_data() if r > 180 and g < 130 and b < 130)


def capture_files(session, binary, overrides):
    """【KDE虚拟机】【命令行捕获】按 session、binary、overrides 导出整屏与区域，返回整屏尺寸并核对原始裁切像素"""
    paths = [session.remote + "/output/full.png", session.remote + "/output/region.png"]
    session.command(binary, ["--capture-to", paths[0], "--all-outputs", "--speed"], overrides)
    session.command(binary, ["--capture-to", paths[1], "--region", "150,100,320,240", "--speed"], overrides)
    full_path = session.download(paths[0], "full.png")
    region_path = session.download(paths[1], "region.png")
    with Image.open(full_path) as full, Image.open(region_path) as region:
        assert full.size == session.size, (full.size, session.size)
        assert region.size == (320, 240), region.size
        assert full.convert("RGB").crop((150, 100, 470, 340)).tobytes() == region.convert("RGB").tobytes(), "区域导出像素不等于原始裁切"
        return full.size


def run(session, binary, overrides):
    """【KDE虚拟机】【标注流程】按 session、binary、overrides 检查工具与导出，返回结果；失败也结束测试进程"""
    session.setup(config(session))
    full_size = capture_files(session, binary, overrides)
    pid = session.launch(binary, ["--speed"], overrides=overrides)
    tools = [("r", (230, 200), (480, 330)), ("c", (500, 210), (710, 350)),
             ("a", (250, 380), (640, 450)), ("p", (240, 490), (570, 505)),
             ("k", (720, 220), (875, 320)), ("n", (800, 400), None),
             ("d", (700, 480), (850, 500)), ("m", (400, 350), (490, 420)),
             ("z", (740, 520), (820, 580)), ("h", (260, 540), (500, 545)),
             ("u", (600, 530), (710, 570))]
    try:
        time.sleep(1)
        assert session.alive(pid), session.log("capture")
        session.screenshot("initial")
        session.drag((150, 100), (970, 610))
        visible_selection(session.screenshot("selected"))
        for key, start, end in tools:
            session.key(key)
            time.sleep(0.4)
            visible_selection(session.screenshot(f"tool-{key}"))
            session.drag(start, end) if end else session.click(*start)
            time.sleep(0.4)
            visible_selection(session.screenshot(f"drawn-{key}"))
        session.key("t")
        session.click(180, 440)
        session.screenshot("text-active")
        for key in ["k", "d", "e", "spc", "t", "e", "s", "t"]:
            session.key(key)
        session.screenshot("text-typed")
        session.key("ctrl+ret")
        session.key("v")
        before = session.screenshot("text-committed")
        session.key("ctrl+z")
        undo = session.screenshot("undo")
        session.key("ctrl+shift+z")
        redo = session.screenshot("redo")
        with Image.open(before) as a, Image.open(undo) as b, Image.open(redo) as c:
            region = (160, 420, 310, 480)
            first, removed, restored = [image.convert("RGB").crop(region) for image in (a, b, c)]
            assert ImageChops.difference(first, removed).getbbox(), "撤销没有删除文字正文"
            assert not ImageChops.difference(first, restored).getbbox(), "重做没有恢复文字正文"
        session.key("ctrl+s")
        session.wait(lambda: not session.alive(pid), "保存后截图窗口未关闭")
        files = session.wait(lambda: session.files(session.remote + "/output", "[0-9]*.png"), "没有保存标注图像")
        output = session.download(files[-1], "annotations.png")
        count = red_pixels(output)
        assert count > 1500, f"保存图像中缺少标注正文: {count}"
        with Image.open(output) as image:
            size = image.size
            assert size[0] > 800 and size[1] > 490, size
        # 1. 【KDE虚拟机】【复制链路】重新创建小选区，确认程序退出后剪贴板仍持有正确尺寸的图像
        pid = session.launch(binary, ["--speed"], name="copy", overrides=overrides)
        time.sleep(0.8)
        session.drag((180, 180), (500, 420))
        session.key("ctrl+c")
        session.wait(lambda: not session.alive(pid), "复制后截图窗口未关闭")
        copied = session.clipboard()
        with Image.open(copied) as image:
            assert all(abs(a - b) <= 2 for a, b in zip(image.size, (320, 240))), image.size
        return {"drawing_tools": [value[0] for value in tools] + ["t"], "save_size": size, "full_capture_size": full_size,
                "region_exact_pixels": True, "undo_redo_text": True,
                "saved_annotation_pixels": count, "copy_png": True}
    finally:
        session.stop(pid)
        session.log("capture")
        session.cleanup()
