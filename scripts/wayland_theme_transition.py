"""【主题回归】【呈现等待】配置保存与原生呈现分别验证，不把写入完成当作画面完成"""

from pathlib import Path
import tempfile
import time
import tomllib

from PIL import Image


def toggle(config, expected, key, screenshot):
    """【主题回归】【实际转换】用key切换config至expected，screenshot只读取私有画面，返回达到可见主题变化所需秒数"""
    with tempfile.TemporaryDirectory(prefix="mark-shot-theme-transition-") as temporary:
        path = Path(temporary) / "frame.png"
        screenshot(path)
        before = Image.open(path).convert("RGB")
        key("ctrl+shift+l")
        start = time.monotonic()
        deadline = start + 6
        while time.monotonic() < deadline:
            if tomllib.loads(config.read_text()).get("ui", {}).get("theme") == expected:
                screenshot(path)
                after = Image.open(path).convert("RGB")
                count = sum((sum(first) - sum(second) if expected == "dark" else sum(second) - sum(first)) > 300
                            for first, second in zip(before.get_flattened_data(), after.get_flattened_data()))
                if count >= 500:
                    return time.monotonic() - start
            time.sleep(0.03)
    raise AssertionError(f"saved {expected} theme did not appear in the actual private output")
