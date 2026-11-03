"""【文字菜单测试】【抓取坐标】检查最小回归能够拒绝菜单关闭后的旧坐标，不连接桌面"""

import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location("text_popup_pointer", Path(__file__).with_name("wayland-text-popup-pointer.py"))
CHECKS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKS)


class PopupPointerChecks(unittest.TestCase):
    """【文字菜单测试】【协议断言】使用实际格式的协议字符串验证事件解析与量化边界"""

    def test_last_motion_is_selected_after_destruction(self):
        """【文字菜单测试】【事件顺序】解析末次真实事件，不把菜单前的坐标当作恢复结果，无参数、无返回值"""
        protocol = "wl_pointer@18.motion, (10, 672.7070, 359.3438)\nxdg_popup@32.destroy()\nwl_pointer@18.motion, (20, 341.0039, 499.8477)"
        self.assertEqual(CHECKS.last_motion(protocol), (341.0039, 499.8477))

    def test_stale_grab_coordinates_fail(self):
        """【文字菜单测试】【旧坐标拒绝】真实失败坐标不能通过抓取释放验证，无参数、无返回值"""
        with self.assertRaisesRegex(AssertionError, "after popup dismissal"):
            CHECKS.assert_motion((672.707, 359.3438), (341, 500))

    def test_pixel_quantization_is_not_a_stuck_pointer(self):
        """【文字菜单测试】【输出量化】分数输出的单栅格误差通过，超过边界失败，无参数、无返回值"""
        CHECKS.assert_motion((341.7, 499.85), (341, 500))
        with self.assertRaises(AssertionError):
            CHECKS.assert_motion((343, 500), (341, 500))

    def test_missing_motion_fails(self):
        """【文字菜单测试】【事件缺失】没有后续 motion 不能返回虚构坐标，无参数、无返回值"""
        with self.assertRaisesRegex(AssertionError, "no pointer motion"):
            CHECKS.last_motion("xdg_popup@32.destroy()")

    def test_pre_popup_motion_cannot_substitute_for_a_post_popup_event(self):
        """【文字菜单测试】【时序边界】销毁前的目标坐标不代替销毁后的真实事件，无参数、无返回值"""
        with self.assertRaisesRegex(AssertionError, "after popup destruction"):
            CHECKS.last_motion("wl_pointer@18.motion, (10, 341.0039, 499.8477)\nxdg_popup@32.destroy()")


if __name__ == "__main__":
    unittest.main()
