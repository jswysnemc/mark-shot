"""【Windows文本贴图】【像素断言】验证文本生成图像、主题往返及父子窗口生命周期"""

import subprocess
import time
import tomllib

from PIL import Image
from windows_ocr_actions import key, path_w, rectangle, set_clipboard, snapshot, visible


def copy_image(probe, environment, root, identity, name):
    """【Windows文本贴图】【系统图像】复制identity并通过probe读取PNG到root/name，返回RGBA图像，不依赖窗口截图分辨率"""
    key(environment, identity, "ctrl+c")
    path = root / (name + ".png")
    subprocess.run(["wine", probe, "image", path_w(path, environment)], env=environment, cwd=root, check=True, timeout=8)
    return Image.open(path).convert("RGBA")


def wait_mode(root, mode):
    """【Windows文本贴图】【主题确认】等待root配置实际保存mode，超时失败，无返回值，不重复发送快捷键"""
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if tomllib.loads((root / "config.toml").read_text())["ui"]["theme"] == mode:
            return
        time.sleep(0.03)
    raise AssertionError(f"text pin did not save {mode} theme")


def paste_text(binary, probe, environment, root, parent, text, name):
    """【Windows文本贴图】【独立文本窗口】将text经系统剪贴板粘贴为新窗口，验证可见字形、主题不改变原件及父窗保留，返回证据"""
    set_clipboard(binary, environment, root, text)
    key(environment, parent, "ctrl+v")
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        windows = visible(environment, "Pinned Mark Shot")
        if len(windows) == 2 and parent in windows:
            child = next(value for value in windows if value != parent)
            break
        time.sleep(0.05)
    else:
        raise AssertionError("text clipboard did not create an independent pin")
    original = copy_image(probe, environment, root, child, name)
    bounds = rectangle(environment, child)
    assert original.size == (bounds["width"], bounds["height"])
    assert original.height == 32 + 24 * len(text.split("\n")), "line count changed"
    rows = []
    for index, line in enumerate(text.split("\n")):
        pixels = original.crop((16, 16 + 24 * index, original.width - 16, 40 + 24 * index)).convert("RGB")
        count = sum(max(pixel) < 150 for pixel in pixels.get_flattened_data())
        assert count >= 15 if line else count == 0, f"text row {index} has no visible content or lost its blank line"
        rows.append(count)
    snapshot(environment, root, name + "-window")
    # 1. 【Windows文本贴图】【原件冻结】新图像创建后主题只改变窗口控件，不重新着色已生成的文字图像
    key(environment, child, "ctrl+shift+l")
    wait_mode(root, "dark")
    dark = copy_image(probe, environment, root, child, name + "-dark-original")
    assert dark.tobytes() == original.tobytes() and dark.size == original.size
    key(environment, child, "ctrl+shift+l")
    wait_mode(root, "light")
    restored = copy_image(probe, environment, root, child, name + "-restored-original")
    assert restored.size == original.size and restored.tobytes() == original.tobytes()
    key(environment, child, "Escape")
    deadline = time.monotonic() + 6
    while visible(environment, "Pinned Mark Shot") != [parent]:
        assert time.monotonic() < deadline, "text pin did not close independently"
        time.sleep(0.04)
    return {"size": original.size, "row_ink_pixels": rows, "theme_preserves_image": True, "parent_preserved": True}
