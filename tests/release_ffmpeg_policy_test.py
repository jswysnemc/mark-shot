#!/usr/bin/env python3

from pathlib import Path
import re
import unittest


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
FFMPEG_FLAG = "-DMARK_SHOT_REQUIRE_FFMPEG=ON"
DEBIAN_DEVELOPMENT_PACKAGES = (
    "libavcodec-dev",
    "libavformat-dev",
    "libavutil-dev",
    "libswresample-dev",
    "libswscale-dev",
)


def read_repository_file(relative_path: str) -> str:
    """读取仓库内指定文本文件并返回内容。"""
    return (REPOSITORY_ROOT / relative_path).read_text(encoding="utf-8")


def workflow_job(relative_path: str, job_name: str) -> str:
    """提取 GitHub Actions 工作流中指定任务的文本。"""
    workflow = read_repository_file(relative_path)
    match = re.search(
        rf"(?ms)^  {re.escape(job_name)}:\n(.*?)(?=^  [A-Za-z0-9_-]+:\n|\Z)",
        workflow,
    )
    if match is None:
        raise AssertionError(f"workflow job not found: {relative_path}:{job_name}")
    return match.group(0)


class ReleaseFfmpegPolicyTest(unittest.TestCase):
    """验证官方 Linux 构建不会静默禁用 FFmpeg 录制支持。"""

    def assert_debian_ffmpeg_development_packages(self, text: str) -> None:
        """断言 Debian 或 Ubuntu 构建安装全部 libav 开发包。"""
        for package in DEBIAN_DEVELOPMENT_PACKAGES:
            self.assertIn(package, text)

    def test_linux_ci_jobs_require_ffmpeg(self) -> None:
        """验证常规 Linux 构建任务安装并强制启用 FFmpeg。"""
        x86_job = workflow_job(".github/workflows/build.yml", "linux")
        arm_job = workflow_job(".github/workflows/build.yml", "linux-arm64")

        self.assertIn("ffmpeg", x86_job)
        self.assertIn(FFMPEG_FLAG, x86_job)
        self.assert_debian_ffmpeg_development_packages(arm_job)
        self.assertIn(FFMPEG_FLAG, arm_job)

    def test_release_binary_jobs_require_ffmpeg(self) -> None:
        """验证通用二进制发行任务安装并强制启用 FFmpeg。"""
        x86_job = workflow_job(".github/workflows/release-binaries.yml", "linux")
        arm_job = workflow_job(".github/workflows/release-binaries.yml", "linux-arm64")

        self.assertIn("ffmpeg", x86_job)
        self.assertIn(FFMPEG_FLAG, x86_job)
        self.assert_debian_ffmpeg_development_packages(arm_job)
        self.assertIn(FFMPEG_FLAG, arm_job)

    def test_linux_package_jobs_require_ffmpeg(self) -> None:
        """验证 AppImage、DEB、RPM 与 pacman 发行任务强制启用 FFmpeg。"""
        appimage_job = workflow_job(".github/workflows/release-appimage.yml", "appimage")
        deb_job = workflow_job(".github/workflows/release-deb.yml", "deb")
        rpm_job = workflow_job(".github/workflows/release-rpm.yml", "rpm")
        aur_job = workflow_job(".github/workflows/release-aur-bin.yml", "pkg")

        self.assertIn("ffmpeg", appimage_job)
        self.assertIn(FFMPEG_FLAG, appimage_job)
        self.assert_debian_ffmpeg_development_packages(deb_job)
        self.assertIn(FFMPEG_FLAG, deb_job)
        self.assertIn("ffmpeg-free-devel", rpm_job)
        self.assert_debian_ffmpeg_development_packages(aur_job)
        self.assertIn(FFMPEG_FLAG, aur_job)
        # 预编译包的 .PKGINFO 由 CI 手写，绕过了 makepkg 的 soname 依赖生成。
        # 这里必须按实际链接的 soname 生成依赖，否则 FFmpeg 大版本升级后
        # 旧包仍能通过依赖检查却无法加载库。
        self.assertIn("arch-soname-depends.sh", aur_job)
        self.assertNotIn("depend = ffmpeg", aur_job)

    def test_package_manifests_require_ffmpeg(self) -> None:
        """验证 RPM、Nix、Flatpak 与 AUR 清单声明 FFmpeg。"""
        rpm_spec = read_repository_file("packaging/rpm/mark-shot.spec")
        flake = read_repository_file("flake.nix")
        flatpak = read_repository_file(
            "packaging/flatpak/io.github.jswysnemc.MarkShot.yml"
        )
        aur_bin = read_repository_file("packaging/aur_bin/PKGBUILD")
        aur_source = read_repository_file("packaging/aur/PKGBUILD")

        for component in (
            "libavcodec",
            "libavformat",
            "libavutil",
            "libswresample",
            "libswscale",
        ):
            self.assertIn(f"pkgconfig({component})", rpm_spec)
        self.assertIn(FFMPEG_FLAG, rpm_spec)
        self.assertIn("pkgs.ffmpeg", flake)
        self.assertIn(FFMPEG_FLAG, flake)
        self.assertIn(FFMPEG_FLAG, flatpak)
        self.assertIn("org.freedesktop.Platform.ffmpeg-full:", flatpak)
        self.assertIn("version: '24.08'", flatpak)
        self.assertIn("directory: lib/ffmpeg", flatpak)
        self.assertIn("add-ld-path: .", flatpak)
        self.assertIn("no-autodownload: false", flatpak)

        # PKGBUILD 声明 .so 名而不是裸包名 'ffmpeg'：makepkg 会把它展开为
        # libavformat.so=62-64 形式，FFmpeg 大版本升级后 pacman 直接判定
        # 依赖不满足，而不是让用户装上一个跑不起来的包。
        for pkgbuild in (aur_bin, aur_source):
            for library in (
                "libavcodec.so",
                "libavformat.so",
                "libavutil.so",
                "libswresample.so",
                "libswscale.so",
            ):
                self.assertRegex(pkgbuild, rf"(?ms)^depends=.*'{re.escape(library)}'")
            self.assertNotRegex(pkgbuild, r"(?m)^depends=.*'ffmpeg'")


if __name__ == "__main__":
    unittest.main()
