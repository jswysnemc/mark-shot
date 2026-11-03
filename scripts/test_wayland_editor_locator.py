"""【界面定位测试】【属性面板】验证定位随面板移动，不命中输入框或无关图形"""

import unittest
from colorsys import hsv_to_rgb

from PIL import Image, ImageDraw

from wayland_editor_locator import color_popover_rectangle, filter_controls, line_controls, magnifier_controls, marker_controls, number_controls, ruler_controls, settings_rectangle


BORDER = (96, 113, 127)


def image(offset=(0, 0)):
    """【界面定位测试】【界面构造】返回带 offset 平移的工具栏、标记属性面板与笔宽框"""
    result = Image.new("RGB", (1000, 700), (30, 34, 38))
    painter = ImageDraw.Draw(result)
    x, y = offset
    painter.rounded_rectangle((120 + x, 10 + y, 890 + x, 51 + y), radius=8, outline=BORDER)
    painter.rounded_rectangle((120 + x, 60 + y, 432 + x, 101 + y), radius=8, outline=BORDER)
    painter.rounded_rectangle((232 + x, 67 + y, 315 + x, 94 + y), radius=4, outline=BORDER)
    return result


def color_popup(result, left, top, outer=True, hex_field=True):
    """【界面定位测试】【颜色构造】在 result 的 left、top 绘制颜色弹层；outer、hex_field 控制真实边框是否存在，无返回值"""
    painter = ImageDraw.Draw(result)
    if outer:
        painter.rounded_rectangle((left, top, left + 199, top + 227), radius=11, fill=(15,17,23), outline=(29,31,37))
    painter.rounded_rectangle((left + 8, top + 8, left + 191, top + 119), radius=4, fill=(20,24,28), outline=(29,31,37))
    for x in range(9,191):
        color=tuple(round(value*255) for value in hsv_to_rgb((x-8.5)/182,1,1))
        painter.line((left+x,top+128,left+x,top+136),fill=color)
    if hex_field:
        painter.rounded_rectangle((left + 40, top + 162, left + 191, top + 187), radius=6, fill=(31,33,39), outline=(38,40,46))


class LocatorTests(unittest.TestCase):
    """【界面定位测试】【定位断言】覆盖基本、平移、缺失与歧义界面"""

    def test_settings_and_marker_button(self):
        """【界面定位测试】【控件坐标】返回实际标记属性框，不把笔宽框当作面板"""
        self.assertEqual(settings_rectangle(image(), BORDER), (120, 60, 433, 102))
        self.assertEqual(marker_controls(image(), BORDER)["shape"], (182, 81))

    def test_translation(self):
        """【界面定位测试】【平移跟随】面板与工具栏改变位置后仍返回对应按钮"""
        self.assertEqual(marker_controls(image((55, 120)), BORDER)["shape"], (237, 201))

    def test_ruler_mode_and_width_coordinates(self):
        """【界面定位测试】【尺子坐标】模式按钮和笔宽输入相对同一个属性面板定位，并跟随平移"""
        self.assertEqual(ruler_controls(image(), BORDER), {"mode": (202, 81), "width": (293, 81)})
        self.assertEqual(ruler_controls(image((55, 120)), BORDER), {"mode": (258, 201), "width": (348, 201)})

    def test_magnifier_coordinates_follow_the_actual_panel(self):
        """【界面定位测试】【放大镜坐标】双数值字段按实际面板定位，框形按钮跟随面板右边界"""
        self.assertEqual(magnifier_controls(image(), BORDER), {"width": (192, 81), "zoom": (300, 81), "shape": (411, 81)})
        self.assertEqual(magnifier_controls(image((55, 120)), BORDER), {"width": (247, 201), "zoom": (355, 201), "shape": (466, 201)})

    def test_number_controls_follow_the_actual_panel(self):
        """【界面定位测试】【序号坐标】样式、笔宽和重置按钮按同一属性框定位，并跟随平移"""
        self.assertEqual(number_controls(image(), BORDER), {"style": (188, 81), "width": (265, 81), "reset": (411, 81)})
        self.assertEqual(number_controls(image((55, 120)), BORDER), {"style": (244, 201), "width": (320, 201), "reset": (466, 201)})

    def test_arrow_controls_keep_head_and_pattern_separate(self):
        """【界面定位测试】【箭头坐标】头部、箭身线型和笔宽按同一属性框定位，随实际面板平移"""
        self.assertEqual(line_controls(image(), BORDER), {"head": (186, 81), "pattern": (238, 81), "width": (312, 81)})
        self.assertEqual(line_controls(image((55, 120)), BORDER), {"head": (241, 201), "pattern": (293, 201), "width": (367, 201)})

    def test_line_controls_have_no_arrow_head_button(self):
        """【界面定位测试】【直线坐标】普通直线只定位线型和笔宽，不把线型误作箭头头部按钮"""
        self.assertEqual(line_controls(image(), BORDER, arrow=False), {"pattern": (186, 81), "width": (260, 81)})
        self.assertEqual(line_controls(image((55, 120)), BORDER, arrow=False), {"pattern": (241, 201), "width": (315, 201)})

    def test_filter_controls_follow_the_actual_panel(self):
        """【界面定位测试】【滤镜坐标】透明度、原图强度和模式按钮随实际属性框平移，不把模式误作强度字段"""
        self.assertEqual(filter_controls(image(), BORDER), {"opacity": (142, 81), "strength": (208, 81), "mode": (310, 81)})
        self.assertEqual(filter_controls(image((55, 120)), BORDER), {"opacity": (197, 201), "strength": (263, 201), "mode": (365, 201)})

    def test_missing_panel(self):
        """【界面定位测试】【界面缺失】无面板图像不提供猜测坐标"""
        with self.assertRaises(ValueError):
            settings_rectangle(Image.new("RGB", (400, 300)), BORDER)

    def test_ambiguous_panel(self):
        """【界面定位测试】【界面歧义】多个属性面板候选需要报告歧义"""
        result = image()
        ImageDraw.Draw(result).rounded_rectangle((500, 110, 760, 151), radius=8, outline=BORDER)
        with self.assertRaises(ValueError):
            settings_rectangle(result, BORDER)

    def test_antialias_border_does_not_merge_with_selection(self):
        """【界面定位测试】【缩放边框】抗锯齿蓝灰边框仍可定位，不与相邻灰色选区线合并"""
        result = image()
        painter = ImageDraw.Draw(result)
        painter.rounded_rectangle((120, 60, 432, 101), radius=8, outline=(69, 80, 90))
        painter.line((20, 59, 980, 59), fill=(156, 166, 171))
        self.assertEqual(settings_rectangle(result, BORDER), (120, 60, 433, 102))

    def test_faint_native_filter_border_remains_detectable(self):
        """【界面定位测试】【弱抗锯齿】使用真实滤镜截图中的暗边框颜色，仍须定位完整属性框而非内部输入框"""
        result = image()
        ImageDraw.Draw(result).rounded_rectangle((120, 60, 432, 101), radius=8, outline=(52, 60, 66))
        self.assertEqual(settings_rectangle(result, BORDER), (120, 60, 433, 102))

    def test_disconnected_border_pieces_keep_the_panel_geometry(self):
        """【界面定位测试】【边框分段】真实重绘的弱侧边不能使完整属性框丢失，上下片段仍须返回正确几何"""
        result = image()
        painter = ImageDraw.Draw(result)
        painter.rectangle((120, 68, 122, 93), fill=(30, 34, 38))
        painter.rectangle((430, 68, 432, 93), fill=(30, 34, 38))
        self.assertEqual(settings_rectangle(result, BORDER), (120, 60, 433, 102))

    def test_textured_background_leaves_a_partial_frame_and_separate_right_stem(self):
        """【界面定位测试】【纹理分段】左侧完整高度、断开的顶部和独立右侧竖边共同确定原属性框，不丢弃短边片段"""
        result = image()
        painter = ImageDraw.Draw(result)
        painter.rectangle((120,60,432,101),fill=(30,34,38))
        painter.line((120,60,200,60),fill=BORDER)
        painter.line((120,60,120,101),fill=BORDER)
        painter.line((120,101,200,101),fill=BORDER)
        painter.line((202,60,432,60),fill=BORDER)
        painter.line((432,68,432,94),fill=BORDER)
        self.assertEqual(settings_rectangle(result,BORDER),(120,60,433,102))
        painter.line((432,68,432,94),fill=(30,34,38))
        with self.assertRaises(ValueError):
            settings_rectangle(result,BORDER)

    def test_color_popover_can_touch_the_toolbar(self):
        """【界面定位测试】【相接弹层】外框与工具栏连通时，色阶和十六进制字段仍须确定唯一弹层位置"""
        result = image()
        color_popup(result, 300, 5)
        self.assertEqual(color_popover_rectangle(result, BORDER), (300, 5, 500, 233))

    def test_color_popover_occupies_the_entire_reference_buffer(self):
        """【界面定位测试】【最小缓冲】二百乘二百二十八的实际控件缓冲仍可定位，不遗漏右边界候选"""
        result = Image.new("RGB", (200, 228), (30,34,38))
        color_popup(result, 0, 0)
        self.assertEqual(color_popover_rectangle(result, BORDER), (0,0,200,228))

    def test_color_square_alone_is_not_a_popover(self):
        """【界面定位测试】【内部缺失】没有外框和对应数值字段的孤立色阶方框不能作为弹层位置"""
        result = image()
        color_popup(result, 300, 5, outer=False, hex_field=False)
        with self.assertRaises(ValueError):
            color_popover_rectangle(result, BORDER)

    def test_two_color_popovers_are_ambiguous(self):
        """【界面定位测试】【颜色歧义】两个拥有完整控件的颜色弹层必须报告歧义，不能任意选择一个"""
        result = image()
        color_popup(result, 300, 5)
        color_popup(result, 650, 180)
        with self.assertRaises(ValueError):
            color_popover_rectangle(result, BORDER)


if __name__ == "__main__":
    unittest.main()
