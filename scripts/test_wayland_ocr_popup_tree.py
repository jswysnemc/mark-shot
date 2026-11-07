"""【更多菜单测试】【检查器回归】构造协议错误验证检查器可失败，不把模拟记录作为真实运行证据"""

import unittest

from wayland_ocr_popup_tree import active_tree, pointer_position, popup_tree

ROOT = """get_xdg_surface(xdg_surface@12, wl_surface@11)
wl_pointer@22.button, (100, 200, 272, 1)
get_xdg_surface(xdg_surface@31, wl_surface@30)
xdg_surface@31.get_popup(xdg_popup@32, xdg_surface@12, xdg_positioner@29)
xdg_popup@32.grab(wl_seat@20, 100)
xdg_popup@32.configure, (-10, 36, 195, 37)
wl_surface@30.attach(wl_buffer@40, 0, 0)
"""
CHILD = """get_xdg_surface(xdg_surface@51, wl_surface@50)
xdg_surface@51.get_popup(xdg_popup@52, xdg_surface@31, xdg_positioner@49)
xdg_popup@52.grab(wl_seat@20, 100)
xdg_popup@52.configure, (-250, 0, 252, 95)
wl_surface@50.attach(wl_buffer@60, 0, 0)
"""


class PopupTreeTests(unittest.TestCase):
    """【更多菜单测试】【父链错误】区分真子层、兄弟层、陈旧抓取、表面缺失和关闭顺序"""

    def test_independent_surfaces_inherit_live_grab(self):
        """【更多菜单测试】【活动链】两个独立表面继承同一抓取，负定位保持原值，无参数、无返回值"""
        nodes = active_tree(ROOT + CHILD)
        self.assertEqual([node["depth"] for node in nodes], [0, 1])
        self.assertNotEqual(nodes[0]["surface"], nodes[1]["surface"])
        self.assertEqual(nodes[1]["rect"], [-250, 0, 252, 95])
        self.assertEqual(nodes[0]["grab"], nodes[1]["grab"])

    def test_reverse_destruction_is_required(self):
        """【更多菜单测试】【逆序关闭】正确回收通过，先销毁根节点必须失败，无参数、无返回值"""
        text = ROOT + CHILD + "xdg_popup@52.destroy()\nxdg_popup@32.destroy()"
        self.assertTrue(all(node["destroyed"] for node in popup_tree(text, closed=True)))
        with self.assertRaisesRegex(AssertionError, "ancestor"):
            popup_tree(ROOT + CHILD + "xdg_popup@32.destroy()")

    def test_sibling_and_unrelated_grab_are_rejected(self):
        """【更多菜单测试】【非法子层】兄弟角色或不同子层抓取必须失败，无参数、无返回值"""
        with self.assertRaisesRegex(AssertionError, "deepest"):
            popup_tree(ROOT + CHILD.replace("xdg_surface@31,", "xdg_surface@12,"))
        with self.assertRaisesRegex(AssertionError, "inherit"):
            popup_tree(ROOT + CHILD.replace("wl_seat@20, 100", "wl_seat@20, 101"))

    def test_closed_serial_cannot_create_a_new_root(self):
        """【更多菜单测试】【旧抓取隔离】根关闭后相同序号不得重建会话，新输入序号可以，无参数、无返回值"""
        text = ROOT + "xdg_popup@32.destroy()\n"
        with self.assertRaisesRegex(AssertionError, "fresh"):
            popup_tree(text + ROOT)
        self.assertEqual(len(popup_tree(text + ROOT.replace("100", "101"))), 2)

    def test_missing_surface_or_grab_cannot_pass(self):
        """【更多菜单测试】【真实资源】缺少原生表面、抓取或仍存活的根均不得算作关闭，无参数、无返回值"""
        with self.assertRaisesRegex(AssertionError, "surface"):
            popup_tree(ROOT.replace("get_xdg_surface(xdg_surface@31, wl_surface@30)", ""))
        with self.assertRaisesRegex(AssertionError, "explicit"):
            popup_tree(ROOT.replace("xdg_popup@32.grab(wl_seat@20, 100)", ""))
        with self.assertRaisesRegex(AssertionError, "retained"):
            popup_tree(ROOT, closed=True)

    def test_pointer_delivery_cannot_use_old_or_other_surface_coordinates(self):
        """【更多菜单测试】【首个事件隔离】只接受实际目标表面进入与移动，其他表面和离开后坐标不得通过，无参数、无返回值"""
        text = "wl_pointer@24.enter, (1, wl_surface@30, 24.0000, 18.5000)\n"
        self.assertEqual(pointer_position(text, 30), (24.0, 18.5))
        self.assertEqual(pointer_position(text + "wl_pointer@24.motion, (2, 25.0000, 18.5000)", 30), (25.0, 18.5))
        self.assertIsNone(pointer_position(text, 50))
        self.assertIsNone(pointer_position(text + "wl_pointer@24.leave, (3, wl_surface@30)", 30))


if __name__ == "__main__":
    unittest.main()
