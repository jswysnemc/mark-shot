"""【截图历史测试】【辅助工具回归】确保测试断言本身拒绝路径、链接和像素差异"""

import io
from pathlib import Path
import tempfile
import unittest

from PIL import Image

from wayland_history_checks import equal_image, idat, owned


def png(color):
    """【辅助工具测试】【PNG 数据】返回 color 的小型 RGBA PNG 字节"""
    output = io.BytesIO()
    Image.new("RGBA", (3, 2), color).save(output, format="PNG")
    return output.getvalue()


class Checks(unittest.TestCase):
    """【辅助工具测试】【独立校验】验证历史文件枚举和全像素断言"""

    def test_pixel_assertions_include_alpha(self):
        """【辅助工具测试】【透明度差异】alpha 或 RGB 有任何变化都必须失败"""
        original = png((10, 20, 30, 91))
        equal_image(original, original)
        for changed in [png((10, 20, 30, 92)), png((11, 20, 30, 91))]:
            with self.assertRaises(AssertionError):
                equal_image(changed, original)
        self.assertTrue(idat(original))
        with self.assertRaises(ValueError):
            idat(original[:-1])

    def test_owned_names_require_valid_dates_and_no_links(self):
        """【辅助工具测试】【安全枚举】无关文件、错误日期和链接不属于历史"""
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            valid = directory / "capture-20261103123456123-0123456789abcdef0123456789abcdef.png"
            valid.write_bytes(png((10, 20, 30, 91)))
            (directory / "notes.png").write_bytes(b"notes")
            (directory / valid.name.replace("20261103", "20260230")).write_bytes(b"bad date")
            link = directory / valid.name.replace("123456123", "123456124")
            link.symlink_to(valid)
            self.assertEqual(owned(directory), [valid])


if __name__ == "__main__":
    unittest.main()
