"""【原图回归】【像素校验】比较原始图像的颜色与透明度，不以透明差异通道代替完整像素"""

import re

from PIL import ImageChops


def equal_pixels(actual, expected):
    """【原图回归】【完整像素】比较 actual、expected 的尺寸、RGB 和透明度，返回是否相同，不改变输入图片"""
    return actual.size == expected.size and ImageChops.difference(actual.convert("RGBA"), expected.convert("RGBA")).getbbox(alpha_only=False) is None


def keyboard_release_grab(text, code=None):
    """【原图回归】【抓取来源】从 text 验证同设备按下、释放与抓取，code 可限定原始键码，返回配对序号，不假定虚拟键盘的动态键码"""
    keys = re.findall(r"wl_keyboard@(\d+)\.key, \((\d+), \d+, (\d+), ([01])\)", text)
    grabs = re.findall(r"xdg_popup@\d+\.grab\(wl_seat@\d+, (\d+)\)", text)
    if (
        len(keys) != 2
        or keys[0][3] != "1"
        or keys[1][3] != "0"
        or len(grabs) != 1
        or keys[0][0] != keys[1][0]
        or keys[0][2] != keys[1][2]
        or (code is not None and keys[0][2] != str(code))
    ):
        raise AssertionError("popup has no single real press/release transaction")
    if grabs[0] != keys[0][1] or grabs[0] == keys[1][1]:
        raise AssertionError("popup grab did not consume its matching press serial")
    return {"press": int(keys[0][1]), "release": int(keys[1][1]), "grab": int(grabs[0]), "code": int(keys[0][2])}
