"""【原文菜单测试】【协议回归】区分 Rust 事件日志与请求表示，不因格式差异丢失真实弹层证据"""

import unittest

from wayland_ocr_source_menu_checks import popup_configurations


class SourceMenuProtocolTests(unittest.TestCase):
    """【原文菜单测试】【矩形解析】覆盖配置表示、负坐标和父窗口隔离"""

    def test_rust_received_event_format(self):
        """【原文菜单测试】【接收格式】Rust 事件带逗号与括号，返回实际逻辑矩形，无参数、无返回值"""
        self.assertEqual(popup_configurations("[1][rs] <- xdg_popup@42.configure, (120, 120, 225, 229)"), [(120, 120, 225, 229)])

    def test_negative_positions_and_request_format(self):
        """【原文菜单测试】【避让坐标】负坐标和调用表示均保留精确值，不进行额外贴边，无参数、无返回值"""
        self.assertEqual(popup_configurations("xdg_popup@1.configure(-100, -20, 225, 229)\nxdg_popup@2.configure, (30, 40, 225, 229)"), [(-100, -20, 225, 229), (30, 40, 225, 229)])

    def test_parent_window_is_not_a_popup(self):
        """【原文菜单测试】【角色隔离】只有独立弹层配置可以作为菜单证据，无参数、无返回值"""
        self.assertEqual(popup_configurations("xdg_toplevel@42.configure, (225, 229, [])\nxdg_surface@3.configure, (120)"), [])


if __name__ == "__main__":
    unittest.main()
