"""【原图测试】【比较器回归】相同透明度不能掩盖颜色变化，也不能忽略尺寸和透明度"""

import unittest

from PIL import Image
from wayland_ocr_preview_checks import equal_pixels, keyboard_release_grab


class PixelChecksTests(unittest.TestCase):
    """【原图测试】【真实比较】直接使用 Pillow 像素构造变化，不连接桌面"""

    def test_same_alpha_does_not_hide_rgb_difference(self):
        """【原图测试】【颜色变化】保持透明度一致并只改变红色通道，必须判定不相同，无参数、无返回值"""
        baseline = Image.new("RGBA", (2, 2), (15, 23, 42, 255))
        changed = baseline.copy()
        changed.putpixel((0, 0), (16, 23, 42, 255))
        self.assertFalse(equal_pixels(changed, baseline))

    def test_alpha_and_size_are_compared(self):
        """【原图测试】【透明度和尺寸】相同图片通过，透明度或尺寸变化拒绝，无参数、无返回值"""
        baseline = Image.new("RGBA", (2, 2), (15, 23, 42, 255))
        self.assertTrue(equal_pixels(baseline, baseline.copy()))
        self.assertFalse(equal_pixels(Image.new("RGBA", (2, 2), (15, 23, 42, 254)), baseline))
        self.assertFalse(equal_pixels(Image.new("RGBA", (3, 2), (15, 23, 42, 255)), baseline))


    def test_keyboard_release_menu_consumes_its_press_serial(self):
        """【原图测试】【真实配对】释放完成的菜单使用同一次按下序号，不使用释放序号，无参数、无返回值"""
        text = "wl_keyboard@21.key, (22, 100, 57, 1)\nwl_keyboard@21.key, (23, 101, 57, 0)\nxdg_popup@30.grab(wl_seat@19, 22)"
        self.assertEqual(keyboard_release_grab(text, 57), {"press": 22, "release": 23, "grab": 22, "code": 57})

    def test_dynamic_virtual_keymap_uses_its_actual_raw_code(self):
        """【原图测试】【动态键码】虚拟键盘把 Space 映射为动态键码时，仍须由相同设备和键码配对，无参数、无返回值"""
        text = "wl_keyboard@26.key, (27, 0, 1, 1)\nwl_keyboard@26.key, (28, 0, 1, 0)\nxdg_popup@32.grab(wl_seat@20, 27)"
        self.assertEqual(keyboard_release_grab(text), {"press": 27, "release": 28, "grab": 27, "code": 1})

    def test_release_or_stale_serial_cannot_authorize_a_popup(self):
        """【原图测试】【序号复用】释放序号与其他事务旧序号必须失败，无参数、无返回值"""
        for serial in (23, 18):
            with self.subTest(serial=serial):
                text = f"wl_keyboard@21.key, (22, 100, 57, 1)\nwl_keyboard@21.key, (23, 101, 57, 0)\nxdg_popup@30.grab(wl_seat@19, {serial})"
                with self.assertRaisesRegex(AssertionError, "matching press serial"):
                    keyboard_release_grab(text, 57)

    def test_incomplete_press_release_transaction_is_rejected(self):
        """【原图测试】【配对完整】缺少真实按下或释放时，不从弹层创建推断抓取来源，无参数、无返回值"""
        with self.assertRaisesRegex(AssertionError, "real press/release transaction"):
            keyboard_release_grab("wl_keyboard@21.key, (23, 101, 57, 0)\nxdg_popup@30.grab(wl_seat@19, 23)", 57)


    def test_another_device_or_raw_code_cannot_complete_the_transaction(self):
        """【原图测试】【来源隔离】其他键盘设备或不同原始键码不能配对当前按下，无参数、无返回值"""
        for keyboard, code in ((26, 57), (21, 58)):
            with self.subTest(keyboard=keyboard, code=code):
                text = f"wl_keyboard@21.key, (22, 100, 57, 1)\nwl_keyboard@{keyboard}.key, (23, 101, {code}, 0)\nxdg_popup@30.grab(wl_seat@19, 22)"
                with self.assertRaisesRegex(AssertionError, "real press/release transaction"):
                    keyboard_release_grab(text)


if __name__ == "__main__":
    unittest.main()
