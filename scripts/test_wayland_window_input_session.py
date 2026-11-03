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


if __name__ == "__main__":
    unittest.main()
