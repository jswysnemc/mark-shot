"""【窗口输入法测试】【呈现边界】协议状态先于实际画面时继续等待，不能放宽真实字形断言"""

from pathlib import Path
import unittest
from unittest.mock import patch

from PIL import Image
import wayland_window_input_session as session


class WindowPresentationTests(unittest.TestCase):
    """【窗口输入法测试】【严格像素】确定候选等待、光标排除与新序号边界，不连接桌面"""

    def test_delayed_candidate_waits_for_actual_glyph_pixels(self):
        """【窗口输入法测试】【延迟回归】首张截图尚未呈现候选时继续获取画面，无参数、无返回值"""
        before = Image.new("RGB", (60, 40))
        after = before.copy()
        after.paste((200, 200, 200), (14, 2, 19, 6))
        with patch.object(session, "screenshot", side_effect=[before, after]) as capture, patch.object(session, "cursor_rect", return_value=(24, 0, 1, 20)), patch.object(session.time, "sleep"):
            actual, pixels = session.presented_candidate(Path("private"), "candidate", before, {"x": 0, "y": 0}, (10, 0, 1, 20), Path("private/log"))
        self.assertIs(actual, after)
        self.assertEqual(pixels, 20)
        self.assertEqual(capture.call_count, 2)

    def test_cursor_and_underline_do_not_prove_candidate_glyphs(self):
        """【窗口输入法测试】【光标排除】仅光标与下划线变化不满足真实候选字形条件，无参数、无返回值"""
        before = Image.new("RGB", (60, 40))
        after = before.copy()
        after.paste((255, 255, 255), (24, 0, 25, 20))
        after.paste((255, 255, 255), (10, 16, 24, 17))
        self.assertEqual(session.candidate_pixels(before, after, {"x": 0, "y": 0}, (10, 0, 1, 20), (24, 0, 1, 20)), 0)

    def test_no_glyph_pixels_eventually_fail(self):
        """【窗口输入法测试】【失败保留】协议激活但画面始终不变时仍失败，无参数、无返回值"""
        before = Image.new("RGB", (60, 40))
        with patch.object(session, "screenshot", return_value=before), patch.object(session, "cursor_rect", return_value=(24, 0, 1, 20)), patch.object(session.time, "monotonic", side_effect=[0, 0, 4]), patch.object(session.time, "sleep"):
            with self.assertRaisesRegex(AssertionError, "real candidate glyphs"):
                session.presented_candidate(Path("private"), "candidate", before, {"x": 0, "y": 0}, (10, 0, 1, 20), Path("private/log"))


    def test_menu_cancellation_compares_matching_inactive_focus(self):
        """【窗口输入法测试】【相同焦点】基准与待测图片均为菜单焦点时，完全一致的候选区域通过，无参数、无返回值"""
        before = Image.new("RGB", (760, 460))
        self.assertEqual(session.canceled_candidate_pixels(before, before.copy(), {"x": 0, "y": 0}, (68, 88, 1, 19), (153, 88, 1, 19)), 0)

    def test_menu_cancellation_rejects_glyph_underline_and_new_cursor_residue(self):
        """【窗口输入法测试】【取消残留】原光标、候选字形、下划线和新光标的任意像素残留必须失败，无参数、无返回值"""
        before = Image.new("RGB", (760, 460))
        for point in ((68, 93), (110, 93), (110, 106), (153, 93)):
            with self.subTest(point=point):
                after = before.copy()
                after.putpixel(point, (255, 255, 255))
                self.assertEqual(session.canceled_candidate_pixels(before, after, {"x": 0, "y": 0}, (68, 88, 1, 19), (153, 88, 1, 19)), 1)


    def test_menu_cancellation_uses_the_real_translated_candidate_range(self):
        """【窗口输入法测试】【长候选范围】窗口偏移与长候选必须按真实光标定位，不使用固定截断区域，无参数、无返回值"""
        before = Image.new("RGB", (760, 460))
        after = before.copy()
        after.putpixel((240, 118), (255, 255, 255))
        self.assertEqual(session.canceled_candidate_pixels(before, after, {"x": 30, "y": 25}, (68, 88, 1, 19), (230, 88, 1, 19)), 1)

    def test_menu_cancellation_rejects_a_nonadvancing_candidate_cursor(self):
        """【窗口输入法测试】【无效候选】没有候选光标范围时拒绝空比较，不能以零像素差异认定取消成功，无参数、无返回值"""
        image = Image.new("RGB", (760, 460))
        with self.assertRaisesRegex(AssertionError, "real candidate range"):
            session.canceled_candidate_pixels(image, image, {"x": 0, "y": 0}, (68, 88, 1, 19), (68, 88, 1, 19))


if __name__ == "__main__":
    unittest.main()
