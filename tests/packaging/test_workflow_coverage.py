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
    def test_packaging_does_not_depend_on_local_documentation(self):
        """【原生构建】【文档隔离】无传参，确认 Linux 和 Windows 打包不复制本地文档，无返回值"""
        linux = next(
            step
            for step in workflow("native-build")["jobs"]["linux"]["steps"]
            if step.get("name") == "Package native build"
        )["run"]
        windows = (ROOT / "scripts/windows-package.ps1").read_text()
        for text in (linux, windows):
            self.assertNotIn("README.md", text)
            self.assertNotIn("docs/", text)
            self.assertNotIn(".docs", text)

    def test_publication_checks_out_exact_tag_with_full_history(self):
        """【原生发布】【固定检出】无传参，确认统一发布入口检出已验证提交和完整标签历史，无返回值"""
        checkout = next(
            step
            for step in workflow("native-release")["jobs"]["publish"]["steps"]
            if step.get("uses") == "actions/checkout@v4"
        )
        self.assertEqual(checkout["with"]["fetch-depth"], "0")
        self.assertEqual(checkout["with"]["persist-credentials"], "false")
        self.assertEqual(
            checkout["with"]["ref"], "${{ needs.context.outputs.source_sha }}"
        )

    def test_release_waits_for_all_inherited_builds_and_verification(self):
        """【原生发布】【发布门槛】无传参，确保既有平台构建与验证全部成功后才能公开安装包，无返回值"""
        data = workflow("native-release")
        expected = {
            "context",
            "verify",
            "binaries",
            "deb",
            "rpm",
            "arch",
            "appimage",
            "flatpak",
        }
        self.assertEqual(set(data["jobs"]["publish"]["needs"]), expected)
        self.assertEqual(data["on"]["push"]["tags"], ["native-v*"])
        for job in expected - {"context", "verify"}:
            self.assertNotIn("upload_to_release", data["jobs"][job]["with"])
        for name in (
            "release-binaries",
            "release-deb",
            "release-rpm",
            "release-aur-bin",
            "release-appimage",
            "release-flatpak",
        ):
            child = workflow(name)
            self.assertIn("workflow_call", child["on"])
            self.assertNotIn("push", child["on"])
            self.assertNotIn("release", child["on"])
            self.assertNotIn(
                "upload_to_release", child["on"]["workflow_call"]["inputs"]
            )
            self.assertNotIn(
                "upload_to_release", child["on"]["workflow_dispatch"]["inputs"]
            )
            self.assertNotIn("publish", child["jobs"])

    def test_binary_aur_upload_requires_explicit_manual_request(self):
        """【原生发布】【稳定渠道保护】无传参，统一发布调用不得在缺少 AUR 密钥时自动上传稳定渠道，无返回值"""
        step = next(
            step
            for step in workflow("release-aur-bin")["jobs"]["aur"]["steps"]
            if step.get("name") == "Publish validated AUR metadata"
        )
        self.assertIn("inputs.push_to_aur", step["if"])

    def test_deleted_branches_are_not_workflow_triggers(self):
        """【原生构建】【分支同步】无传参，禁止发布和构建入口依赖已删除分支，无返回值"""
        for name in (
            "native-build",
            "windows-build",
            "debian-source",
            "native-release",
        ):
            self.assertNotIn(
                "rewrite/native-wayland", (WORKFLOWS / f"{name}.yml").read_text()
            )

    def test_publication_retry_validates_build_before_cross_run_download(self):
        """【原生发布】【复用顺序】无传参，跨任务下载之前必须验证固定源码和全部平台，并与主入口共享并发锁，无返回值"""
        data = workflow("native-release-publish")
        steps = data["jobs"]["publish"]["steps"]
        validation = next(
            step for step in steps if "release_build.py" in step.get("run", "")
        )
        download = next(
            step for step in steps if step.get("uses") == "actions/download-artifact@v4"
        )
        self.assertLess(steps.index(validation), steps.index(download))
        self.assertEqual(
            download["with"]["run-id"], "${{ steps.context.outputs.run_id }}"
        )
        self.assertEqual(
            data["concurrency"]["group"], "native-release-${{ inputs.tag }}"
        )
        self.assertNotIn("push", data["on"])

    def test_source_verification_failures_preserve_build_evidence(self):
        """【原生构建】【失败证据】无传参，验证前收集构建产物，正式验证产物仍在验证后上传，失败使用独立证据名称，无返回值"""
        steps = workflow("debian-source")["jobs"]["resolute"]["steps"]
        collection = next(
            step
            for step in steps
            if step.get("name") == "Collect source and binary artifacts"
        )
        verification = next(
            step
            for step in steps
            if step.get("name") == "Lint and install reconstructed package"
        )
        uploads = [
            step for step in steps if step.get("uses") == "actions/upload-artifact@v4"
        ]
        self.assertLess(steps.index(collection), steps.index(verification))
        normal = next(
            step
            for step in uploads
            if step["with"]["name"] == "native-debian-source-resolute"
        )
        evidence = next(
            step
            for step in uploads
            if step["with"]["name"] == "evidence-debian-source-resolute"
        )
        self.assertLess(steps.index(verification), steps.index(normal))
        self.assertEqual(evidence["if"], "failure()")
        self.assertEqual(evidence["with"]["path"], "target/source-artifacts/")

    def test_debian_source_lint_uses_the_debian_vendor_profile(self):
        """【原生构建】【发行标识校验】无传参，Ubuntu 容器中的 Debian 源码包必须使用 Debian 校验配置，不忽略合法 unstable 标识，无返回值"""
        step = next(
            step
            for step in workflow("debian-source")["jobs"]["resolute"]["steps"]
            if step.get("name") == "Lint and install reconstructed package"
        )
        self.assertIn("lintian --profile debian", step["run"])
        self.assertIn("--fail-on error", step["run"])
        self.assertNotIn("--suppress-tags", step["run"])

    def test_ppa_upload_uses_a_verified_workspace_relative_directory(self):
        """【原生发布】【容器产物路径】无传参，防止容器外临时路径导致只上传 dsc 而丢失源码归档，无返回值"""
        steps = workflow("ppa-publish")["jobs"]["source"]["steps"]
        upload = next(
            step for step in steps if step.get("uses") == "actions/upload-artifact@v4"
        )
        self.assertEqual(upload["with"]["path"], "target/ppa-source-artifacts/")
        collection = next(
            step for step in steps if "collect_debian_source.py" in step.get("run", "")
        )
        self.assertLess(steps.index(collection), steps.index(upload))

    def test_rpm_cache_does_not_restore_packaging_workspaces(self):
        """【原生构建】【缓存范围】无传参，确认 RPM 只缓存 Cargo 工具和下载，不恢复打包工作目录，无返回值"""
        steps = workflow("native-linux-package")["jobs"]["package"]["steps"]
        cache = next(
            step for step in steps if step.get("uses") == "Swatinem/rust-cache@v2"
        )
        self.assertEqual(cache["with"]["cache-targets"], "${{ inputs.kind != 'rpm' }}")
        flatpak = next(
            step
            for step in workflow("release-flatpak")["jobs"]["packages"]["steps"]
            if step.get("uses") == "Swatinem/rust-cache@v2"
        )
        self.assertEqual(flatpak["with"]["cache-targets"], "false")
        self.assertEqual(flatpak["with"]["key"], "flatpak-vendor-v2")

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
