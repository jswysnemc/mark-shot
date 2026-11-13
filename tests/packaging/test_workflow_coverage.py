"""【原生构建】【迁移审计】检查 Qt 工作流逐项迁移及正式发布的默认保护边界"""

from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest

import yaml


ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github/workflows"
MIGRATED = (
    "build",
    "release-binaries",
    "release-deb",
    "release-rpm",
    "release-aur-bin",
    "release-appimage",
    "release-flatpak",
    "debian-source",
    "aur-publish",
    "ppa-publish",
)


def workflow(name):
    """【原生构建】【清单读取】按 name 读取 YAML 标量文本，避免将 on 误解析为布尔键，返回工作流"""
    return yaml.load((WORKFLOWS / f"{name}.yml").read_text(), Loader=yaml.BaseLoader)


class WorkflowCoverage(unittest.TestCase):
    @unittest.skipUnless(shutil.which("flatpak"), "Flatpak command is not installed")
    def test_flatpak_finish_permissions_are_accepted(self):
        """【原生构建】【权限验证】无传参，使用真实 Flatpak 命令校验完整权限，不依赖运行时下载，无返回值"""
        data = yaml.load(
            (ROOT / "packaging/flatpak/io.github.jswysnemc.MarkShot.yml").read_text(),
            Loader=yaml.BaseLoader,
        )
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            (directory / "files").mkdir()
            (directory / "metadata").write_text(
                "[Application]\nname=io.github.jswysnemc.MarkShot\nruntime=org.freedesktop.Platform/x86_64/25.08\nsdk=org.freedesktop.Sdk/x86_64/25.08\n"
            )
            subprocess.run(
                [
                    "flatpak",
                    "build-finish",
                    "--no-exports",
                    *data["finish-args"],
                    str(directory),
                ],
                check=True,
                capture_output=True,
            )

    def test_container_array_scripts_use_bash(self):
        """【原生构建】【容器脚本】无传参，复现默认 sh 无法解析 Bash 数组的问题，无返回值"""
        for name in (*MIGRATED, "native-linux-package"):
            data = workflow(name)
            for job in data["jobs"].values():
                if not job.get("container"):
                    continue
                shell = job.get("defaults", {}).get("run", {}).get("shell")
                for step in job.get("steps", []):
                    if re.search(r"\b\w+=\(", step.get("run", "")):
                        self.assertEqual(step.get("shell", shell), "bash", name)

    def test_all_ten_qt_workflows_have_native_entries(self):
        """【原生构建】【覆盖测试】无传参，验证十个入口和两个公共任务均存在，无返回值"""
        for name in (*MIGRATED, "native-linux-package", "native-release-context"):
            self.assertTrue(workflow(name)["jobs"], name)

    def test_debian_matrix_covers_eight_targets(self):
        """【原生构建】【DEB矩阵】无传参，确认四个发行版均覆盖 amd64 与 arm64，无返回值"""
        matrix = workflow("release-deb")["jobs"]["packages"]["strategy"]["matrix"][
            "include"
        ]
        self.assertEqual(
            {(row["arch"], row["target"]) for row in matrix},
            {
                (arch, target)
                for arch in ("x86_64", "aarch64")
                for target in ("debian12", "debian13", "ubuntu2404", "ubuntu2604")
            },
        )

    def test_build_and_rpm_include_arm(self):
        """【原生构建】【架构矩阵】无传参，确认原生构建、便携包、RPM 和 pacman 包包含 ARM64，无返回值"""
        for name, job in (
            ("native-build", "linux"),
            ("release-binaries", "packages"),
            ("release-rpm", "packages"),
            ("release-aur-bin", "packages"),
        ):
            rows = workflow(name)["jobs"][job]["strategy"]["matrix"]["include"]
            self.assertEqual({item["arch"] for item in rows}, {"x86_64", "aarch64"})

    def test_manual_runs_do_not_publish_by_default(self):
        """【原生发布】【保护测试】无传参，验证手动发布开关默认关闭且标签不接受 Qt v 前缀，无返回值"""
        for name in MIGRATED:
            data = workflow(name)
            inputs = data["on"].get("workflow_dispatch") or {}
            for field in (inputs.get("inputs") or {}).values():
                if field.get("type") == "boolean":
                    self.assertEqual(field.get("default"), "false", name)
            push = data["on"].get("push") or {}
            if "tags" in push:
                self.assertEqual(push["tags"], ["native-v*"], name)

    def test_no_qt_build_toolchain_or_plugins(self):
        """【原生构建】【工具链测试】无传参，检查迁移入口不依赖 Qt 构建或旧插件路径，无返回值"""
        for name in MIGRATED:
            text = (WORKFLOWS / f"{name}.yml").read_text()
            for forbidden in (
                "qt6-base",
                "qt6-qtbase",
                "cmake --build",
                "qt/packaging",
                "linuxdeploy-plugin-qt",
            ):
                self.assertNotIn(forbidden, text, name)
        data = yaml.load(
            (ROOT / "packaging/flatpak/io.github.jswysnemc.MarkShot.yml").read_text(),
            Loader=yaml.BaseLoader,
        )
        self.assertEqual(data["runtime"], "org.freedesktop.Platform")
        self.assertIn("org.freedesktop.Sdk.Extension.llvm20", data["sdk-extensions"])
        commands = "\n".join(data["modules"][0]["build-commands"])
        self.assertIn("/app/share/icons/hicolor/128x128/apps/", commands)
        self.assertIn("/app/share/icons/hicolor/256x256/apps/", commands)
        self.assertIn("--share=network", data["finish-args"])
        self.assertIn("--socket=pulseaudio", data["finish-args"])


if __name__ == "__main__":
    unittest.main()
