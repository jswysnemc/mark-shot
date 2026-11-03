"""【运行器测试】【比例参数】验证私有输出比例，不启动桌面进程或改写用户配置"""

from pathlib import Path
import unittest

from wayland_test_runtime import TestRuntime


class RuntimeScaleTests(unittest.TestCase):
    """【运行器测试】【参数边界】只检查构造参数，不建立 Wayland 或 X11 连接"""

    def test_default_and_subunit_scales_are_preserved(self):
        """【运行器测试】【合法比例】默认比例及小于一的比例保持原值，无参数、无返回值"""
        self.assertEqual(TestRuntime(Path("target/unused-runtime")).scale, 1.2)
        for scale in (0.5, 0.75, 1.0, 2.0, 4.0):
            with self.subTest(scale=scale):
                self.assertEqual(TestRuntime(Path("target/unused-runtime"), scale).scale, scale)

    def test_invalid_scales_are_rejected_before_starting_processes(self):
        """【运行器测试】【无效比例】非有限值及运行器范围外参数在启动进程前拒绝，无参数、无返回值"""
        for scale in (float("nan"), float("inf"), -1.0, 0.0, 0.25, 5.0):
            with self.subTest(scale=scale):
                with self.assertRaises(ValueError):
                    TestRuntime(Path("target/unused-runtime"), scale)


if __name__ == "__main__":
    unittest.main()
