"""【原生发布】【源码测试】验证原生标签隔离、固定提交及 Debian 离线组件的来源"""

from pathlib import Path
import shutil
import subprocess
import sys
import struct
import tarfile
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/packaging"))
from prepare_aur import prepare as prepare_aur
from prepare_source import prepare
from release_context import resolve
from debian_build_dependencies import system_dependencies, verify
from prepare_flatpak_source import render_icons


def repository(directory):
    """【原生发布】【仓库样本】在 directory 创建最小原生提交和标签，返回提交摘要"""
    directory.mkdir()
    shutil.copytree(ROOT / "debian", directory / "debian")
    (directory / "packaging").mkdir()
    (directory / "packaging/placeholder").write_text("source\n")
    (directory / "Cargo.toml").write_text(
        '[package]\nname="mark-shot"\nversion="0.2.0"\n[features]\nocr-dynamic=[]\n[dependencies]\nshot-media={path="crates/shot-media"}\n'
    )
    (directory / "Cargo.lock").write_text("version=4\n")
    subprocess.run(["git", "init", "-q", str(directory)], check=True)
    subprocess.run(["git", "-C", str(directory), "add", "."], check=True)
    subprocess.run(
        [
            "git",
            "-C",
            str(directory),
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-qm",
            "test: native source",
        ],
        check=True,
    )
    commit = subprocess.check_output(
        ["git", "-C", str(directory), "rev-parse", "HEAD"], text=True
    ).strip()
    subprocess.run(["git", "-C", str(directory), "tag", "native-v0.2.0"], check=True)
    return commit


class ReleaseSources(unittest.TestCase):
    @unittest.skipUnless(shutil.which("rsvg-convert"), "SVG renderer is not installed")
    def test_flatpak_icons_have_explicit_raster_dimensions(self):
        """【原生打包】【图标测试】无传参，实际渲染 AppStream 图标并校验 PNG 尺寸，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            icons = root / "assets/hicolor/scalable/apps"
            icons.mkdir(parents=True)
            shutil.copy2(ROOT / "assets/hicolor/scalable/apps/mark-shot.svg", icons)
            paths = render_icons(root)
            for path, size in zip(paths, (128, 256)):
                data = path.read_bytes()
                self.assertEqual(data[:8], b"\x89PNG\r\n\x1a\n")
                self.assertEqual(struct.unpack_from(">II", data, 16), (size, size))

    def test_debian_autotools_steps_keep_vendor_files_unchanged(self):
        """【原生打包】【vendor保护】无传参，执行实际 Debian 规则入口，拒绝自动改写第三方构建文件，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            shutil.copytree(ROOT / "debian", root / "debian")
            source = root / "vendor/onig_sys/oniguruma/config.sub"
            source.parent.mkdir(parents=True)
            source.write_text("immutable upstream config\n")
            before = source.read_bytes()
            for target in (
                "override_dh_update_autotools_config",
                "override_dh_autoreconf",
            ):
                subprocess.run(
                    ["make", "-f", "debian/rules", target],
                    cwd=root,
                    check=True,
                    capture_output=True,
                )
                self.assertEqual(source.read_bytes(), before)

    def test_ci_toolchain_check_keeps_source_constraints(self):
        """【原生打包】【工具链测试】无传参，确认 CI 分离校验不改正式源码约束，拒绝 Rust 1.93，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            shutil.copytree(ROOT / "debian", root / "debian")
            (root / "Cargo.toml").write_text('[package]\nrust-version="1.95"\n')
            control = (root / "debian/control").read_text()
            dependencies = system_dependencies(root)
            self.assertNotIn("rustc", dependencies)
            self.assertNotIn("cargo", dependencies)
            self.assertIn("libpipewire-0.3-dev", dependencies)
            with patch(
                "debian_build_dependencies.subprocess.check_output",
                return_value="cargo 1.93.1 (test)\n",
            ):
                with self.assertRaises(ValueError):
                    verify(root)
            with (
                patch(
                    "debian_build_dependencies.subprocess.check_output",
                    side_effect=["cargo 1.99.1 (test)\n", "rustc 1.99.1 (test)\n"],
                ),
                patch("debian_build_dependencies.subprocess.run") as check,
            ):
                verify(root)
                self.assertEqual(
                    check.call_args.args[0], ["dpkg-checkbuilddeps", "-d", dependencies]
                )
            self.assertEqual((root / "debian/control").read_text(), control)

    def test_only_native_tags_and_matching_versions(self):
        """【原生发布】【边界测试】无传参，拒绝 Qt 标签、错误版本及无效引用，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "repo"
            commit = repository(root)
            context = resolve(root, "native-v0.2.0", commit, publish=True)
            self.assertEqual(context.commit, commit)
            self.assertTrue(context.publish)
            for tag in ("v0.2.0", "native-v0.3.0", "native-v../../bad"):
                with self.assertRaises(ValueError):
                    resolve(root, tag, commit)
            with self.assertRaises(ValueError):
                resolve(root, "native-v0.2.0", "HEAD;ls")
            (root / "change").write_text("new source")
            subprocess.run(["git", "-C", str(root), "add", "change"], check=True)
            subprocess.run(
                [
                    "git",
                    "-C",
                    str(root),
                    "-c",
                    "user.name=Test",
                    "-c",
                    "user.email=test@example.com",
                    "commit",
                    "-qm",
                    "test: later source",
                ],
                check=True,
            )
            with self.assertRaises(ValueError):
                resolve(root, "native-v0.2.0", "HEAD", publish=True)

    def test_qt_source_is_rejected(self):
        """【原生发布】【源码测试】无传参，拒绝具有 Cargo.toml 但没有原生模块的提交，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "repo"
            repository(root)
            (root / "Cargo.toml").write_text(
                '[package]\nname="mark-shot"\nversion="0.2.0"\n'
            )
            subprocess.run(["git", "-C", str(root), "add", "Cargo.toml"], check=True)
            subprocess.run(
                [
                    "git",
                    "-C",
                    str(root),
                    "-c",
                    "user.name=Test",
                    "-c",
                    "user.email=test@example.com",
                    "commit",
                    "-qm",
                    "test: Qt source",
                ],
                check=True,
            )
            with self.assertRaises(ValueError):
                resolve(root, "native-v0.2.0", "HEAD")

    def test_debian_components_and_source_reconstruction(self):
        """【原生源码】【组件测试】无传参，验证上游、vendor 和媒体组件可由 dpkg-source 重建，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            root = directory / "repo"
            commit = repository(root)
            original_run = subprocess.run

            def vendor(command, **kwargs):
                """【原生源码】【依赖样本】按 command 模拟外部依赖取得，其他命令使用 kwargs 原样执行，返回结果"""
                if str(command[1]).endswith("vendor_sources.sh"):
                    source = Path(command[2])
                    for name in ("vendor", "media", "packaging/licenses"):
                        (source / name).mkdir(parents=True)
                    (source / "vendor/test.rs").write_text("fn main() {}\n")
                    (source / "media/SOURCE-SHA256SUMS").write_text("source manifest\n")
                    (source / "packaging/licenses/DEPENDENCIES.txt").write_text(
                        "dependency licenses\n"
                    )
                    return subprocess.CompletedProcess(command, 0)
                return original_run(command, **kwargs)

            with patch("prepare_source.subprocess.run", side_effect=vendor):
                result = prepare(root, commit, "0.2.0", directory / "output", "debian")
            source = result["source_root"]
            self.assertFalse((source / "SOURCE-COMMIT").exists())
            self.assertEqual(
                (source / "debian/source-commit").read_text().strip(), commit
            )
            self.assertTrue(
                (source / "debian/dependency-licenses/DEPENDENCIES.txt").is_file()
            )
            for component in ("vendor", "media"):
                with tarfile.open(
                    directory / f"output/mark-shot_0.2.0.orig-{component}.tar.xz"
                ) as stream:
                    self.assertEqual(stream.getmembers()[0].name, component)
            if shutil.which("dpkg-source"):
                subprocess.run(
                    ["dpkg-source", "-b", str(source)],
                    cwd=directory / "output",
                    check=True,
                    capture_output=True,
                )
                reconstructed = directory / "reconstructed"
                subprocess.run(
                    [
                        "dpkg-source",
                        "--no-check",
                        "-x",
                        str(directory / "output/mark-shot_0.2.0-1.dsc"),
                        str(reconstructed),
                    ],
                    check=True,
                    capture_output=True,
                )
                self.assertTrue((reconstructed / "vendor/test.rs").is_file())
                self.assertTrue((reconstructed / "media/SOURCE-SHA256SUMS").is_file())

    def test_aur_template_is_pinned_and_validated(self):
        """【原生发布】【AUR测试】无传参，检查固定提交、版本替换和非法修订号拦截，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "repo"
            repository(root)
            context = resolve(root, "native-v0.2.0", "HEAD")
            result = prepare_aur(ROOT, context, Path(temporary) / "aur")
            self.assertIn(f"_commit={context.commit}", result.read_text())
            self.assertNotIn("@pkgver@", result.read_text())
            with self.assertRaises(ValueError):
                prepare_aur(ROOT, context, Path(temporary) / "bad", revision="1;ls")


if __name__ == "__main__":
    unittest.main()
