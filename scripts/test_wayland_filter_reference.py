"""【滤镜参考测试】【像素断言】验证独立参考的模式、舍入、透明度与失败检测"""

import unittest

import numpy as np
from PIL import Image

from wayland_filter_reference import assert_region, expected_export, filtered, gaussian, overlay, premultiplied


class FilterReferenceTests(unittest.TestCase):
    """【滤镜参考测试】【独立规则】覆盖全部模式与逐像素比较，错误结果不能通过回归"""

    def test_reference_colors(self):
        """【滤镜参考测试】【颜色规则】灰度、反色和提亮使用原版整数权重与半值舍入"""
        source = Image.new("RGBA", (1, 1), (200, 100, 50, 255))
        self.assertEqual(filtered(source, (0, 0, 1, 1), "grayscale", 14)[0, 0].tolist(), [126, 126, 126, 255])
        self.assertEqual(filtered(source, (0, 0, 1, 1), "invert", 14)[0, 0].tolist(), [55, 155, 205, 255])
        self.assertEqual(filtered(source, (0, 0, 1, 1), "brighten", 30)[0, 0].tolist(), [228, 178, 153, 255])

    def test_pixelation_does_not_premultiply_average_twice(self):
        """【滤镜参考测试】【块透明度】恒定半透明绿色仍保留原始预乘通道，不重复乘透明度"""
        source = Image.new("RGBA", (4, 4), (0, 255, 0, 128))
        self.assertEqual(filtered(source, (0, 0, 4, 4), "pixelate", 4)[0, 0].tolist(), [0, 128, 0, 128])

    def test_partial_blocks_have_independent_averages(self):
        """【滤镜参考测试】【残余块】末尾单列不与之前四像素块混合"""
        source = Image.new("RGBA", (5, 4), (0, 0, 0, 255))
        for y in range(4):
            source.putpixel((4, y), (255, 255, 255, 255))
        result = filtered(source, (0, 0, 5, 4), "pixelate", 4)
        self.assertEqual(result[1, 3].tolist(), [0, 0, 0, 255])
        self.assertEqual(result[1, 4].tolist(), [255, 255, 255, 255])

    def test_blur_matches_reference_split_edge(self):
        """【滤镜参考测试】【模糊边界】黑白边界变为中间灰度，远端保持原色"""
        source = Image.new("RGBA", (40, 10), (0, 0, 0, 255))
        for y in range(10):
            for x in range(20, 40):
                source.putpixel((x, y), (255, 255, 255, 255))
        result = gaussian(premultiplied(source), 6)
        self.assertGreater(int(result[5, 20, 0]), 40)
        self.assertLess(int(result[5, 20, 0]), 215)
        self.assertEqual(int(result[5, 0, 0]), 0)
        self.assertEqual(int(result[5, 39, 0]), 255)

    def test_blur_keeps_constant_premultiplied_alpha(self):
        """【滤镜参考测试】【恒定透明度】最大强度模糊保留恒定预乘像素"""
        values = np.full((20, 20, 4), (0, 128, 0, 128), dtype=np.uint32)
        np.testing.assert_array_equal(gaussian(values, 48), values)

    def test_translucent_overlay_matches_source_over_alpha(self):
        """【滤镜参考测试】【透明覆盖】覆盖层按标注透明度合成，颜色滤镜仍保留源透明度"""
        source = Image.new("RGBA", (1, 1), (0, 255, 0, 128))
        result = expected_export(source, [{"native": (0, 0, 1, 1), "mode": "invert", "strength": 14}])
        self.assertEqual(result.getpixel((0, 0)), (170, 85, 170, 192))
        zero = overlay(premultiplied(source), filtered(source, (0, 0, 1, 1), "invert", 14), 0)
        np.testing.assert_array_equal(zero, premultiplied(source))

    def test_opacity_keeps_fractional_channels_until_source_over_finishes(self):
        """【滤镜参考测试】【合成量化】透明度保留小数直到覆盖完成，不能先量化蓝色通道导致直通颜色相差两单位"""
        destination = np.array([[[16, 30, 13, 64]]], dtype=np.uint32)
        effect = np.array([[[46, 29, 23, 93]]], dtype=np.uint32)
        self.assertEqual(overlay(destination, effect, 128)[0, 0].tolist(), [36, 39, 22, 99])

    def test_pixel_assertion_rejects_color_and_alpha_changes(self):
        """【滤镜参考测试】【失败检测】颜色超差和任何透明度变化都不能通过断言"""
        expected = Image.new("RGBA", (2, 2), (100, 80, 60, 128))
        for value in [(103, 80, 60, 128), (100, 80, 60, 127)]:
            with self.assertRaises(AssertionError):
                assert_region(Image.new("RGBA", (2, 2), value), expected, (0, 0, 2, 2))
        assert_region(Image.new("RGBA", (2, 2), (101, 80, 60, 128)), expected, (0, 0, 2, 2))

    def test_invalid_modes_and_strengths_fail(self):
        """【滤镜参考测试】【参数拒绝】未知模式与非有限强度不产生假成功图像"""
        source = Image.new("RGBA", (2, 2))
        for mode, strength in [("other", 14), ("blur", float("nan")), ("blur", 100)]:
            with self.assertRaises(ValueError):
                filtered(source, (0, 0, 2, 2), mode, strength)
        values = premultiplied(source)
        for opacity in (-1, 256, float("nan")):
            with self.assertRaises(ValueError):
                overlay(values, values, opacity)


if __name__ == "__main__":
    unittest.main()
