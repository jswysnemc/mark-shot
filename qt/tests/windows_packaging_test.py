#!/usr/bin/env python3
"""使用临时目录和可控 PE 导入表验证 Windows 打包脚本。"""

from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import unittest
import zipfile

SCRIPTS = Path(__file__).resolve().parents[1] / "scripts"


class WindowsPackagingTest(unittest.TestCase):
    """不依赖 Windows 主机，验证真实部署和归档脚本的分支。"""

    def setUp(self) -> None:
        """建立每个测试独立的运行时与工具目录，无返回值。"""
        self.temporary = tempfile.TemporaryDirectory(prefix="mark-shot-package-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.tools = self.root / "tools"
        self.tools.mkdir()
        self.prefix = self.root / "ucrt64"
        (self.prefix / "bin").mkdir(parents=True)
        self.app = self.root / "app" / "bin"
        self.app.mkdir(parents=True)
        self.exe = self.app / "mark-shot.exe"
        self.exe.write_text("DLL Name: KERNEL32.dll\n", encoding="utf-8")
        self.exe.chmod(0o755)
        self.tool("objdump", '#!/usr/bin/env bash\ncat "$2"\n')
        self.tool("windeployqt", "#!/usr/bin/env bash\nexit 0\n")
        self.env = dict(os.environ, PATH=f"{self.tools}:{os.environ['PATH']}",
                        MINGW_PREFIX=str(self.prefix))
        self.env.pop("MARK_SHOT_FFMPEG_PREFIX", None)
        self.env.pop("MARK_SHOT_RUNTIME_EXTRA_PREFIX", None)
        self.env.pop("MARK_SHOT_QT_EXTRA_PLUGINS", None)

    def tool(self, name: str, body: str) -> None:
        """创建名称为 name、内容为 body 的测试工具，无返回值。"""
        path = self.tools / name
        path.write_text(body, encoding="utf-8")
        path.chmod(0o755)

    def deploy(self) -> subprocess.CompletedProcess:
        """运行真实部署脚本，返回退出码与标准输出。"""
        return subprocess.run(["bash", str(SCRIPTS / "windows-deploy-runtime.sh"),
                               str(self.app), str(self.exe)], env=self.env,
                              text=True, capture_output=True, timeout=10)

    def test_missing_non_system_dependency_fails(self) -> None:
        """缺少普通依赖必须阻止发行包生成，无返回值。"""
        self.exe.write_text("DLL Name: libmissing-1.dll\n", encoding="utf-8")
        result = self.deploy()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("libmissing-1.dll", result.stderr)

    def test_dependency_names_are_case_insensitive(self) -> None:
        """按照 Windows 规则解析大小写不同的导入名，无返回值。"""
        self.exe.write_text("DLL Name: Helper.DLL\n", encoding="utf-8")
        (self.prefix / "bin/helper.dll").write_text("DLL Name: USER32.dll\n", encoding="utf-8")
        result = self.deploy()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.app / "helper.dll").is_file())

    def test_prefix_dependency_shadowing_system32_is_bundled(self) -> None:
        """构建前缀提供的 DLL 即使在 System32 有同名旧版也必须随包发布，无返回值。"""
        windir = self.root / "Windows"
        (windir / "System32").mkdir(parents=True)
        (windir / "System32/onnxruntime.dll").write_text("windows 1.17", encoding="utf-8")
        (windir / "System32/VCRUNTIME140.dll").write_text("system only", encoding="utf-8")
        self.exe.write_text("DLL Name: onnxruntime.dll\nDLL Name: VCRUNTIME140.dll\n", encoding="utf-8")
        (self.prefix / "bin/onnxruntime.dll").write_text("DLL Name: KERNEL32.dll\n", encoding="utf-8")
        self.env["WINDIR"] = str(windir)
        result = self.deploy()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.app / "onnxruntime.dll").read_text(encoding="utf-8"),
                         "DLL Name: KERNEL32.dll\n")
        self.assertFalse((self.app / "VCRUNTIME140.dll").exists())

    def test_private_compatibility_runtime_keeps_both_abis(self) -> None:
        """【Windows测试】【并存依赖】旧 ABI 从独立目录补齐，新 ABI 仍使用工具链版本。"""
        compatibility = self.root / "compatibility"
        (compatibility / "bin").mkdir(parents=True)
        old_name = "libabsl_hash-2605.0.0.dll"
        new_name = "libabsl_hash-2608.0.0.dll"
        self.exe.write_text(f"DLL Name: {old_name}\nDLL Name: {new_name}\n", encoding="utf-8")
        (compatibility / "bin" / old_name).write_text("DLL Name: KERNEL32.dll\n", encoding="utf-8")
        (self.prefix / "bin" / new_name).write_text("DLL Name: USER32.dll\n", encoding="utf-8")
        missing = self.deploy()
        self.assertNotEqual(missing.returncode, 0)
        self.assertIn(old_name, missing.stderr)
        self.env["MARK_SHOT_RUNTIME_EXTRA_PREFIX"] = str(compatibility)
        result = self.deploy()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.app / old_name).is_file())
        self.assertTrue((self.app / new_name).is_file())
        self.assertFalse((self.prefix / "bin" / old_name).exists())

    def test_mismatched_minimal_ffmpeg_does_not_fall_back(self) -> None:
        """后端 ABI 与裁剪库不一致时禁止悄悄使用完整 FFmpeg，无返回值。"""
        self.exe.write_text("DLL Name: avcodec-62.dll\n", encoding="utf-8")
        (self.prefix / "bin/avcodec-62.dll").write_text("full distribution", encoding="utf-8")
        minimal = self.root / "minimal"
        (minimal / "bin").mkdir(parents=True)
        (minimal / "bin/avcodec-61.dll").write_text("wrong ABI", encoding="utf-8")
        self.env["MARK_SHOT_FFMPEG_PREFIX"] = str(minimal)
        result = self.deploy()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("avcodec-62.dll", result.stderr)

    def test_ocr_test_runtime_includes_transitive_dependencies(self) -> None:
        """【Windows测试】【依赖部署】把插件间接依赖放到测试入口旁，无参数和返回值。"""
        build = self.root / "build"
        self.app = build / "qt"
        self.app.mkdir(parents=True)
        self.exe = build / "mark-shot.exe"
        self.exe.write_text("DLL Name: KERNEL32.dll\n", encoding="utf-8")
        self.exe.chmod(0o755)
        plugin_dir = build / "qt/plugins/ocr-rapid"
        plugin_dir.mkdir(parents=True)
        (plugin_dir / "mark-shot-ocr-rapid.dll").write_text(
            "DLL Name: onnxruntime.dll\n", encoding="utf-8")
        imports = "DLL Name: libonnx.dll\nDLL Name: KERNEL32.dll\n"
        # 1. 【Windows测试】【已有运行时】模拟 CMake 已经复制 ONNX 本体，但没有递归部署依赖
        (self.app / "onnxruntime.dll").write_text(imports, encoding="utf-8")
        (self.prefix / "bin/onnxruntime.dll").write_text(imports, encoding="utf-8")
        (self.prefix / "bin/libonnx.dll").write_text("DLL Name: libprotobuf.dll\n", encoding="utf-8")
        (self.prefix / "bin/libprotobuf.dll").write_text("DLL Name: KERNEL32.dll\n", encoding="utf-8")
        result = self.deploy()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.app / "libonnx.dll").is_file())
        self.assertTrue((self.app / "libprotobuf.dll").is_file())

    def test_dynamic_ocr_runtime_is_explicitly_bundled(self) -> None:
        """【Windows测试】【动态运行库】插件不导入 ORT 时仍部署本体及间接依赖，无返回值。"""
        plugins = self.app.parent / "lib/mark-shot/plugins"
        plugins.mkdir(parents=True)
        (plugins / "mark-shot-ocr-rapid.dll").write_text("DLL Name: KERNEL32.dll\n")
        imports = "DLL Name: libprotobuf.dll\n"
        (self.prefix / "bin/onnxruntime.dll").write_text(imports)
        (self.prefix / "bin/libprotobuf.dll").write_text("DLL Name: KERNEL32.dll\n")
        result = self.deploy()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.app / "onnxruntime.dll").is_file())
        self.assertTrue((self.app / "libprotobuf.dll").is_file())

    def test_missing_dynamic_ocr_runtime_rejects_package(self) -> None:
        """【Windows测试】【缺失运行库】含 OCR 插件却缺少构建运行库时必须阻止发布，无返回值。"""
        plugins = self.app / "plugins"
        plugins.mkdir()
        (plugins / "mark-shot-ocr-rapid.dll").write_text("DLL Name: KERNEL32.dll\n")
        result = self.deploy()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("ONNX Runtime", result.stderr)

    def test_dynamic_ocr_replaces_stale_staged_runtime(self) -> None:
        """【Windows测试】【运行库版本】已部署的旧 ORT 必须由构建所用版本替换，无返回值。"""
        (self.app / "mark-shot-ocr-rapid.dll").write_text("DLL Name: KERNEL32.dll\n")
        (self.app / "onnxruntime.dll").write_text("obsolete runtime\n")
        expected = "DLL Name: KERNEL32.dll\n"
        (self.prefix / "bin/onnxruntime.dll").write_text(expected)
        result = self.deploy()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.app / "onnxruntime.dll").read_text(), expected)

    def test_test_runtime_requests_offscreen_platform(self) -> None:
        """【Windows测试】【平台插件】测试部署显式包含离屏插件，普通发行包不增加该插件，无返回值。"""
        arguments = self.root / "deploy-arguments"
        self.tool("windeployqt", '#!/usr/bin/env bash\nprintf "%s\\n" "$@" > "$DEPLOY_ARGUMENTS"\n')
        self.env["DEPLOY_ARGUMENTS"] = str(arguments)
        self.env["MARK_SHOT_QT_EXTRA_PLUGINS"] = "qoffscreen,qminimal"
        result = self.deploy()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("--include-plugins\nqoffscreen,qminimal\n", arguments.read_text())
        self.env.pop("MARK_SHOT_QT_EXTRA_PLUGINS")
        result = self.deploy()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn("--include-plugins", arguments.read_text())

    def test_closure_includes_lazy_backend(self) -> None:
        """主程序没有媒体导入时，仍部署按需后端的间接依赖，无返回值。"""
        backend = self.app / "recording"
        backend.mkdir()
        (backend / "mark-shot-recording-libav.dll").write_text(
            "DLL Name: avcodec-62.dll\n", encoding="utf-8")
        minimal = self.root / "minimal"
        (minimal / "bin").mkdir(parents=True)
        (minimal / "bin/avcodec-62.dll").write_text("DLL Name: libx264.dll\n", encoding="utf-8")
        (self.prefix / "bin/libx264.dll").write_text("DLL Name: KERNEL32.dll\n", encoding="utf-8")
        self.env["MARK_SHOT_FFMPEG_PREFIX"] = str(minimal)
        result = self.deploy()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.app / "avcodec-62.dll").is_file())
        self.assertTrue((self.app / "libx264.dll").is_file())

    def test_invalid_archive_keeps_existing_output(self) -> None:
        """无效归档格式不能删除已有文件，无返回值。"""
        archive = self.root / "existing.invalid"
        archive.write_bytes(b"keep existing artifact")
        result = subprocess.run(["bash", str(SCRIPTS / "windows-package.sh"),
                                 "--stage-dir", str(self.app.parent), "--archive-only",
                                 "--archive", str(archive)], capture_output=True, timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue(archive.is_file())
        self.assertEqual(archive.read_bytes(), b"keep existing artifact")

    def test_archive_cannot_be_inside_stage(self) -> None:
        """禁止归档递归包含自身，无返回值。"""
        archive = self.app / "recursive.zip"
        result = subprocess.run(["bash", str(SCRIPTS / "windows-package.sh"),
                                 "--stage-dir", str(self.app.parent), "--archive-only",
                                 "--archive", str(archive)], capture_output=True, timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(archive.exists())

    @unittest.skipUnless(shutil.which("zip"), "zip is not installed")
    def test_archive_only_preserves_staged_files(self) -> None:
        """归档阶段不能再次剥离或修改已签名文件，无返回值。"""
        archive = self.root / "release.zip"
        original = self.exe.read_bytes()
        result = subprocess.run(["bash", str(SCRIPTS / "windows-package.sh"),
                                 "--stage-dir", str(self.app.parent), "--archive-only",
                                 "--archive", str(archive)], capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.exe.read_bytes(), original)
        with zipfile.ZipFile(archive) as bundle:
            self.assertEqual(bundle.read("bin/mark-shot.exe"), original)

    def test_unowned_stage_is_not_deleted(self) -> None:
        """部署不能清空调用者已有但不属于打包脚本的目录，无返回值。"""
        build = self.root / "build"
        build.mkdir()
        (build / "CMakeCache.txt").write_text("CMAKE_BUILD_TYPE:STRING=Release\n", encoding="utf-8")
        result = subprocess.run(["bash", str(SCRIPTS / "windows-package.sh"),
                                 "--build-dir", str(build), "--stage-dir", str(self.app.parent)],
                                capture_output=True, timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue(self.exe.is_file())

    def test_invalid_import_table_fails(self) -> None:
        """解析 PE 失败必须中止部署，无返回值。"""
        self.tool("objdump", "#!/usr/bin/env bash\nexit 2\n")
        self.assertNotEqual(self.deploy().returncode, 0)

    def test_missing_schannel_runtime_is_rejected(self) -> None:
        """选择系统 TLS 后端后必须确认它确实进入发行目录，无返回值。"""
        tls = self.prefix / "share/qt6/plugins/tls"
        tls.mkdir(parents=True)
        (tls / "qschannelbackend.dll").write_text("", encoding="utf-8")
        result = self.deploy()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Schannel", result.stderr)

    @unittest.skipUnless(shutil.which("7z"), "7z is not installed")
    def test_solid_archive_is_readable(self) -> None:
        """验证 LZMA2 归档可正常读取，无返回值。"""
        archive = self.root / "release.7z"
        result = subprocess.run(["bash", str(SCRIPTS / "windows-package.sh"),
                                 "--stage-dir", str(self.app.parent), "--archive-only",
                                 "--archive", str(archive)], capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        check = subprocess.run(["7z", "t", str(archive)], capture_output=True, timeout=10)
        self.assertEqual(check.returncode, 0, check.stderr)


if __name__ == "__main__":
    unittest.main()
