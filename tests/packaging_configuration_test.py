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
        workflow = (PROJECT_ROOT / ".github/workflows/release-deb.yml").read_text(
            encoding="utf-8"
        )
        expected_targets = {
            "debian12": ("debian:12", 'asset_suffix: ""'),
            "debian13": ("debian:13", 'asset_suffix: ".debian13"'),
            "ubuntu2404": ("ubuntu:24.04", 'asset_suffix: ".ubuntu24.04"'),
            "ubuntu2604": ("ubuntu:26.04", 'asset_suffix: ".ubuntu26.04"'),
        }

        for target, (image, suffix) in expected_targets.items():
            with self.subTest(target=target):
                target_block = re.search(
                    rf"- target: {target}\n(?P<body>(?:\s+.*\n){{1,8}})", workflow
                )
                self.assertIsNotNone(target_block)
                body = target_block.group("body")
                self.assertIn(f"image: {image}", body)
                self.assertIn(suffix, body)

        self.assertIn("Verify package installs on target", workflow)

    def test_appimage_uses_compatible_glibc_baseline(self) -> None:
        """
        验证 AppImage 使用稳定基线并限制 glibc 符号版本。

        @return 无返回值。
        """
        workflow = (PROJECT_ROOT / ".github/workflows/release-appimage.yml").read_text(
            encoding="utf-8"
        )

        self.assertRegex(workflow, r"container:\s*\n\s+image: debian:12")
        self.assertIn("ninja-build", workflow)
        self.assertIn('MAX_GLIBC_VERSION: "2.36"', workflow)
        self.assertIn("Assert glibc compatibility", workflow)
        self.assertIn("checkout_ref:", workflow)
        self.assertIn("ref: ${{ steps.version.outputs.checkout_ref }}", workflow)
        self.assertIn("appimage-ubuntu2404:", workflow)
        self.assertIn('image: ubuntu:24.04', workflow)
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


if __name__ == "__main__":
    unittest.main()
