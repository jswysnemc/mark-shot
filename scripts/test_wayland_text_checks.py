"""【文字校验测试】【边缘隔离】保证圆角容许值不能掩盖正文区域泄漏"""

import unittest

from PIL import Image
from wayland_text_checks import transparent_text_is_absent


class TransparentTextChecks(unittest.TestCase):
    """【文字校验测试】【透明背景】覆盖纯背景、圆角量化、正文泄漏与角落错误颜色"""

    def setUp(self):
        """【文字校验测试】【样例构造】创建确定 RGB 背景和检测范围，无参数、无返回值"""
        self.source = (30, 34, 38)
        self.image = Image.new("RGB", (40, 30), self.source)
        self.bounds = (4, 4, 36, 26)

    def test_plain_background_passes(self):
        """【文字校验测试】【纯背景】无字形与圆角量化时通过，返回 None"""
        self.assertTrue(transparent_text_is_absent(self.image, self.bounds, self.source))

    def test_only_corner_quantization_passes(self):
        """【文字校验测试】【圆角量化】四角各通道加一符合已测背景绘制边缘，返回 None"""
        for point in ((4, 4), (35, 4), (4, 25), (35, 25)):
            self.image.putpixel(point, (31, 35, 39))
        self.assertTrue(transparent_text_is_absent(self.image, self.bounds, self.source))

    def test_same_quantization_in_text_area_fails(self):
        """【文字校验测试】【正文隔离】正文区域出现相同量化值仍失败，返回 None"""
        self.image.putpixel((20, 15), (31, 35, 39))
        self.assertFalse(transparent_text_is_absent(self.image, self.bounds, self.source))

    def test_red_glyphs_fail_even_at_corners(self):
        """【文字校验测试】【字形泄漏】圆角内也不能出现文字颜色，返回 None"""
        self.image.putpixel((4, 4), (255, 77, 77))
        self.assertFalse(transparent_text_is_absent(self.image, self.bounds, self.source))


if __name__ == "__main__":
    unittest.main()
