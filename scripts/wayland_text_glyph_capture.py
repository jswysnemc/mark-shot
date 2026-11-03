"""【文字格式回归】【相位分离】从两次真实截图剔除窄红色光标，字形恢复仍执行完整单像素断言"""

import time

from PIL import Image, ImageChops
import wayland_text_format_session as session


def without_caret(first, second):
    """【文字格式回归】【光标分离】比较 first、second；只有窄红色光标差异可以选择较暗的完整原始帧，其他变化失败，不拼接通道，返回真实画面"""
    bounds = ImageChops.difference(first, second).getbbox()
    if bounds is None:
        return first
    if bounds[2] - bounds[0] > 3 or not 10 <= bounds[3] - bounds[1] <= 80:
        raise AssertionError(f"document changed while isolating its caret phase: {bounds}")
    y = (bounds[1] + bounds[3]) // 2
    colors = max(((first.getpixel((x, y)), second.getpixel((x, y))) for x in range(bounds[0], bounds[2])),
                 key=lambda pair: max(pixel[0] for pixel in pair))
    if not any(red > 100 and red > green * 1.7 and red > blue * 1.7 for red, green, blue in colors):
        raise AssertionError(f"the changing narrow area is not the annotation's red caret: {colors}")
    score = lambda image: sum(image.crop(bounds).getchannel("R").tobytes())
    return first if score(first) <= score(second) else second


def capture(process, root, name):
    """【文字格式回归】【实际双帧】保存 process 在 root 的 name 两个半周期相位，返回不含光标的字形；先保留原始画面，再严格检查变化范围"""
    if name == "initial":
        # 1. 【文字格式回归】【启动稳定】初始画面没有文字光标，先等待字体线程和工具栏入场结束，不把启动动画当成字形差异
        session.wait_single_thread(process.pid)
        time.sleep(0.3)
    time.sleep(0.12)
    images = []
    for index in range(2):
        if index:
            time.sleep(0.52)
        if process.poll() is not None:
            raise AssertionError(f"editor exited before glyph capture: {process.returncode}")
        path = root / f"{name}-phase-{index}.png"
        session.screenshot(path)
        with Image.open(path) as image:
            images.append(image.convert("RGB"))
    result = without_caret(*images)
    result.save(root / f"{name}.png")
    return result
