"""【滤镜场景测试】【参考边界】验证原图往返稳定、跨块布局和实际输入交叉核对"""

import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import numpy as np

from wayland_filter_fixture import geometry, layout_for, read_commits, source_image, validate_commit
from wayland_filter_reference import premultiplied, straight


class FilterFixtureTests(unittest.TestCase):
    """【滤镜场景测试】【独立校验】防止错误提交日志通过模式、强度和坐标检查"""

    def setUp(self):
        """【滤镜场景测试】【基础布局】创建固定输出与原图用于每项校验；无返回值"""
        self.source = source_image()
        self.layout = layout_for({"width": 1066, "height": 666}, self.source)
        self.annotation = {"id": 1, "mode": "blur", "strength": 14, "native": (100, 100, 300, 200), "logical": (94.625, 113.8125, 219.875, 176.4375)}
        self.start = self.annotation["logical"][:2]
        self.end = self.annotation["logical"][2:]

    def test_source_roundtrip_and_transparency(self):
        """【滤镜场景测试】【像素稳定】原图预乘往返完全一致，并包含透明及半透明测试区域"""
        np.testing.assert_array_equal(np.asarray(self.source), straight(premultiplied(self.source)).astype(np.uint8))
        self.assertEqual(self.source.getpixel((20, 20)), (255, 0, 0, 128))
        self.assertEqual(self.source.getpixel((800, 0)), (0, 0, 0, 0))

    def test_matrix_and_both_native_seams(self):
        """【滤镜场景测试】【矩阵布局】十五区域与大区域互不遮挡，大区域横纵均超过五百一十二原图像素"""
        self.assertEqual(len(self.layout["matrix"]), 15)
        self.assertEqual({cell["strength"] for cell in self.layout["matrix"]}, {4, 14, 48})
        self.assertLess(max(cell["end"][1] for cell in self.layout["matrix"]), self.layout["edit_start"][1])
        for index in (0, 1):
            self.assertGreater((self.layout["edit_end"][index] - self.layout["edit_start"][index]) / self.layout["viewport"][2], 512)

    def test_valid_commit_matches_independent_mapping(self):
        """【滤镜场景测试】【有效坐标】准确日志通过独立视口映射并返回原记录"""
        self.assertIs(validate_commit(self.annotation, self.start, self.end, "blur", 14, self.layout), self.annotation)

    def test_fractional_output_uses_observed_buffer_dimensions(self):
        """【滤镜场景测试】【分数尺寸】实际逻辑截图允许一像素取整差，较大差异必须报告错误，不能从日志反推视口"""
        output = json.dumps({"winit": {"logical": {"width": 1066, "height": 666}}}).encode()
        with patch("wayland_filter_fixture.subprocess.check_output", return_value=output):
            layout = geometry({}, self.source, (1067, 667))
            for actual, expected in zip(layout["viewport"], (32, 51.40625, 0.626875)):
                self.assertAlmostEqual(actual, expected, places=9)
            with self.assertRaises(AssertionError):
                geometry({}, self.source, (1100, 667))

    def test_wrong_mode_strength_or_pointer_cannot_pass(self):
        """【滤镜场景测试】【输入失败检测】模式、强度与指针位置错误不能借助日志区域通过校验"""
        for change in ({"mode": "invert"}, {"strength": 48}, {"logical": (98.625, 113.8125, 219.875, 176.4375)}):
            with self.assertRaises(AssertionError):
                validate_commit(dict(self.annotation, **change), self.start, self.end, "blur", 14, self.layout)

    def test_wrong_native_mapping_cannot_pass(self):
        """【滤镜场景测试】【原图失败检测】日志中的错位、空范围与越界范围都必须报告错误"""
        for native in ((100, 100, 310, 200), (100, 100, 100, 200), (-1, 100, 300, 200), (100, 100, 1601, 200)):
            with self.assertRaises(AssertionError):
                validate_commit(dict(self.annotation, native=native), self.start, self.end, "blur", 14, self.layout)

    def test_log_reader_rejects_non_finite_coordinates(self):
        """【滤镜场景测试】【日志格式】提交读取保留编号及顺序，非有限坐标立即报告错误"""
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "application.log"
            prefix = "【滤镜】【区域提交】id=1 mode=blur strength=14 source=[100,100,300,200] logical="
            path.write_text(prefix + "[94.6250,113.8125,219.8750,176.4375]\n")
            self.assertEqual(read_commits(path)[0]["id"], 1)
            path.write_text(prefix + "[nan,113.8125,219.8750,176.4375]\n")
            with self.assertRaises(ValueError):
                read_commits(path)


if __name__ == "__main__":
    unittest.main()
