"""【截图历史测试】【独立断言】PNG 像素、文件身份和空闲资源校验"""

import io
from pathlib import Path
import re
import stat
import struct
import subprocess
import time
from datetime import datetime

import numpy as np
from PIL import Image

import wayland_test_session as session


NAME = re.compile(r"capture-([0-9]{17})-[0-9a-f]{32}\.png\Z")
DPR = "mark-shot-device-pixel-ratio"
SEARCH = "mark-shot-history-search"


def owned(directory):
    """【截图历史测试】【文件枚举】返回 directory 中合法单链接普通历史文件，按新到旧排序"""
    if not directory.exists():
        return []
    result = []
    for path in directory.iterdir():
        match = NAME.fullmatch(path.name)
        if not match or path.is_symlink() or not path.is_file() or path.stat().st_nlink != 1:
            continue
        try:
            datetime.strptime(match[1], "%Y%m%d%H%M%S%f")
        except ValueError:
            continue
        result.append(path)
    return sorted(result, reverse=True)


def wait_count(directory, count, timeout=5):
    """【截图历史测试】【数量等待】等待 directory 中记录达到 count；返回记录列表，超时抛出异常"""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        files = owned(directory)
        if len(files) == count:
            return files
        time.sleep(0.04)
    raise AssertionError(f"history count: expected {count}, found {len(owned(directory))}")


def rgba(value):
    """【截图历史测试】【独立解码】解码 value 的 PNG 字节或路径，返回完整 RGBA 数组"""
    source = io.BytesIO(value) if isinstance(value, bytes) else value
    with Image.open(source) as image:
        if image.format != "PNG":
            raise AssertionError("the output is not PNG")
        return np.asarray(image.convert("RGBA")).copy()


def equal_image(actual, expected):
    """【截图历史测试】【像素比较】逐像素比较 actual、expected 的 RGBA；尺寸和透明度都必须完全一致"""
    left, right = rgba(actual), rgba(expected)
    if left.shape != right.shape:
        raise AssertionError(f"image shape changed: {left.shape} != {right.shape}")
    different = np.any(left != right, axis=2)
    if np.any(different):
        raise AssertionError(f"{np.count_nonzero(different)} RGBA pixels differ")


def idat(value):
    """【截图历史测试】【压缩数据】返回 value 的全部 IDAT 数据块；独立检查长度，拒绝截断文件"""
    data = value if isinstance(value, bytes) else Path(value).read_bytes()
    offset, result = 8, bytearray()
    while offset < len(data):
        if offset + 12 > len(data):
            raise ValueError("truncated PNG chunk")
        length = struct.unpack_from(">I", data, offset)[0]
        end = offset + length + 12
        if end > len(data):
            raise ValueError("truncated PNG data")
        if data[offset + 4:offset + 8] == b"IDAT":
            result.extend(data[offset:end])
        offset = end
    return bytes(result)


def clipboard(environment, expected, artifact):
    """【截图历史测试】【图片剪贴板】等待 environment 中图片等于 expected；保存实际数据到 artifact 并返回字节"""
    deadline = time.monotonic() + 5
    error = None
    while time.monotonic() < deadline:
        process = subprocess.run(["wl-paste", "--no-newline", "--type", "image/png"], env=environment, capture_output=True)
        if process.returncode == 0:
            try:
                equal_image(process.stdout, expected)
                artifact.write_bytes(process.stdout)
                return process.stdout
            except (AssertionError, OSError) as caught:
                error = caught
        time.sleep(0.04)
    raise AssertionError(f"image clipboard did not match: {error}")


def private_files(directory):
    """【截图历史测试】【权限和排序】校验 directory 的权限、记录权限与严格毫秒排序；返回记录数量"""
    if stat.S_IMODE(directory.stat().st_mode) != 0o700:
        raise AssertionError("history directory is not private")
    files = owned(directory)
    timestamps = [NAME.fullmatch(path.name)[1] for path in files]
    if len(timestamps) != len(set(timestamps)):
        raise AssertionError("history timestamps collide at millisecond precision")
    for path in files:
        if stat.S_IMODE(path.stat().st_mode) != 0o600:
            raise AssertionError("history file is not private")
        with Image.open(path) as image:
            scale = float(image.info.get(DPR, 0))
            if not 1 <= scale <= 16:
                raise AssertionError(f"invalid stored scale: {scale}")
    return len(files)


def idle(pid):
    """【截图历史测试】【空闲断言】等待 pid 的一次性线程退出，稳定一秒后检查一秒 CPU 增量；返回采样"""
    session.wait_single_thread(pid, timeout=15)
    time.sleep(1)
    before = session.resources(pid)
    time.sleep(1)
    after = session.resources(pid)
    if after["threads"] != 1 or after["cpu_ticks"] != before["cpu_ticks"]:
        raise AssertionError(f"history retained idle work: {before} -> {after}")
    return {"before": before, "after": after}
