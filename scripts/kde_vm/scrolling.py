"""【KDE虚拟机】【滚动截图】独立测试页面接收真实输入，长图与原始纹理逐像素比较"""

import time

from PIL import Image, ImageChops
from annotations import config


def texture(width, height):
    """【KDE虚拟机】【独立纹理】按 width、height 生成测试页面从局部坐标 80,80 开始的期望像素，返回 RGB 图像"""
    pixels = bytearray()
    for y in range(80, 80 + height):
        seed = (y * 2654435761) & 0xffffffff
        rotated = ((seed << 7) | (seed >> 25)) & 0xffffffff
        for x in range(80, 80 + width):
            value = rotated ^ ((x * 2246822519) & 0xffffffff)
            pixels.extend((value & 255, (value >> 8) & 255, (value >> 16) & 255))
    return Image.frombytes("RGB", (width, height), bytes(pixels))


def page_frame(session, area, expected):
    """【KDE虚拟机】【页面就绪】按 session、area、expected 核对真实首帧，匹配返回图像；过渡帧返回空值"""
    with Image.open(session.screenshot("page-source")) as frame:
        image = frame.convert("RGB").crop(area)
    return image if not ImageChops.difference(image, expected).getbbox() else None


def controls(path):
    """【KDE虚拟机】【滚动按钮】按 path 识别绿色保存按钮，返回暂停和复制位置；画面没有面板时返回空值"""
    with Image.open(path) as frame:
        image = frame.convert("RGB")
    remaining = {(x, y) for y in range(image.height) for x in range(image.width)
                 if image.getpixel((x, y)) in ((16, 185, 129), (52, 211, 153))}
    groups = []
    while remaining:
        pending, group = [remaining.pop()], []
        while pending:
            x, y = pending.pop()
            group.append((x, y))
            for point in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]:
                if point in remaining:
                    remaining.remove(point)
                    pending.append(point)
        groups.append(group)
    pixels = max(groups, key=len, default=[])
    if len(pixels) < 200:
        return None
    x = (min(p[0] for p in pixels) + max(p[0] for p in pixels)) / 2
    y = (min(p[1] for p in pixels) + max(p[1] for p in pixels)) / 2
    ink = sum(1 for row in range(round(y) - 2, round(y) + 3)
              for column in range(round(x - 94) + 5, round(x - 94) + 8)
              if min(image.getpixel((column, row))) < 130
              and max(image.getpixel((column, row))) - min(image.getpixel((column, row))) > 15)
    return {"pause": (x - 94, y), "copy": (x + 47, y), "paused": ink > 3}


def run(session, binary, overrides, page_binary="/tmp/mark-shot-kde-scroll-page", wayland_display="/run/user/1000/wayland-1", hidden_controls=False):
    """【虚拟机测试】【滚动流程】按会话、程序、环境和 hidden_controls 框选四次滚动，返回原始像素校验结果"""
    session.setup(config(session) + '[capture]\nhide_own_windows=false\n')
    session.guest("""
root=Path(values);(root/'runtime').mkdir(mode=0o700,exist_ok=True)
(root/'runtime/runtime.json').write_text(json.dumps({'private_vm_test':True}))
""", session.remote)
    page = session.launch(page_binary, [session.remote + "/position.txt"], name="page", with_config=False,
                          overrides={"XDG_RUNTIME_DIR": session.remote + "/runtime", "WAYLAND_DISPLAY": wayland_display})
    pid = None
    try:
        window = session.wait(lambda: session.windows("mark-shot-scroll-test-page"), "原生测试页面没有显示")[0]
        time.sleep(0.2)
        left, top = window["x"] + 80, window["y"] + 80
        right, bottom = left + 320, top + 320
        source = session.wait(lambda: page_frame(session, (left, top, right, bottom), texture(320, 320)), "测试页面首帧没有呈现")
        source.save(session.root / "source.png")
        pid = session.launch(binary, ["-S", "--speed", "--to", "c"], overrides=overrides)
        time.sleep(0.8)
        session.drag((left, top), (right, bottom))
        session.wait(lambda: controls(session.screenshot("scroll-controls")), "滚动截图面板没有显示")
        time.sleep(0.4)
        for index in range(4):
            if hidden_controls:
                # 1. 【虚拟机测试】【普通窗口避让】等待静止暂停后恢复；真实滚动紧接恢复，避免观察操作超过一秒暂停期限
                time.sleep(1.1)
                buttons = controls(session.screenshot(f"before-resume-{index}"))
                assert buttons, "静止暂停后没有恢复控制窗口"
                if buttons["paused"]:
                    session.click(*buttons["pause"])
            session.move(left + 100, top + 100)
            session.scroll()
            expected_position = f"0 {(index + 1) * 48}"
            session.wait(lambda: session.guest("p=Path(values);print(p.read_text().strip() if p.exists() else '')", session.remote + "/position.txt") == expected_position, "滚轮没有传递到测试页面")
            time.sleep(0.3)
            with Image.open(session.screenshot(f"scroll-{index}")) as frame:
                area = frame.convert("RGB").crop((left, top, right, bottom))
            assert ImageChops.difference(area, source).getbbox(), "页面位置变化但画面没有更新"
        if hidden_controls:
            time.sleep(1.1)
        buttons = controls(session.screenshot("scroll-completed"))
        assert buttons, "滚动结束后没有可操作的控制窗口"
        if not buttons["paused"]:
            session.click(*buttons["pause"])
        session.click(*buttons["copy"])
        session.wait(lambda: not session.alive(pid), "长图复制后程序没有退出")
        actual_path = session.clipboard()
        expected = texture(320, 512)
        expected.save(session.root / "expected.png")
        with Image.open(actual_path) as result:
            actual = result.convert("RGB")
            assert actual.size == expected.size and not ImageChops.difference(actual, expected).getbbox(), f"长图像素不一致: {actual.size}"
        return {"page_received_wheel": True, "continuous_stitching": True, "pause_copy": True,
                "stitched_size": [320, 512], "exact_pixels": True}
    finally:
        if pid:
            session.stop(pid)
            session.log("capture")
        session.stop(page)
        session.log("page")
        session.cleanup()
