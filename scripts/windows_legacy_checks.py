"""【Windows旧主题】【窗口证据】独立配置启动同一图像编辑器，比较稳定帧与原始剪贴板输出"""

import subprocess
import time

from PIL import Image, ImageChops
from windows_ocr_actions import key, path_w, snapshot, stop, window, xdo
from windows_pin_checks import absent


LEGACY = ('[theme]\nbackground="#1E2124FA"\nforeground="#E5EDEF"\nmuted="#A2B1B4"\n'
          'hover="#B4BEFE"\naccent="#4BBF9B"\non_accent="#1E2124"\n'
          '[toolbar]\nbackground="#1E2124FA"\nicon="#E5EDEF"\n')


def case(binary, probe, environment, root, log, source, mode, legacy):
    """【Windows旧主题】【独立启动】按mode与legacy构造新配置，用binary启动source并输出原件，返回稳定像素与配置保持结果"""
    root.mkdir()
    config = root / "config.toml"
    text = ('[general]\nto="c"\n[notifications]\nenabled=false\n[history]\nenabled=false\n'
            '[ui]\nlanguage="en"\n' + (f'theme="{mode}"\n' if mode else '') + (LEGACY if legacy else ''))
    config.write_text(text)
    process = subprocess.Popen(["wine", binary, "--config", path_w(config, environment), "--fullscreen", path_w(source, environment)],
                               env=environment, cwd=root, stdout=log, stderr=log)
    try:
        identity = window(environment, "Mark Shot", root, process)
        xdo(environment, "mousemove", 1100, 700)
        time.sleep(0.4)
        before = snapshot(environment, root, "first")
        for index in range(20):
            time.sleep(0.1)
            current = snapshot(environment, root, "stable")
            if ImageChops.difference(before, current).getbbox() is None:
                break
            before = current
        else:
            raise AssertionError("legacy configuration window did not stabilize")
        key(environment, identity, "ctrl+c")
        assert process.wait(timeout=8) == 0
        absent(environment, "Mark Shot")
        target = root / "export.png"
        subprocess.run(["wine", probe, "image", path_w(target, environment)], env=environment, cwd=root, check=True, timeout=8)
        expected = Image.open(source).convert("RGBA")
        actual = Image.open(target).convert("RGBA")
        assert actual.size == expected.size and actual.tobytes() == expected.tobytes(), "theme changed source export"
        assert config.read_text() == text, "startup modified unrelated legacy configuration"
        return current
    finally:
        stop(process)
