"""【OCR 翻译回归测试】【证据工具】验证私有配置、真实任务记录与原版命中点，拒绝启动桌面或连接外部服务"""

import contextlib
import io
import json
from pathlib import Path
import shlex
import tempfile
import tomllib
import unittest

from PIL import Image, ImageDraw
from wayland_ocr_translation_checks import configure, notice_rows, point, requests
from wayland_ocr_translation_command import execute


class TranslationChecksTests(unittest.TestCase):
    """【OCR 翻译回归测试】【工具边界】使用临时目录核对命令配置、输入快照与响应，无真实翻译服务"""

    def test_configure_keeps_quoted_paths_and_uses_the_explicit_local_command(self):
        """【OCR 翻译回归测试】【安全路径】配置路径含空格和单引号时 TOML 与 shell 参数仍完整，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "quoted ' path"
            root.mkdir()
            config = tomllib.loads(configure(root).read_text())
            translation = config["translation"]
            arguments = shlex.split(translation["command"])
            self.assertEqual(arguments[arguments.index("--root") + 1], str(root))
            self.assertEqual(arguments[-4:], ["--input", "{input}", "--target", "{targetLanguage}"])
            self.assertEqual(translation["provider"], "plugin:unavailable")
            self.assertEqual(translation["targetLanguage"], "English")
            self.assertFalse(config["history"]["enabled"])

    def test_fixture_records_the_confirmed_source_before_returning_success(self):
        """【OCR 翻译回归测试】【一次成功】真实命令读取输入并记录稳定目标，返回固定译文，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "mode").write_text("success")
            source = {"tokens": [{"text": "first"}, {"text": "second"}]}
            input_path = root / "input.json"
            input_path.write_text(json.dumps(source))
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                self.assertEqual(execute(root, input_path, "Esperanto"), 0)
            records = requests(root, 1)
            self.assertEqual(records[0]["source"], source)
            self.assertEqual(records[0]["target"], "Esperanto")
            self.assertIsNone(records[0]["child"])
            self.assertEqual(json.loads(output.getvalue()), {"tokens": [{"text": "translated first\nsecond"}]})

    def test_fixture_failure_records_the_request_without_a_success_body(self):
        """【OCR 翻译回归测试】【明确失败】错误命令保留请求证据并返回非零状态，不伪造成功响应，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "mode").write_text("error")
            input_path = root / "input.json"
            input_path.write_text('{"tokens":[{"text":"source"}]}')
            output, errors = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
                self.assertEqual(execute(root, input_path, "Russian"), 7)
            self.assertEqual(output.getvalue(), "")
            self.assertIn("fixture failure", errors.getvalue())
            self.assertEqual(requests(root, 1)[0]["mode"], "error")

    def test_action_points_match_qt_six_width_translation_geometry(self):
        """【OCR 翻译回归测试】【原版命中】全部宽度的译文编辑、语言和复制命中点位于真实 Qt 矩形内，无参数、无返回值"""
        fixture = json.loads((Path(__file__).resolve().parents[1] / "tests/fixtures/qt-ocr-translation.json").read_text())
        for case in fixture["cases"]:
            state = next(step["state"] for step in case["states"] if step["name"] == "busy")
            window = {"x": 0, "y": 0, "width": state["size"][0], "height": state["size"][1]}
            for action, name in [("action", "ocrTranslateButton"), ("language", "ocrLanguageCombo"), ("translation", "ocrTranslationEditor"), ("copy", "translation:ocrCopyButton")]:
                x, y = point(window, action)
                left, top, width, height = state["widgets"][name]["rect"]
                self.assertTrue(left <= x < left + width and top <= y < top + height, (case["width"], action))

    def test_notice_rows_distinguishes_wrapping_from_a_centered_single_line(self):
        """【OCR 翻译回归测试】【换行证据】同一提示矩形中，两行字形与居中单行分别返回二和一，无参数、无返回值"""
        window = {"x": 0, "y": 0, "width": 760, "height": 460}
        before = Image.new("RGB", (760, 460))
        wrapped = before.copy()
        draw = ImageDraw.Draw(wrapped)
        for top in [403, 420]:
            draw.rectangle((460, top, 700, top + 8), fill="white")
        self.assertEqual(notice_rows(before, wrapped, window), 2)
        single = before.copy()
        ImageDraw.Draw(single).rectangle((460, 412, 700, 423), fill="white")
        self.assertEqual(notice_rows(before, single, window), 1)


if __name__ == "__main__":
    unittest.main()
