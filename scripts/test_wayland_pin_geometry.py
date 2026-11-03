"""【贴图回归测试】【输入边界】不连接合成器，核对键盘就绪等待、像素断言与环境恢复"""

import importlib.util
import io
import os
from pathlib import Path
import unittest
from unittest.mock import patch

from PIL import Image
from wayland_pin_pointer import private_environment

spec = importlib.util.spec_from_file_location("pin_geometry_checks", Path(__file__).with_name("wayland-pin-geometry.py"))
checks = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checks)


class PinGeometryChecks(unittest.TestCase):
    """【贴图回归测试】【辅助规则】检查新验证工具不会丢失按下事件或放宽原图通道断言"""

    def test_virtual_keyboard_waits_before_key_press_and_modifier_release(self):
        """【贴图回归测试】【设备等待】虚拟键盘能力需要先到达客户端，按下前必须显式等待，无返回值"""
        with patch.object(checks.subprocess, "run") as run, patch.object(checks.time, "sleep"):
            checks.key({"WAYLAND_DISPLAY": "private"}, "ctrl+shift+c")
        self.assertEqual(run.call_args.args[0], ["wtype", "-s", "100", "-M", "ctrl", "-M", "shift",
                                               "-k", "c", "-m", "shift", "-m", "ctrl", "-s", "50"])
        self.assertEqual(run.call_args.kwargs["env"], {"WAYLAND_DISPLAY": "private"})

    def test_private_environment_is_restored_after_an_exception(self):
        """【贴图回归测试】【环境恢复】隔离输入异常退出也必须恢复调用者环境，无返回值"""
        with patch.dict(os.environ, {"WAYLAND_DISPLAY": "real"}, clear=True):
            with self.assertRaisesRegex(RuntimeError, "interrupted"):
                with private_environment({"WAYLAND_DISPLAY": "private"}):
                    self.assertEqual(dict(os.environ), {"WAYLAND_DISPLAY": "private"})
                    raise RuntimeError("interrupted")
            self.assertEqual(dict(os.environ), {"WAYLAND_DISPLAY": "real"})

    def test_source_assertion_checks_alpha_as_well_as_rgb(self):
        """【贴图回归测试】【完整通道】尺寸和 RGB 相同但透明度不同时必须失败，无返回值"""
        expected = Image.new("RGBA", (2, 2), (60, 90, 120, 128))
        actual = Image.new("RGBA", (2, 2), (60, 90, 120, 255))
        buffer = io.BytesIO()
        actual.save(buffer, format="PNG")
        with patch.object(checks, "key"), patch.object(checks.subprocess, "check_output", return_value=buffer.getvalue()):
            with self.assertRaisesRegex(AssertionError, "source export pixels"):
                checks.same_source({}, expected)

    def test_native_geometry_tolerance_does_not_accept_the_old_resize_regression(self):
        """【贴图回归测试】【反馈回退】旧缓冲反馈造成六乘十二像素回退，现有真实断言必须保持失败，无返回值"""
        with self.assertRaisesRegex(AssertionError, "geometry mismatch"):
            checks.check_rect((1290, 160, 1494, 568), (1290, 160, 1500, 580), tolerance=3)


if __name__ == "__main__":
    unittest.main()
