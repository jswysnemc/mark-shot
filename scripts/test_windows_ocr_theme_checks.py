"""【Windows识别主题测试】【配置隔离】验证外部保存不覆盖业务字段，错误输入保持文件完整"""

from pathlib import Path
import tempfile
import unittest

from windows_ocr_theme_checks import save_mode


class ThemeSaveTests(unittest.TestCase):
    """【Windows识别主题测试】【原子保存】只使用临时目录验证测试驱动的配置契约"""

    def test_preserves_other_sections_and_roundtrips(self):
        """【Windows识别主题测试】【往返保留】无参数，保存明暗时保留正文配置和注释，无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "config.toml"
            text = '# kept\n[ui]\ntheme="light"\nlanguage="en"\n[history]\nenabled=false\n'
            path.write_text(text)
            save_mode(path, "dark")
            self.assertEqual(path.read_text(), text.replace('theme="light"', 'theme="dark"'))
            save_mode(path, "light")
            self.assertEqual(path.read_text(), text)
            self.assertEqual(list(Path(directory).iterdir()), [path])

    def test_ambiguous_or_missing_theme_does_not_write(self):
        """【Windows识别主题测试】【拒绝歧义】无参数，缺失或重复主题时不改动文件、不留下临时文件，无返回值"""
        for text in ('[ui]\nlanguage="en"\n', 'theme="light"\ntheme="dark"\n'):
            with self.subTest(text=text), tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / "config.toml"
                path.write_text(text)
                with self.assertRaises(AssertionError):
                    save_mode(path, "dark")
                self.assertEqual(path.read_text(), text)
                self.assertEqual(list(Path(directory).iterdir()), [path])


if __name__ == "__main__":
    unittest.main()
