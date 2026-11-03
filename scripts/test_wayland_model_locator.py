"""【Wayland 测试】【模型定位回归】验证推荐条目移动后仍能定位下载按钮，不向错误图像发送点击"""

import unittest

from PIL import Image, ImageDraw

from wayland_model_locator import download_position


def guide(origin, icons=True):
    """【定位测试】【引导图像】按 origin 绘制默认模型列表；icons 控制下载列，返回 RGB 图像与目标坐标"""
    image = Image.new("RGB", (1300, 700), (10, 10, 10))
    painter = ImageDraw.Draw(image)
    left, top = origin[0] + 8, origin[1] + 52
    painter.rounded_rectangle((left, top, left + 304, top + 44), radius=4, outline=(60, 140, 105), width=1)
    painter.line((origin[0] + 18, origin[1] + 22, origin[0] + 198, origin[1] + 22), fill=(75, 191, 155))
    button_x = left + 304 - 18
    if icons:
        for index in range(7):
            y = top + 22 + index * 44
            painter.line((button_x, y - 5, button_x, y + 3), fill=(229, 237, 239))
            painter.line((button_x - 4, y - 1, button_x, y + 3, button_x + 4, y - 1), fill=(229, 237, 239))
            painter.line((button_x - 5, y + 2, button_x - 5, y + 6, button_x + 5, y + 6, button_x + 5, y + 2), fill=(229, 237, 239))
    return image, (button_x, top + 22 + 4 * 44)


class ModelLocatorTests(unittest.TestCase):
    """【定位测试】【断言集合】覆盖平移、无引导和防止误匹配"""

    def test_moving_the_popover_keeps_the_download_hit_inside_its_button(self):
        """【定位测试】【弹层平移】不同 origin 下返回的坐标均处于目标按钮内，无返回值"""
        for origin in ((80, 20), (478, 5), (650, 100)):
            with self.subTest(origin=origin):
                image, expected = guide(origin)
                position = download_position(image, 4)
                self.assertLessEqual(abs(position[0] - expected[0]), 5)
                self.assertLessEqual(abs(position[1] - expected[1]), 2)

    def test_missing_guide_is_rejected(self):
        """【定位测试】【空画面】没有模型列表时拒绝构造点击坐标，无返回值"""
        with self.assertRaises(ValueError):
            download_position(Image.new("RGB", (600, 400)), 4)

    def test_translucent_original_theme_keeps_the_download_target(self):
        """【定位测试】【半透明主题】原版青色强调合成后的弱边框仍须通过下载列校验，无返回值"""
        image, expected = guide((80, 20))
        pixels = image.load()
        for y in range(image.height):
            for x in range(image.width):
                if pixels[x, y] == (60, 140, 105):
                    pixels[x, y] = (37, 52, 54)
        actual = download_position(image, 4)
        self.assertLessEqual(abs(actual[0] - expected[0]), 5)
        self.assertLessEqual(abs(actual[1] - expected[1]), 2)

    def test_a_green_border_without_download_icons_is_rejected(self):
        """【定位测试】【错误图像】仅有绿色矩形而没有下载列时拒绝点击，无返回值"""
        image, _ = guide((80, 20), icons=False)
        with self.assertRaises(ValueError):
            download_position(image, 4)

    def test_invalid_model_index_is_rejected(self):
        """【定位测试】【条目边界】负索引和超出画面范围的条目均返回错误，无返回值"""
        image, _ = guide((80, 20))
        for index in (-1, 40):
            with self.subTest(index=index), self.assertRaises(ValueError):
                download_position(image, index)


if __name__ == "__main__":
    unittest.main()
