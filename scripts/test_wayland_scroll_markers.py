"""【长截图双框测试】【检测边界】验证纵横短边线与原图噪声的区别"""

import random
import unittest
from PIL import Image, ImageDraw
from wayland_scroll_markers import check


class MarkerTests(unittest.TestCase):
    """【长截图双框测试】【固定画面】用确定像素验证双色检测，不连接桌面"""

    def test_vertical_and_horizontal_markers(self):
        """【长截图双框测试】【方向覆盖】无参数，十二像素以上纵横边线均可定位，无返回值"""
        buttons = {"axis": (30, 100), "cancel": (170, 100), "save": (100, 100)}
        for horizontal in [False, True]:
            with self.subTest(horizontal=horizontal):
                image = Image.new("RGB", (200, 120), "white")
                draw = ImageDraw.Draw(image)
                draw.rectangle((50, 10, 65, 60), outline=(45, 212, 191), width=2)
                draw.line((90, 20, 90, 34) if horizontal else (90, 20, 130, 20), fill=(211, 204, 54), width=2)
                result = check(image, buttons)
                self.assertLess(result["cyan"][0], result["yellow"][0])

    def test_source_noise_and_single_color_do_not_pass(self):
        """【长截图双框测试】【噪声拒绝】无参数，随机原图或单一青色边框不能冒充双框，无返回值"""
        randomizer = random.Random(42)
        image = Image.frombytes("RGB", (200, 120), randomizer.randbytes(200 * 120 * 3))
        buttons = {"axis": (30, 100), "cancel": (170, 100), "save": (100, 100)}
        with self.assertRaises(AssertionError):
            check(image, buttons)
        image.paste("white", (0, 0, 200, 120))
        ImageDraw.Draw(image).rectangle((30, 20, 170, 60), outline=(45, 212, 191), width=2)
        with self.assertRaises(AssertionError):
            check(image, buttons)


if __name__ == "__main__":
    unittest.main()
