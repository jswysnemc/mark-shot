"""【文字光标测试】【像素断言】检查时序回归能拒绝静态光标、持续叠加、错误颜色与失焦变化，不连接桌面"""

import importlib.util
from pathlib import Path
import unittest

from PIL import Image

SPEC = importlib.util.spec_from_file_location("text_caret", Path(__file__).with_name("wayland-text-caret.py"))
CHECKS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKS)


class CaretChecks(unittest.TestCase):
    """【文字光标测试】【固定相位】使用已知像素与时刻测试严格断言，不创建输入设备"""

    def frames(self, color=(255, 77, 77)):
        """【文字光标测试】【相位构造】按 color 返回十七个半秒交替相位，背景相同，无其他字形变化"""
        hidden = Image.new("RGB", (100, 60), (30, 34, 38))
        visible = hidden.copy()
        visible.paste(color, (60, 10, 61, 35))
        return [(index * 0.125, visible if index // 4 % 2 == 0 else hidden) for index in range(17)]

    def test_red_caret_half_period_passes(self):
        """【文字光标测试】【合法相位】红色窄光标半秒切换通过，并记录实际边界，无参数、无返回值"""
        result = CHECKS.blink(self.frames())
        self.assertEqual(result["bounds"], (60, 10, 61, 35))
        self.assertEqual(result["half_periods_seconds"], [0.5, 0.5, 0.5])

    def test_static_caret_fails(self):
        """【文字光标测试】【静态拒绝】单一相位不能冒充已实现闪烁，无参数、无返回值"""
        image = self.frames()[0][1]
        with self.assertRaisesRegex(AssertionError, "two stable phases"):
            CHECKS.blink([(index * 0.125, image) for index in range(17)])

    def test_shared_white_field_color_is_rejected(self):
        """【文字光标测试】【颜色分流】正文光标不能沿用白色字段颜色，无参数、无返回值"""
        with self.assertRaisesRegex(AssertionError, "actual red foreground"):
            CHECKS.blink(self.frames((255, 255, 255)))

    def test_glyph_accumulation_fails(self):
        """【文字光标测试】【重复合成】出现第三相位必须失败，不能放宽为存在像素变化，无参数、无返回值"""
        frames = self.frames()
        image = frames[9][1].copy()
        image.putpixel((20, 20), (80, 34, 38))
        frames[9] = (frames[9][0], image)
        with self.assertRaisesRegex(AssertionError, "two stable phases"):
            CHECKS.blink(frames)

    def test_unfocused_changes_fail(self):
        """【文字光标测试】【停止校验】焦点暂停期间不能继续切换像素，无参数、无返回值"""
        with self.assertRaisesRegex(AssertionError, "unfocused annotation"):
            CHECKS.stationary(self.frames())


if __name__ == "__main__":
    unittest.main()
