#!/usr/bin/env python3

import argparse
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


HELPER_NAMES = (
    "mark-shot-ocr",
    "mark-shot-code-scan",
    "mark-shot-translate",
    "mark-shot-upload",
    "mark-shot-window-detection-niri",
    "mark-shot-window-detection-hyprland",
    "mark-shot-window-detection-gnome",
    "mark-shot-window-detection-kde",
)


class HelperScriptInstallTest(unittest.TestCase):
    """【打包测试】【辅助脚本】验证私有脚本组件的实际安装布局。"""

    install_options: argparse.Namespace

    @classmethod
    def setUpClass(cls) -> None:
        """【打包测试】【暂存安装】将脚本安装到隔离目录，无参数和返回值。"""
        # 1. 【打包测试】【暂存安装】使用包含空格的目录，并注册自动清理
        temporary_directory = tempfile.TemporaryDirectory(prefix="mark shot install ")
        cls.addClassCleanup(temporary_directory.cleanup)
        staging_directory = Path(temporary_directory.name)
        options = cls.install_options
        cls.helper_directory = staging_directory / options.helper_directory.lstrip("/")
        cls.bin_directory = staging_directory / options.bin_directory.lstrip("/")

        # 2. 【打包测试】【暂存安装】只安装脚本组件，不依赖主程序和插件的构建状态
        subprocess.run(
            [options.cmake, "--install", options.build_dir, "--component", "mark-shot-helpers"],
            env={**os.environ, "DESTDIR": str(staging_directory)},
            check=True,
            capture_output=True,
            text=True,
            timeout=20,
        )

    def test_helpers_are_installed_outside_bin(self) -> None:
        """【打包测试】【私有目录】验证八个脚本均可执行且不进入 bin，无参数和返回值。"""
        for helper_name in HELPER_NAMES:
            with self.subTest(helper_name=helper_name):
                script = self.helper_directory / helper_name
                self.assertTrue(script.is_file())
                self.assertTrue(os.access(script, os.X_OK))
                self.assertFalse((self.bin_directory / helper_name).exists())

    def test_niri_imports_colocated_shared_modules(self) -> None:
        """【打包测试】【共享模块】在脱离源码目录时导入 niri 脚本，无参数和返回值。"""
        script = self.helper_directory / "mark-shot-window-detection-niri"
        module_directory = self.helper_directory / "lib/mark_shot_window_detection"
        for module_name in ("__init__.py", "niri_dms.py", "niri_geometry.py"):
            with self.subTest(module_name=module_name):
                self.assertTrue((module_directory / module_name).is_file())
        result = subprocess.run(
            [sys.executable, "-I", "-c",
             "import runpy, sys; runpy.run_path(sys.argv[1], run_name='install_test')",
             str(script)],
            cwd=self.helper_directory,
            capture_output=True,
            text=True,
            timeout=5,
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_legacy_helpers_accept_direct_invocation(self) -> None:
        """【打包测试】【辅助调用】通过绝对路径调用四个业务脚本，无参数和返回值。"""
        for helper_name in HELPER_NAMES[:4]:
            with self.subTest(helper_name=helper_name):
                result = subprocess.run(
                    [str(self.helper_directory / helper_name), "--help"],
                    capture_output=True,
                    text=True,
                    timeout=5,
                )
                self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--cmake", required=True)
    parser.add_argument("--build-dir", required=True)
    parser.add_argument("--helper-directory", required=True)
    parser.add_argument("--bin-directory", required=True)
    HelperScriptInstallTest.install_options = parser.parse_args()
    unittest.main(argv=[sys.argv[0]])
