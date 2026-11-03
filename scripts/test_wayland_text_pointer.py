"""【文字指针测试】【输入约束】验证私有上下文要求、输出变换和按钮分流，不连接桌面"""

import unittest
from unittest.mock import Mock, patch

import wayland_text_pointer as pointer


class TextPointerChecks(unittest.TestCase):
    """【文字指针测试】【设备边界】使用协议替身验证坐标与按钮，不创建真实输入设备"""

    def test_input_without_private_context_is_rejected(self):
        """【文字指针测试】【范围校验】没有私有设备上下文时拒绝输入，无参数、无返回值"""
        with self.assertRaisesRegex(RuntimeError, "verified private context"):
            pointer.click((10, 20))

    def test_flipped_output_uses_the_real_logical_extents(self):
        """【文字指针测试】【输出变换】上下翻转只反转一次纵轴，设备范围使用实际逻辑尺寸，无参数、无返回值"""
        device = Mock()
        logical = {"x": 0, "y": 0, "width": 1066, "height": 666, "transform": "Flipped180"}
        token = pointer.CURRENT.set((device, logical, 0.04, None))
        try:
            with patch.object(pointer.time, "sleep"):
                pointer.click((10, 20))
            device.move_abs.assert_called_once_with(10, 646, 1066, 666)
            device.click.assert_called_once_with(pointer.BTN_LEFT, hold=0.04)
        finally:
            pointer.CURRENT.reset(token)

    def test_right_button_never_becomes_a_left_click(self):
        """【文字指针测试】【按钮分流】普通输出上的右键保留实际按钮码，无参数、无返回值"""
        device = Mock()
        logical = {"x": 100, "y": 50, "width": 800, "height": 600, "transform": "Normal"}
        token = pointer.CURRENT.set((device, logical, 0.04, None))
        try:
            with patch.object(pointer.time, "sleep"):
                pointer.click((120, 80), "right")
            device.move_abs.assert_called_once_with(20, 30, 800, 600)
            device.click.assert_called_once_with(pointer.BTN_RIGHT, hold=0.04)
        finally:
            pointer.CURRENT.reset(token)

    def test_unknown_button_is_rejected_before_protocol_input(self):
        """【文字指针测试】【非法按钮】未知按钮不能发送任何协议动作，无参数、无返回值"""
        device = Mock()
        token = pointer.CURRENT.set((device, {}, 0.04, None))
        try:
            with self.assertRaisesRegex(ValueError, "unsupported pointer button"):
                pointer.click((10, 20), "other")
            device.move_abs.assert_not_called()
            device.click.assert_not_called()
        finally:
            pointer.CURRENT.reset(token)

    def test_private_xvfb_coordinates_do_not_repeat_virtual_pointer_transform(self):
        """【文字指针测试】【私有显示】私有 Xvfb 注入采用真实物理尺寸，不重复虚拟协议输出翻转，不使用调用环境，无参数、无返回值"""
        logical = {"x": 0, "y": 0, "width": 1000, "height": 500, "transform": "Flipped180"}
        environment = {"DISPLAY": ":isolated-test"}
        token = pointer.CURRENT.set((None, logical, 0.04, (environment, {"width": 1200, "height": 600})))
        try:
            with patch.object(pointer.subprocess, "run") as run, patch.object(pointer.time, "sleep"):
                pointer.click((100, 200), "right")
            self.assertEqual(run.call_args_list[0].args[0], ["xdotool", "mousemove", "--sync", "120", "240"])
            self.assertEqual(run.call_args_list[1].args[0], ["xdotool", "mousedown", "3"])
            self.assertEqual(run.call_args_list[2].args[0], ["xdotool", "mouseup", "3"])
            self.assertTrue(all(call.kwargs["env"] is environment for call in run.call_args_list))
        finally:
            pointer.CURRENT.reset(token)

    def test_invalid_press_duration_fails_before_private_session_access(self):
        """【文字指针测试】【时长边界】非法按下时长在任何会话读取前拒绝，无参数、无返回值"""
        with patch.object(pointer.session, "environment") as environment:
            for hold in (0, -0.1, 3):
                with self.assertRaises(ValueError), pointer.pointer_context(hold):
                    pass
            environment.assert_not_called()


if __name__ == "__main__":
    unittest.main()
