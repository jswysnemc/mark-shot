"""【文字属性定位测试】【歧义保护】验证真实颜色按钮和搜索边框定位，不能把填充色块或多候选当作面板"""

import unittest
from PIL import Image, ImageDraw
from wayland_text_property_locator import font_button, font_popup


class TextPropertyLocatorTests(unittest.TestCase):
    """【文字属性定位测试】【固定图像】使用确定几何验证定位器，不连接显示服务器"""

    def test_font_button_uses_color_button_geometry(self):
        """【文字属性定位测试】【按钮锚点】红色按钮与字体按钮相差七十六像素，返回 None"""
        image = Image.new("RGB", (640, 480), (30, 34, 38))
        ImageDraw.Draw(image).rectangle((324, 10, 353, 39), fill=(255, 77, 77))
        self.assertEqual(font_button(image), (415, 25))

    def test_duplicate_red_buttons_are_rejected(self):
        """【文字属性定位测试】【重复按钮】两个候选红色按钮必须报告歧义，返回 None"""
        image = Image.new("RGB", (640, 480), (30, 34, 38))
        draw = ImageDraw.Draw(image)
        draw.rectangle((100, 10, 129, 39), fill=(255, 77, 77))
        draw.rectangle((200, 10, 229, 39), fill=(255, 77, 77))
        with self.assertRaises(AssertionError):
            font_button(image)

    def test_search_border_ignores_selected_font_fill(self):
        """【文字属性定位测试】【边框与填充】搜索边框可定位，字体选中色块不能冒充搜索框，返回 None"""
        image = Image.new("RGB", (640, 480), (30, 34, 38))
        draw = ImageDraw.Draw(image)
        draw.rectangle((106, 47, 453, 74), outline=(45, 212, 191), width=1)
        draw.rectangle((106, 100, 453, 127), fill=(45, 212, 191))
        self.assertEqual(font_popup(image), (100, 40, 460, 400))

    def test_missing_panel_is_rejected(self):
        """【文字属性定位测试】【无面板】没有搜索边框时定位失败，不能推测弹层位置，返回 None"""
        image = Image.new("RGB", (640, 480), (30, 34, 38))
        with self.assertRaises(AssertionError):
            font_popup(image)

    def test_size_field_locates_panel_when_search_loses_focus(self):
        """【文字属性定位测试】【字号焦点】搜索失去焦点时仍按真实字号底板定位面板，返回 None"""
        for background, field in [((248, 250, 252), (239, 241, 244)), ((15, 17, 23), (30, 32, 38))]:
            with self.subTest(field=field):
                image = Image.new("RGB", (640, 480), background)
                ImageDraw.Draw(image).rectangle((107, 367, 392, 390), fill=field)
                self.assertEqual(font_popup(image), (100, 40, 460, 400))


if __name__ == "__main__":
    unittest.main()
