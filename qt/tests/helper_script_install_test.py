#!/usr/bin/env python3

import argparse
import os
from pathlib import Path
import re
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

    def test_debian_smoke_compiles_installed_scripts(self) -> None:
        """【打包测试】【安装检查】使用 Debian 检查脚本中的实际路径编译已安装文件，无参数和返回值。"""
        # 1. 【打包测试】【路径读取】读取检查脚本的文件参数，保留旧版 bin 路径以复现目录错配
        repository_root = Path(__file__).resolve().parents[2]
        smoke_script = (repository_root / "debian/tests/smoke").read_text(encoding="utf-8")
        helper_paths = re.findall(
            r"/usr/(?:bin|libexec/mark-shot)/mark-shot-[a-z-]+", smoke_script
        )
        self.assertEqual(sorted(Path(path).name for path in helper_paths), sorted(HELPER_NAMES))
        module_paths = re.findall(
            r"/usr/libexec/mark-shot/lib/mark_shot_window_detection/[a-z_]+\.py", smoke_script
        )
        self.assertEqual(
            sorted(Path(path).name for path in module_paths),
            ["__init__.py", "niri_dms.py", "niri_geometry.py"],
        )

        # 2. 【打包测试】【暂存映射】将 Debian 安装前缀映射到当前构建的隔离前缀
        install_prefix = self.bin_directory.parent
        installed_paths = [
            str(install_prefix / path.removeprefix("/usr/"))
            for path in helper_paths + module_paths
        ]

        # 3. 【打包测试】【语法编译】执行与安装检查相同的编译命令，路径不存在时必须失败
        with tempfile.TemporaryDirectory(prefix="mark-shot-smoke-cache.") as cache_directory:
            result = subprocess.run(
                [sys.executable, "-m", "py_compile", *installed_paths],
                env={**os.environ, "PYTHONPYCACHEPREFIX": cache_directory},
                capture_output=True,
                text=True,
                timeout=10,
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
