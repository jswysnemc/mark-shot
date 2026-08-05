#!/usr/bin/env python3

from pathlib import Path
import re
import unittest


PROJECT_ROOT = Path(__file__).resolve().parents[1]


class PackagingConfigurationTest(unittest.TestCase):
    """验证 Linux 发布产物覆盖声明支持的发行版。"""

    def test_deb_matrix_contains_native_distribution_targets(self) -> None:
        """
        验证 DEB 矩阵在目标发行版环境中分别构建。

        @return 无返回值。
        """
        # 1. 读取 DEB 发布工作流
        workflow = (PROJECT_ROOT / ".github/workflows/release-deb.yml").read_text(
            encoding="utf-8"
        )

        # 2. 定义需要原生构建的发行版及产物后缀
        expected_targets = {
            "debian12": ("debian:12", 'asset_suffix: ""'),
            "debian13": ("debian:13", 'asset_suffix: ".debian13"'),
            "ubuntu2404": ("ubuntu:24.04", 'asset_suffix: ".ubuntu24.04"'),
            "ubuntu2604": ("ubuntu:26.04", 'asset_suffix: ".ubuntu26.04"'),
        }

        # 3. 验证各发行版矩阵项和目标容器
        for target, (image, suffix) in expected_targets.items():
            with self.subTest(target=target):
                target_block = re.search(
                    rf"- target: {target}\n(?P<body>(?:\s+.*\n){{1,8}})", workflow
                )
                self.assertIsNotNone(target_block)
                body = target_block.group("body")
                self.assertIn(f"image: {image}", body)
                self.assertIn(suffix, body)

        # 4. 验证 DEB 在目标容器中执行安装检查
        self.assertIn("Verify package installs on target", workflow)

    def test_appimage_uses_compatible_glibc_baseline(self) -> None:
        """
        验证 AppImage 使用稳定基线并限制 glibc 符号版本。

        @return 无返回值。
        """
        # 1. 读取 AppImage 发布工作流
        workflow = (PROJECT_ROOT / ".github/workflows/release-appimage.yml").read_text(
            encoding="utf-8"
        )

        # 2. 验证构建基线、glibc 上限和源码引用
        self.assertRegex(workflow, r"container:\s*\n\s+image: debian:12")
        self.assertIn("ninja-build", workflow)
        self.assertIn('MAX_GLIBC_VERSION: "2.36"', workflow)
        self.assertIn("Assert glibc compatibility", workflow)
        self.assertIn("checkout_ref:", workflow)
        self.assertIn("ref: ${{ steps.version.outputs.checkout_ref }}", workflow)
        self.assertIn("appimage-ubuntu2404:", workflow)
        self.assertIn('image: ubuntu:24.04', workflow)

        # 3. 验证 Ubuntu 24.04 启动任务包含全部外部系统依赖
        runtime_job = workflow.split("  appimage-ubuntu2404:", maxsplit=1)[1]
        for package in (
            "libasound2t64",
            "libcom-err2",
            "libdrm2",
            "libegl1",
            "libfontconfig1",
            "libfreetype6",
            "libfribidi0",
            "libgcc-s1",
            "libgl1",
            "libglx0",
            "libgmp10",
            "libgpg-error0",
            "libharfbuzz0b",
            "libice6",
            "libopengl0",
            "libpipewire-0.3-0t64",
            "libsm6",
            "libstdc++6",
            "libwayland-client0",
            "libx11-6",
            "libx11-xcb1",
            "libxcb-dri3-0",
            "libxcb1",
            "xvfb",
            "zlib1g",
        ):
            with self.subTest(package=package):
                self.assertIn(package, runtime_job)

        # 4. 验证最终产物在虚拟 X11 会话中实际启动
        self.assertIn(
            'xvfb-run -a "./${{ needs.appimage.outputs.asset }}" --version', workflow
        )

    def test_screenshot_desktop_entry_disables_startup_notification(self) -> None:
        """
        验证截图入口不会触发 KDE 启动指针动效。

        @return 无返回值。
        """
        desktop_entry = (PROJECT_ROOT / "data/mark-shot.desktop.in").read_text(
            encoding="utf-8"
        )

        self.assertIn("StartupNotify=false", desktop_entry)

    def test_release_version_is_synchronized(self) -> None:
        """
        验证发布版本在项目配置、打包清单和发布文档中保持一致。

        @return 无返回值。
        """
        # 1. 从项目配置和变更日志首项读取发布版本与日期
        cmake_config = (PROJECT_ROOT / "CMakeLists.txt").read_text(encoding="utf-8")
        changelog = (PROJECT_ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
        version_match = re.search(
            r"project\(mark-shot VERSION (?P<version>[0-9]+\.[0-9]+\.[0-9]+) ",
            cmake_config,
        )
        release_match = re.match(
            r"# Changelog\n\n## (?P<version>[0-9]+\.[0-9]+\.[0-9]+) "
            r"- (?P<date>[0-9]{4}-[0-9]{2}-[0-9]{2})",
            changelog,
        )
        self.assertIsNotNone(version_match)
        self.assertIsNotNone(release_match)
        version = version_match.group("version")
        release_date = release_match.group("date")
        escaped_version = re.escape(version)
        self.assertEqual(version, release_match.group("version"))

        # 2. 验证项目配置与各打包清单使用同一版本
        version_patterns = {
            "flake.nix": rf'version = "{escaped_version}";',
            "packaging/aur/PKGBUILD": rf"(?m)^pkgver={escaped_version}$",
            "packaging/aur/.SRCINFO": rf"(?m)^\s*pkgver = {escaped_version}$",
            "packaging/aur_bin/PKGBUILD": rf"(?m)^pkgver={escaped_version}$",
            "packaging/local_aur/PKGBUILD": rf"(?m)^pkgver={escaped_version}$",
            "packaging/local_aur/.SRCINFO": (
                rf"(?m)^\s*pkgver = {escaped_version}$"
            ),
            "packaging/rpm/mark-shot.spec": (
                rf"(?m)^Version:\s+{escaped_version}$"
            ),
        }
        for relative_path, pattern in version_patterns.items():
            with self.subTest(relative_path=relative_path):
                content = (PROJECT_ROOT / relative_path).read_text(encoding="utf-8")
                self.assertRegex(content, pattern)

        # 3. 验证发布文档将本次版本放在首项
        releases_en = (PROJECT_ROOT / "docs/releases.md").read_text(encoding="utf-8")
        releases_zh = (PROJECT_ROOT / "docs/releases.zh-CN.md").read_text(
            encoding="utf-8"
        )
        metainfo = (
            PROJECT_ROOT
            / "packaging/flatpak/io.github.jswysnemc.MarkShot.metainfo.xml"
        ).read_text(encoding="utf-8")

        self.assertTrue(releases_en.startswith(f"# Release Notes\n\n### {version}"))
        self.assertTrue(releases_zh.startswith(f"# 发版说明\n\n### {version}"))
        self.assertIn(
            f'<release version="{version}" date="{release_date}" />', metainfo
        )

        # 4. 验证 AUR 源码地址指向本次标签或源码包
        aur_srcinfo = (PROJECT_ROOT / "packaging/aur/.SRCINFO").read_text(
            encoding="utf-8"
        )
        local_aur_srcinfo = (
            PROJECT_ROOT / "packaging/local_aur/.SRCINFO"
        ).read_text(encoding="utf-8")
        self.assertIn(f"#tag=v{version}", aur_srcinfo)
        self.assertIn(f"source = mark-shot-{version}.tar.gz", local_aur_srcinfo)


if __name__ == "__main__":
    unittest.main()
