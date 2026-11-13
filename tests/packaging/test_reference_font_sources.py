"""【原生打包】【测试字体来源】验证离线字体准备严格使用源码组件，缺失或摘要不符时禁止联网替代"""

import hashlib
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import sys

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "reference_fonts", ROOT / "scripts/ci-reference-fonts.py"
)
FONTS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(FONTS)
sys.path.insert(0, str(ROOT / "scripts/packaging"))
from debian_build_dependencies import system_dependencies


class ReferenceFontSources(unittest.TestCase):
    def test_reference_verification_rejects_missing_menu_fonts_early(self):
        """【原生打包】【菜单依赖预检】无传参，Noto 校验正常但缺少 DejaVu 时立即失败，不等待完整 Rust 编译，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            config = Path(temporary) / "fonts.conf"
            outputs = [
                str(config.parent / "ttf" / f"NotoSans-{style}.ttf")
                for _ in range(3)
                for style in FONTS.FONTS
            ]
            outputs.extend(
                ["Noto Sans CJK SC|/system/cjk.ttc", "Noto Sans|/system/noto.ttf"]
            )
            with (
                patch.object(FONTS.subprocess, "run"),
                patch.object(FONTS.subprocess, "check_output", side_effect=outputs),
                patch("builtins.print"),
                self.assertRaisesRegex(RuntimeError, "DejaVu Sans"),
            ):
                FONTS.verify(config)

    def test_debian_declares_dejavu_menu_reference_fonts(self):
        """【原生打包】【菜单测试字体】无传参，确保最小构建环境安装 DejaVu 字体及 LGC 别名，不依赖完整主机预装，无返回值"""
        self.assertIn("fonts-dejavu-core", system_dependencies(ROOT).split(", "))

    def test_debian_tests_use_declared_python_and_offline_fonts(self):
        """【原生打包】【测试环境】无传参，检查实际 Debian 测试命令使用离线字体、私有目录及已声明解释器，无返回值"""
        command = subprocess.check_output(
            ["make", "--dry-run", "-f", "debian/rules", "override_dh_auto_test"],
            cwd=ROOT,
            text=True,
        )
        self.assertIn("--archive reference-fonts/NotoSans-v2.015.zip", command)
        self.assertIn("FONTCONFIG_FILE=", command)
        self.assertIn("/target/ci-private/config", command)
        self.assertIn("cargo test --workspace --locked --offline", command)
        self.assertIn("python3 (>= 3.11)", (ROOT / "debian/control").read_text())
        for path in (
            "src/translation/process/tests.rs",
            "src/translation/task/tests/image_tests.rs",
            "src/translation/task/tests/process_tests.rs",
            "src/features/ocr_result/translation/controller/lifecycle_tests.rs",
        ):
            self.assertIn("/usr/bin/python3 -c", (ROOT / path).read_text())
            self.assertNotIn("/usr/bin/python -c", (ROOT / path).read_text())

    def test_offline_archive_never_falls_back_to_network(self):
        """【原生打包】【离线回归】无传参，实际解开带摘要的字体样本，并拒绝缺失或被修改的源码归档，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "reference-fonts/NotoSans-v2.015.zip"
            source.parent.mkdir()
            data = b"pinned font sample"
            license = b"font license"
            with zipfile.ZipFile(source, "w") as archive:
                archive.writestr("NotoSans/hinted/ttf/NotoSans-Regular.ttf", data)
                archive.writestr("OFL.txt", license)
            digest = hashlib.sha256(source.read_bytes()).hexdigest()
            with (
                patch.object(FONTS, "ARCHIVE_SHA256", digest),
                patch.object(
                    FONTS, "FONTS", {"Regular": hashlib.sha256(data).hexdigest()}
                ),
                patch.object(
                    FONTS, "LICENSE_SHA256", hashlib.sha256(license).hexdigest()
                ),
                patch.object(
                    FONTS.subprocess,
                    "run",
                    side_effect=AssertionError("offline font setup must not download"),
                ),
            ):
                config = FONTS.prepare(root / "build/fonts", archive=source)
                self.assertEqual(
                    (config.parent / "ttf/NotoSans-Regular.ttf").read_bytes(), data
                )
                self.assertEqual((config.parent / "OFL.txt").read_bytes(), license)
                self.assertIn(str(config.parent / "ttf"), config.read_text())
                with self.assertRaises(FileNotFoundError):
                    FONTS.prepare(root / "missing", archive=root / "missing.zip")
                source.write_bytes(b"tampered source")
                with self.assertRaises(RuntimeError):
                    FONTS.prepare(config.parent, archive=source)


if __name__ == "__main__":
    unittest.main()
