"""【文字格式测试】【光标边界】双帧只能消除窄红色光标，不允许掩盖字形差异或其他动画"""

import unittest

from PIL import Image
from wayland_text_glyph_capture import without_caret


class GlyphCaptureChecks(unittest.TestCase):
    """【文字格式测试】【原始双帧】用确定像素验证严格变化范围，不连接显示服务器"""

    def test_red_caret_is_removed_without_changing_a_glyph_pixel(self):
        """【文字格式测试】【字形保留】真实字形和光标交叉时保留隐藏相位的原像素，无参数、无返回值"""
        first = Image.new("RGB", (100, 60), (30, 34, 38))
        first.putpixel((20, 20), (180, 60, 65))
        second = first.copy()
        second.paste((255, 77, 77), (20, 10, 21, 40))
        actual = without_caret(first, second)
        self.assertEqual(actual.tobytes(), first.tobytes())

    def test_single_glyph_change_cannot_be_mistaken_for_a_caret(self):
        """【文字格式测试】【单像素边界】孤立非红色像素变化仍然失败，无参数、无返回值"""
        first = Image.new("RGB", (100, 60), (30, 34, 38))
        second = first.copy()
        second.putpixel((20, 20), (31, 34, 38))
        with self.assertRaisesRegex(AssertionError, "document changed"):
            without_caret(first, second)

    def test_single_red_glyph_pixel_is_also_rejected(self):
        """【文字格式测试】【红色单像素】正文红色像素变化也不能冒充完整光标，无参数、无返回值"""
        first = Image.new("RGB", (100, 60), (30, 34, 38))
        second = first.copy()
        second.putpixel((20, 20), (255, 77, 77))
        with self.assertRaisesRegex(AssertionError, "document changed"):
            without_caret(first, second)

    def test_body_width_difference_is_rejected(self):
        """【文字格式测试】【正文范围】改变整个文字区域不能以取较暗颜色掩盖，无参数、无返回值"""
        first = Image.new("RGB", (100, 60), (30, 34, 38))
        second = first.copy()
        second.paste((255, 77, 77), (20, 10, 60, 40))
        with self.assertRaisesRegex(AssertionError, "document changed"):
            without_caret(first, second)

    def test_fractional_cursor_fringe_does_not_hide_its_foreground_column(self):
        """【文字格式测试】【分数边缘】逻辑截图的弱抗锯齿外列不代替内部红色光标列，无参数、无返回值"""
        first = Image.new("RGB", (100, 60), (30, 34, 38))
        second = first.copy()
        second.paste((35, 35, 38), (20, 10, 21, 40))
        second.paste((220, 70, 70), (21, 10, 22, 40))
        self.assertEqual(without_caret(first, second).tobytes(), first.tobytes())

    def test_selected_blue_background_never_becomes_a_mixed_color_frame(self):
        """【文字格式测试】【整帧来源】选区蓝色通道高于光标绿色时仍选择完整原始帧，禁止逐通道混合伪造字形，无参数、无返回值"""
        first = Image.new("RGB", (100, 60), (40, 90, 145))
        second = first.copy()
        second.paste((255, 77, 77), (20, 10, 21, 40))
        self.assertIs(without_caret(first, second), first)


if __name__ == "__main__":
    unittest.main()
