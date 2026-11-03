"""【箭头测试】【断言验证】验证像素判定和参考几何，无显示服务器或真实输入"""

import json
import unittest
from unittest.mock import patch

from PIL import Image, ImageDraw

import wayland_arrow_checks as checks


class ArrowCheckTests(unittest.TestCase):
    """【箭头断言测试】【用例集合】以明确像素检查线型、透明度、曲线和运行范围"""

    def test_shaft_patterns_accept_real_gaps_and_reject_a_solid_substitute(self):
        """【箭头断言测试】【线型区别】实线没有空隙，虚线和点线必须包含独立笔迹，连续替代会失败"""
        solid = Image.new("RGB", (240, 60), (30, 34, 38))
        ImageDraw.Draw(solid).line((10, 30, 220, 30), fill=(143, 56, 58), width=3)
        checks.check_shaft(solid, (10, 30), (220, 30), "solid")
        for pattern, size in [("dashed", 12), ("dotted", 3)]:
            with self.subTest(pattern=pattern):
                image = Image.new("RGB", (240, 60), (30, 34, 38))
                painter = ImageDraw.Draw(image)
                for x in range(10, 220, size + 7):
                    painter.line((x, 30, x + size - 1, 30), fill=(143, 56, 58), width=3)
                checks.check_shaft(image, (10, 30), (220, 30), pattern)
                with self.assertRaises(RuntimeError):
                    checks.check_shaft(solid, (10, 30), (220, 30), pattern)

    def test_alpha_guard_rejects_a_twice_painted_half_transparent_pixel(self):
        """【箭头断言测试】【接缝覆盖】单次半透明混合通过，重复混合的更亮红色必须触发异常"""
        image = Image.new("RGB", (20, 20), (143, 56, 58))
        checks.check_alpha(image, (0, 0, 20, 20))
        image.putpixel((10, 10), (199, 67, 68))
        with self.assertRaises(RuntimeError):
            checks.check_alpha(image, (0, 0, 20, 20))

    def test_red_bounds_ignore_background_and_chrome_colors(self):
        """【箭头断言测试】【正文边界】灰蓝控制框不计入正文边界，无红色正文时不猜测控制点"""
        image = Image.new("RGB", (60, 50), (96, 113, 127))
        ImageDraw.Draw(image).rectangle((20, 15, 40, 30), fill=(143, 56, 58))
        self.assertEqual(checks.red_bounds(image, (0, 0, 60, 50)), (20, 15, 41, 31))
        self.assertTrue(checks.red_near(image, (25, 20)))
        self.assertFalse(checks.red_near(image, (5, 5)))
        with self.assertRaises(RuntimeError):
            checks.red_bounds(image, (0, 0, 10, 10))

    def test_curve_and_rotation_references_are_not_polyline_substitutes(self):
        """【箭头断言测试】【参考几何】单控制点使用真正二次曲线中点，旋转按给定圆心计算"""
        self.assertEqual(checks.curve_point((0, 0), (50, 40), (100, 0), 0.5), (50, 20))
        x, y = checks.rotated((10, 0), (0, 0), 90)
        self.assertAlmostEqual(x, 0)
        self.assertAlmostEqual(y, 10)

    def test_layout_uses_the_actual_private_output_and_rejects_ambiguity(self):
        """【箭头断言测试】【输出布局】所有矩阵单元按实际逻辑尺寸布置，多输出时拒绝模糊的测试目标"""
        source = Image.new("RGBA", (1600, 900))
        output = {"winit": {"logical": {"width": 1066, "height": 666}}}
        with patch.object(checks.subprocess, "check_output", return_value=json.dumps(output).encode()):
            layout = checks.geometry({}, source)
        self.assertEqual(len(layout["matrix"]), 12)
        self.assertEqual(layout["safe_y"], 660)
        self.assertAlmostEqual(layout["viewport"][2], 1002 / 1600)
        for cell in layout["matrix"]:
            self.assertTrue(0 <= cell["start"][0] < cell["end"][0] < 1066)
            self.assertTrue(0 < cell["region"][1] < cell["region"][3] < 666)
        with patch.object(checks.subprocess, "check_output", return_value=b"{}"):
            with self.assertRaises(RuntimeError):
                checks.geometry({}, source)


if __name__ == "__main__":
    unittest.main()
