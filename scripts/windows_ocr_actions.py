"""【Windows识别回归】【私有操作】只操作运行器指定的Wine与Xvfb窗口，保存实际系统结果"""

import hashlib
import subprocess
import time

from PIL import ImageGrab


def xdo(environment, *arguments):
    """【Windows识别回归】【窗口输入】按environment与arguments操作私有X11，返回输出，不使用默认显示"""
    assert environment.get("MARK_SHOT_PRIVATE_WINDOWS_TEST") == "1"
    return subprocess.check_output(["xdotool", *map(str, arguments)], env=environment, text=True, timeout=8).strip()


def path_w(path, environment):
    """【Windows识别回归】【路径转换】按path与environment返回本次前缀中的Windows路径"""
    return subprocess.check_output(["winepath", "-w", str(path)], env=environment, text=True, timeout=8).strip()


def key(environment, identity, value):
    """【Windows识别回归】【键盘事件】聚焦identity并发送value指定的实际快捷键，无返回值"""
    xdo(environment, "windowfocus", "--sync", identity)
    xdo(environment, "key", "--clearmodifiers", value)


def click(environment, identity, x, y, button=1):
    """【Windows识别回归】【指针事件】在identity客户区坐标x与y点击button，无返回值"""
    xdo(environment, "windowfocus", "--sync", identity)
    xdo(environment, "mousemove", "--sync", "--window", identity, x, y)
    xdo(environment, "click", button)


def snapshot(environment, root, name):
    """【Windows识别回归】【画面证据】按environment读取私有显示，保存root中的name并返回RGB图像"""
    image = ImageGrab.grab(xdisplay=environment["DISPLAY"]).convert("RGB")
    image.save(root / (name + ".png"))
    return image


def visible(environment, title):
    """【Windows识别回归】【窗口查询】按title返回私有显示中完全匹配的可见窗口列表"""
    try:
        return xdo(environment, "search", "--onlyvisible", "--name", "^" + title + "$").splitlines()
    except subprocess.CalledProcessError:
        return []


def window(environment, title, root, process=None, timeout=30):
    """【Windows识别回归】【窗口等待】等待title唯一可见窗口并返回身份，process提前退出或timeout到期保存画面后失败"""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        values = visible(environment, title)
        if len(values) == 1:
            return values[0]
        if process is not None and process.poll() is not None:
            snapshot(environment, root, "failed-window")
            raise AssertionError(f"process exited before {title}: {process.returncode}")
        time.sleep(0.05)
    snapshot(environment, root, "failed-window")
    raise AssertionError(f"window did not become unique: {title}")


def rectangle(environment, identity):
    """【Windows识别回归】【客户区几何】按identity读取实际X11窗口尺寸，返回坐标字典，不推测居中位置"""
    values = dict(line.split("=", 1) for line in xdo(environment, "getwindowgeometry", "--shell", identity).splitlines())
    return {key: int(values[key.upper()]) for key in ("x", "y", "width", "height")}


def clipboard_text(probe, environment, root, name):
    """【Windows识别回归】【文字接收】用probe读取原生Windows剪贴板并保存root中的name，返回完整UTF8正文"""
    path = root / (name + ".txt")
    subprocess.run(["wine", probe, "text", path_w(path, environment)], env=environment, cwd=root, check=True, timeout=8)
    return path.read_text()


def set_clipboard(binary, environment, root, text):
    """【Windows识别回归】【文字提供】用同一binary把text交给Windows系统剪贴板，等待提供者退出，返回实际耗时"""
    started = time.monotonic()
    with (root / "clipboard-provider.log").open("ab") as log:
        subprocess.run(["wine", binary, "--clipboard-daemon", "text"], input=text.encode(), env=environment,
                       cwd=root, stdout=log, stderr=log, check=True, timeout=20)
    return time.monotonic() - started


def copy_all(probe, environment, root, identity, name, expected=None):
    """【Windows识别回归】【全文检查】在identity实际全选复制，用probe读取并保存name，expected存在时逐字符比较，返回正文"""
    key(environment, identity, "ctrl+a")
    key(environment, identity, "ctrl+c")
    text = clipboard_text(probe, environment, root, name)
    if expected is not None:
        assert text == expected, f"{name} differs: {text!r} != {expected!r}"
    return text


def model_assets(root):
    """【Windows识别回归】【模型清单】按root查找私有用户模型目录，返回文件大小、SHA256和位置，不访问宿主模型"""
    users = root / "wine-prefix/drive_c/users"
    result = {}
    for path in sorted(users.glob("*/AppData/Roaming/mark-shot/models/*")):
        if path.is_file():
            assert path.name not in result, "multiple Windows user model directories"
            result[path.name] = {"bytes": path.stat().st_size, "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                                 "path": str(path.relative_to(root))}
    return result


def stop(process):
    """【Windows识别回归】【进程回收】按process回收本次仍存活的程序，无返回值，不终止其他Wine前缀"""
    if process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=3)
