"""【字段输入测试】【验证边界】原生用途与实际首帧分别验证，不能用激活事件推断界面已经呈现"""

import unittest
from unittest.mock import Mock, patch

from wayland_input_method_fields import field_type, visible


class FieldBoundaryTests(unittest.TestCase):
    """【字段输入测试】【测试器约束】用途不匹配、多行提示与缺失界面均不能通过"""

    def test_field_purpose_and_multiline_hint_are_checked_independently(self):
        """【字段输入测试】【用途核验】真实数字用途允许单行输入，其他用途或多行属性拒绝，无参数、无返回值"""
        field_type({"hint": 0, "purpose": 2}, 2)
        for state in ({"hint": 0, "purpose": 0}, {"hint": 512, "purpose": 2}):
            with self.assertRaisesRegex(AssertionError, "type mismatch"):
                field_type(state, 2)

    def test_visible_waits_for_actual_pixels_after_native_activation(self):
        """【字段输入测试】【呈现等待】第一次画面没有控件时继续等待，只有真实定位成功才返回，无参数、无返回值"""
        capture = Mock(side_effect=["before-frame", "actual-frame"])
        locate = Mock(side_effect=[AssertionError("not presented"), (10, 20, 30, 40)])
        with patch("wayland_input_method_fields.time.sleep") as sleep:
            self.assertEqual(visible("process", "root", "field", capture, locate), ("actual-frame", (10, 20, 30, 40)))
        self.assertEqual(capture.call_count, 2)
        self.assertEqual(locate.call_count, 2)
        sleep.assert_called_once_with(0.05)

    def test_missing_frame_is_not_replaced_with_guessed_coordinates(self):
        """【字段输入测试】【缺失拒绝】到达截止时间后保留定位失败，不填写猜测坐标，无参数、无返回值"""
        with patch("wayland_input_method_fields.time.monotonic", side_effect=[0, 4]):
            with self.assertRaisesRegex(ValueError, "missing frame"):
                visible("process", "root", "field", Mock(return_value="missing"), Mock(side_effect=ValueError("missing frame")))


if __name__ == "__main__":
    unittest.main()
