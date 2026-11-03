"""【滤镜测试】【结果核对】验证原图逐像素输出、图片剪贴板、历史一致性和严格空闲资源"""

import io
import json
import subprocess
import time

import numpy as np
from PIL import Image, ImageChops

from wayland_editor_locator import filter_controls
from wayland_filter_fixture import BORDER
from wayland_filter_reference import assert_region, expected_export
import wayland_test_session as session


def alive(process):
    """【滤镜测试】【进程校验】确认 process 没有提前退出；无返回值，异常状态包含退出码"""
    if process.poll() is not None:
        raise RuntimeError(f"the filter editor unexpectedly exited: {process.returncode}")


def capture(path):
    """【滤镜测试】【界面取样】把私有桌面截图写入 path，返回独立 RGB 图像"""
    session.screenshot(path)
    with Image.open(path) as image:
        return image.convert("RGB")


def quiet(process, path, layout):
    """【滤镜测试】【稳定界面】把 process 指针移到 layout 安全位置，返回稳定截图并写入 path"""
    session.control("move", "--logical", 5, layout["safe_y"])
    time.sleep(0.3)
    alive(process)
    return capture(path)


def ready(process, path):
    """【滤镜测试】【启动等待】等待 process 源图标记和滤镜属性框出现；返回写入 path 的稳定界面"""
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        alive(process)
        image = capture(path)
        if image.getpixel((image.width // 2 - 110, image.height // 2)) == (30, 34, 38):
            try:
                filter_controls(image, BORDER)
            except ValueError:
                time.sleep(0.04)
                continue
            session.wait_single_thread(process.pid)
            time.sleep(0.3)
            return capture(path)
        time.sleep(0.04)
    raise RuntimeError("the original filter source and its settings panel did not appear")


def difference(first, second, region):
    """【滤镜测试】【区域比较】返回 first、second 在 region 内的差异矩形，没有差异时返回空值"""
    return ImageChops.difference(first.crop(region), second.crop(region)).getbbox()


def history(process, artifacts, layout, name, before, changed):
    """【滤镜测试】【事务核对】在 process 中撤销、重做一次，分别与 before、changed 完全比较；layout、artifacts、name 提供证据范围"""
    if not difference(before, changed, layout["content"]):
        raise AssertionError(f"the {name} transaction did not visibly change the selected filter")
    session.key("ctrl+z")
    undone = quiet(process, artifacts / f"{name}-undo.png", layout)
    if difference(before, undone, layout["content"]):
        raise AssertionError(f"one undo did not exactly restore the {name} transaction")
    session.key("ctrl+shift+z")
    redone = quiet(process, artifacts / f"{name}-redo.png", layout)
    if difference(changed, redone, layout["content"]):
        raise AssertionError(f"one redo did not exactly restore the {name} transaction")


def idle(process, artifacts, name):
    """【滤镜测试】【严格空闲】等待 process 单线程且稳定一秒，再采样一秒；返回并写入 artifacts 的 name 资源记录"""
    session.wait_single_thread(process.pid)
    time.sleep(1)
    before = session.resources(process.pid)
    time.sleep(1)
    after = session.resources(process.pid)
    if before["threads"] != 1 or after["threads"] != 1 or before["cpu_ticks"] != after["cpu_ticks"]:
        raise AssertionError(f"the stable filter editor retained work: {before} -> {after}")
    result = {"before": before, "after": after, "settle_seconds": 1, "sample_seconds": 1}
    (artifacts / f"{name}-resources.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def export(process, environment, destination, source, annotations, artifacts, name):
    """【滤镜测试】【原图导出】确认 process 保存并退出；按 source、annotations 独立核对 destination 和 environment 剪贴板，证据写入 artifacts 的 name 文件"""
    session.key("Return")
    process.wait(timeout=5)
    if process.returncode:
        raise RuntimeError(f"filter export failed: {process.returncode}")
    files = list(destination.rglob("*.png"))
    if len(files) != 1:
        raise RuntimeError(f"expected one saved filter image, received {len(files)}")
    encoded = files[0].read_bytes()
    clipboard = subprocess.check_output(["wl-paste", "--type", "image/png"], env=environment, timeout=5)
    (artifacts / f"{name}-image.png").write_bytes(encoded)
    (artifacts / f"{name}-clipboard.png").write_bytes(clipboard)
    with Image.open(io.BytesIO(encoded)) as saved, Image.open(io.BytesIO(clipboard)) as copied:
        output = saved.convert("RGBA")
        if output.size != source.size or copied.convert("RGBA").tobytes() != output.tobytes():
            raise AssertionError("filter save and persistent clipboard differ or lost original resolution")

    # 1. 【滤镜测试】【原图导出】独立数组实现逐区域处理冻结原图，覆盖顺序只作用于输出，不把先前标注作为采样源
    (artifacts / f"{name}-expected-annotations.json").write_text(json.dumps(annotations, indent=2) + "\n")
    expected = expected_export(source, annotations)
    expected.save(artifacts / f"{name}-expected.png")
    assert_region(output, expected, (0, 0, source.width, source.height))
    untouched = np.ones((source.height, source.width), dtype=bool)
    for annotation in annotations:
        left, top, right, bottom = annotation["native"]
        untouched[top:bottom, left:right] = False
    actual = np.asarray(output)
    original = np.asarray(source)
    if not np.array_equal(actual[untouched], original[untouched]):
        bad = untouched & np.any(actual != original, axis=2)
        y, x = np.argwhere(bad)[0]
        raise AssertionError(f"an untouched native pixel changed: {(int(x), int(y))}")
    report = {
        "size": output.size,
        "compared_pixels": source.width * source.height,
        "rgb_encoding_tolerance": 1,
        "alpha_tolerance": 0,
        "untouched_pixels": int(np.count_nonzero(untouched)),
        "unchanged_untouched_rgba": True,
        "clipboard_matches_after_exit": True,
        "annotations": annotations,
    }
    (artifacts / f"{name}-pixels.json").write_text(json.dumps(report, indent=2) + "\n")
    return output
