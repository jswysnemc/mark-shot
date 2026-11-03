"""【文字格式测试】【像素判定】确认真实回归会拒绝字形差异，不把光标或工具栏误当正文"""

import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

from PIL import Image

SPEC = importlib.util.spec_from_file_location("text_format_history", Path(__file__).with_name("wayland-text-format-history.py"))
CHECKS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKS)


class FormatGlyphChecks(unittest.TestCase):
    """【文字格式测试】【像素辅助】验证字形提取和逐字节恢复断言，不连接任何桌面"""

    def test_extracts_red_glyphs_without_white_caret(self):
        """【文字格式测试】【颜色分流】正文红色覆盖保留，白色光标排除，无参数、无返回值"""
        image = Image.new("RGB", (640, 480), (30, 34, 38))
        image.putpixel((300, 200), (255, 77, 77))
        image.putpixel((310, 200), (255, 255, 255))
        result = CHECKS.glyphs(image, (300, 200))
        self.assertEqual(result.getpixel((10, 10)), (255, 77, 77))
        self.assertEqual(result.getpixel((20, 10)), (0, 0, 0))
        self.assertEqual(result.getbbox(), (10, 10, 11, 11))

    def test_excludes_toolbar_and_other_regions(self):
        """【文字格式测试】【区域边界】文字区域外的红色按钮不计入恢复断言，无参数、无返回值"""
        image = Image.new("RGB", (640, 480), (30, 34, 38))
        image.paste((255, 77, 77), (30, 60, 60, 90))
        image.putpixel((100, 200), (255, 77, 77))
        self.assertIsNone(CHECKS.glyphs(image, (300, 200)).getbbox())

    def test_single_pixel_difference_fails_restoration(self):
        """【文字格式测试】【严格恢复】字形单像素通道差异必须失败，无参数、无返回值"""
        first = Image.new("RGB", (5, 5), (255, 77, 77))
        second = first.copy()
        second.putpixel((2, 3), (254, 77, 77))
        with self.assertRaisesRegex(AssertionError, "restore exact document glyphs"):
            CHECKS.same(first, second, "font undo")

    def test_identical_glyphs_pass_restoration(self):
        """【文字格式测试】【一致恢复】完全相同的字形缓冲通过严格比较，无参数、无返回值"""
        image = Image.new("RGB", (5, 5), (255, 77, 77))
        CHECKS.same(image, image.copy(), "font undo")

    def test_focused_caret_may_redraw_only_a_narrow_half_period_region(self):
        """【文字格式测试】【聚焦时钟】聚焦光标允许两次窄更新和一个 CPU 时钟单位，不要求静态窗口的零增量，无参数、无返回值"""
        protocol = "wl_surface@23.damage_buffer(10, 20, 15, 45)\n" * 2
        with patch.object(CHECKS.os, "sysconf", return_value=100):
            self.assertEqual(CHECKS.focused_resources({"cpu_ticks": 10}, {"cpu_ticks": 11, "threads": 1}, protocol), 2)

    def test_focused_resource_check_rejects_full_frame_and_busy_loop(self):
        """【文字格式测试】【轮询拒绝】聚焦状态仍拒绝全屏更新和高 CPU，不能把支持闪烁当作取消资源边界，无参数、无返回值"""
        with patch.object(CHECKS.os, "sysconf", return_value=100):
            with self.assertRaisesRegex(AssertionError, "narrow redraw"):
                CHECKS.focused_resources({"cpu_ticks": 10}, {"cpu_ticks": 11, "threads": 1}, "wl_surface@23.damage_buffer(0, 0, 1280, 800)")
            with self.assertRaisesRegex(AssertionError, "resource bound"):
                CHECKS.focused_resources({"cpu_ticks": 10}, {"cpu_ticks": 20, "threads": 1}, "")


if __name__ == "__main__":
    unittest.main()
