"""【箭头测试】【验证辅助】检查真实像素、源图导出、剪贴板和隔离编辑器的空闲资源"""

import io
import json
import math
import subprocess
import time

from PIL import Image, ImageChops

import wayland_test_session as session


BORDER = (96, 113, 127)
HEADS = ["fletched", "kde", "bidirectional_fletched", "bidirectional_kde"]
PATTERNS = ["solid", "dashed", "dotted"]


def alive(process):
    """【箭头测试】【进程检查】确认 process 仍在编辑；提前退出时抛出异常，无返回值"""
    if process.poll() is not None:
        raise RuntimeError(f"the arrow editor unexpectedly exited: {process.returncode}")


def capture(path):
    """【箭头测试】【界面截图】把私有桌面写入 path，返回用于比较的 RGB 图像"""
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def quiet(process, path, layout):
    """【箭头测试】【悬停隔离】把 process 指针移到 layout 安全位置，返回稳定截图并写入 path"""
    session.control("move", "--logical", 5, layout["safe_y"])
    time.sleep(0.3)
    alive(process)
    return capture(path)


def ready(process, path):
    """【箭头测试】【启动等待】等待 process 显示源图并结束启动线程，证据写入 path；返回稳定界面"""
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        alive(process)
        image = capture(path)
        if image.getpixel((image.width // 2 - 110, image.height // 2)) == (30, 34, 38):
            session.wait_single_thread(process.pid)
            time.sleep(0.3)
            return capture(path)
        time.sleep(0.04)
    raise RuntimeError("the arrow source image did not appear")


def difference(first, second, region):
    """【箭头测试】【区域比较】返回 first、second 在 region 内的差异矩形，无差异时返回空值"""
    return ImageChops.difference(first.crop(region), second.crop(region)).getbbox()


def red_pixel(pixel):
    """【箭头测试】【笔迹判断】判断 pixel 是否包含半透明红色笔迹；返回布尔值，排除背景和控制框"""
    return pixel[0] - pixel[1] > 35 and pixel[0] - pixel[2] > 35


def red_bounds(image, region):
    """【箭头测试】【正文边界】返回 image 在 region 内红色笔迹的最小矩形，无正文时抛出异常"""
    left, top, right, bottom = region
    points = [(x, y) for y in range(top, bottom) for x in range(left, right) if red_pixel(image.getpixel((x, y)))]
    if not points:
        raise RuntimeError(f"the expected annotation has no visible red pixels: {region}")
    return min(x for x, _ in points), min(y for _, y in points), max(x for x, _ in points) + 1, max(y for _, y in points) + 1


def red_near(image, point, radius=6):
    """【箭头测试】【定点正文】判断 image 在 point 的 radius 邻域内是否存在红色正文；返回布尔值"""
    x, y = map(round, point)
    return any(red_pixel(image.getpixel((px, py))) for py in range(max(0, y - radius), min(image.height, y + radius + 1)) for px in range(max(0, x - radius), min(image.width, x + radius + 1)))


def check_shaft(image, start, end, pattern, margin=40):
    """【箭头测试】【线型像素】用 image 的 start、end 中部验证 pattern；margin 排除头部，失败时抛出异常"""
    row = round(start[1])
    mask = [red_pixel(image.getpixel((x, row))) for x in range(round(start[0] + margin), round(end[0] - margin))]
    if len(mask) < 20 or not any(mask):
        raise RuntimeError(f"the {pattern} shaft has no useful pixels")
    pieces = sum(value and (index == 0 or not mask[index - 1]) for index, value in enumerate(mask))
    if pattern == "solid" and sum(mask) < len(mask) * 0.95:
        raise RuntimeError(f"the solid shaft contains unintended gaps: {sum(mask)}/{len(mask)}")
    if pattern != "solid" and (all(mask) or pieces < 2):
        raise RuntimeError(f"the {pattern} shaft did not preserve separate pieces: {pieces}")


def check_alpha(image, region):
    """【箭头测试】【半透明上限】在 image 的 region 中检查半透明笔迹不会重复覆盖接缝；无返回值"""
    maximum = image.crop(region).getchannel("R").getextrema()[1]
    if maximum > 146:
        raise RuntimeError(f"a half-transparent arrow was painted more than once: red={maximum}")


def grid_point(width, height, x, y):
    """【箭头测试】【网格坐标】将 width、height 中的 x、y 比例转为五像素网格位置；返回逻辑坐标"""
    return round(width * x / 5) * 5, round(height * y / 5) * 5


def geometry(environment, source):
    """【箭头测试】【界面布局】根据 environment 输出及 source 原图计算矩阵、独立编辑区和视口；返回逻辑坐标字典"""
    outputs = json.loads(subprocess.check_output(["niri", "msg", "-j", "outputs"], env=environment))
    if len(outputs) != 1:
        raise RuntimeError("the arrow regression requires one unambiguous output")
    logical = next(iter(outputs.values()))["logical"]
    width, height = logical["width"], logical["height"]
    scale = min((width - 64) / source.width, (height - 96) / source.height, 1)
    left, top = (width - source.width * scale) / 2, (height - source.height * scale) / 2
    matrix = []
    for head in range(4):
        for pattern in range(3):
            start = grid_point(width, height, 0.16 + pattern * 0.28, 0.25 + head * 0.115)
            end = (start[0] + round(width * 0.17 / 5) * 5, start[1])
            curve_start, curve_end = (start[0], start[1] + 30), (end[0], end[1] + 30)
            control = (round((start[0] + end[0]) / 2), curve_start[1] + 22)
            matrix.append({"head": head, "pattern": pattern, "start": start, "end": end, "curve_start": curve_start, "curve_end": curve_end, "control": control, "region": (start[0] - 18, start[1] - 18, end[0] + 18, curve_start[1] + 30)})
    edit_start = grid_point(width, height, 0.20, 0.80)
    edit_end = grid_point(width, height, 0.42, 0.80)
    edit_control = (round((edit_start[0] + edit_end[0]) / 10) * 5, edit_start[1] - 80)
    lines = []
    for index in range(3):
        start = grid_point(width, height, 0.67, 0.79 + index * 0.04)
        lines.append((start, (start[0] + round(width * 0.18 / 5) * 5, start[1])))
    return {"matrix": matrix, "edit_start": edit_start, "edit_end": edit_end, "edit_control": edit_control, "second_control": (edit_end[0] - 20, edit_start[1] + 35), "edit_region": (edit_start[0] - 60, edit_start[1] - 60, edit_end[0] + 140, math.floor(top + source.height * scale) - 2), "lines": lines, "content": (50, math.ceil(top) + 90, width - 50, math.floor(top + source.height * scale) - 2), "safe_y": height - 6, "viewport": (left, top, scale), "cancel_start": grid_point(width, height, 0.55, 0.81), "cancel_end": grid_point(width, height, 0.60, 0.88), "reload_start": grid_point(width, height, 0.24, 0.42), "reload_end": grid_point(width, height, 0.52, 0.42), "reload_line_start": grid_point(width, height, 0.24, 0.64), "reload_line_end": grid_point(width, height, 0.52, 0.64)}


def curve_point(start, control, end, progress):
    """【箭头测试】【二次曲线位置】用 start、control、end 和 progress 计算参考曲线位置；返回逻辑坐标"""
    before = 1 - progress
    return tuple(before * before * start[index] + 2 * before * progress * control[index] + progress * progress * end[index] for index in (0, 1))


def rotated(point, center, degrees):
    """【箭头测试】【旋转参考】把 point 绕 center 旋转 degrees 度，返回参考位置"""
    angle = math.radians(degrees)
    dx, dy = point[0] - center[0], point[1] - center[1]
    return center[0] + dx * math.cos(angle) - dy * math.sin(angle), center[1] + dx * math.sin(angle) + dy * math.cos(angle)


def idle(process, artifacts, name):
    """【箭头测试】【空闲采样】等待 process 单线程并稳定一秒，再采样一秒；将 name 资源证据写入 artifacts 并返回快照"""
    session.wait_single_thread(process.pid)
    time.sleep(1)
    before = session.resources(process.pid)
    time.sleep(1)
    after = session.resources(process.pid)
    if after["threads"] != 1 or before["cpu_ticks"] != after["cpu_ticks"]:
        raise RuntimeError(f"the stable arrow editor keeps polling: {before} -> {after}")
    result = {"before": before, "after": after}
    (artifacts / f"{name}-resources.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def export(process, environment, destination, source, artifacts, name):
    """【箭头测试】【原图导出】确认 process 保存到 destination 并退出；用 environment 比较 source 原图与剪贴板，证据写入 artifacts 的 name 文件"""
    session.key("Return")
    process.wait(timeout=5)
    if process.returncode:
        raise RuntimeError(f"arrow export failed: {process.returncode}")
    files = list(destination.rglob("*.png"))
    if len(files) != 1:
        raise RuntimeError(f"expected one saved arrow image, received {len(files)}")
    encoded = files[0].read_bytes()
    clipboard = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment, timeout=5)
    with Image.open(io.BytesIO(encoded)) as saved, Image.open(io.BytesIO(clipboard)) as copied:
        output = saved.convert("RGBA")
        if output.size != source.size or copied.convert("RGBA").tobytes() != output.tobytes():
            raise RuntimeError("arrow save and persistent clipboard differ or lost original resolution")
        for region in [(0, 0, 1600, 8), (20, 20, 43, 43), (1470, 800, 1580, 890)]:
            if output.crop(region).tobytes() != source.crop(region).tobytes():
                raise RuntimeError("arrow export changed untouched pixels or original transparency")
    (artifacts / f"{name}-image.png").write_bytes(encoded)
    (artifacts / f"{name}-clipboard.png").write_bytes(clipboard)
    return output
