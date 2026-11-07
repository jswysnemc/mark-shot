"""【OCR 置顶测试】【解析回归】真实角色需要创建、配置及绘制证据，销毁后编号复用不能继承旧状态"""

import unittest
from wayland_ocr_stacking_checks import roles


class StackingProtocolTests(unittest.TestCase):
    """【OCR 置顶测试】【协议边界】核对 Qt 和 Rust 日志语法，不连接显示服务器"""

    def test_real_window_requires_its_own_configure_buffer_and_keyboard_focus(self):
        """【OCR 置顶测试】【普通角色】窗口创建、配置、实际缓冲与座席进入各自独立记录，无参数、无返回值"""
        log = """xdg_wm_base@5.get_xdg_surface(xdg_surface@31, wl_surface@30)
xdg_surface@31.get_toplevel(xdg_toplevel@32)
xdg_toplevel@32.configure, (760, 460, [])
wl_surface@30.attach(Some(wl_buffer@40), 0, 0)
wl_keyboard@9.enter, (4, wl_surface@30, [])
"""
        value = roles(log)[0]
        self.assertEqual(value["surface"], 30)
        self.assertTrue(value["configured"] and value["drawn"] and value["keyboard_focused"])

    def test_layer_creation_configuration_and_negative_margins_are_retained(self):
        """【OCR 置顶测试】【浮层角色】浮层必须真实创建和配置，负边距不能静默裁剪，无参数、无返回值"""
        log = """zwlr_layer_shell_v1#5.get_layer_surface(new id zwlr_layer_surface_v1#31, wl_surface#30, nil, 2, "mark-shot-ocr-result")
zwlr_layer_surface_v1#31.set_margin(-20, 0, 0, 40)
zwlr_layer_surface_v1#31.configure(4, 760, 460)
wl_surface#30.attach(wl_buffer#40, 0, 0)
"""
        value = roles(log)[0]
        self.assertEqual(value["position"], [40, -20])
        self.assertEqual(value["size"], [760, 460])
        self.assertTrue(value["configured"] and value["drawn"])

    def test_reused_layer_identity_does_not_keep_old_configuration_or_pixels(self):
        """【OCR 置顶测试】【编号复用】旧角色销毁后同编号必须重新配置、绘制和定位，无参数、无返回值"""
        create = 'zwlr_layer_shell_v1@5.get_layer_surface(zwlr_layer_surface_v1@31, wl_surface@30, None, 2, "mark-shot-ocr-result")\n'
        log = create + "zwlr_layer_surface_v1@31.configure, (4, 760, 460)\nwl_surface@30.attach(Some(wl_buffer@40), 0, 0)\nzwlr_layer_surface_v1@31.destroy()\n" + create
        value = roles(log)[0]
        self.assertFalse(value["configured"] or value["drawn"])
        self.assertNotIn("position", value)

    def test_global_advertisement_and_orphan_configure_do_not_prove_a_role(self):
        """【OCR 置顶测试】【错误证据】接口广播和未创建对象的配置不能当作实际置顶，无参数、无返回值"""
        self.assertEqual(roles('wl_registry@2.global(5, "zwlr_layer_shell_v1", 4)\nzwlr_layer_surface_v1@31.configure, (4, 760, 460)'), [])


    def test_anonymous_zero_output_is_the_actual_nullable_object_format(self):
        """【OCR 置顶测试】【空输出对象】Rust 实际日志使用匿名零对象表达空输出，层级必须仍精确读取为 Top，无参数、无返回值"""
        log = 'zwlr_layer_shell_v1@7.get_layer_surface(zwlr_layer_surface_v1@13, wl_surface@12, <anonymous>@0, 2, Some("mark-shot-ocr-result"))'
        self.assertEqual(roles(log)[0].get("level"), 2)


if __name__ == "__main__":
    unittest.main()
