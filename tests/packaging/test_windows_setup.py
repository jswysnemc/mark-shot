"""【Windows安装】【发布契约】验证 Setup 与便携包同时交付、版本一致和摘要保护"""

from pathlib import Path
import sys
import tempfile
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/packaging"))

from artifact_metadata import finalize
from publish_assets import assets
from release_context import Context


def workflow(name):
    """【Windows安装】【工作流读取】按 name 返回 YAML 字符串模型，保留 on 键语义"""
    return yaml.load(
        (ROOT / f".github/workflows/{name}.yml").read_text(), Loader=yaml.BaseLoader
    )


class WindowsSetup(unittest.TestCase):
    def test_release_accepts_verified_setup_and_portable_assets(self):
        """【Windows安装】【双包交付】无传参，验证两个包同时发布且 Setup 篡改会阻止交付，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            context = Context("v0.2.0", "0.2.0", "a" * 40, True)
            packages = []
            for suffix, kind in [(".zip", "windows"), ("-setup.exe", "windows-setup")]:
                package = (
                    directory / f"mark-shot-native-0.2.0-windows-x86_64-msvc{suffix}"
                )
                package.write_bytes(b"verified package")
                finalize(package, context.version, "x86_64", context.commit, kind)
                packages.append(package)
            self.assertEqual(set(assets(directory, context)), set(packages))
            packages[-1].write_bytes(b"tampered setup")
            with self.assertRaisesRegex(ValueError, "checksum mismatch"):
                assets(directory, context)

    def test_unrecorded_setup_is_rejected(self):
        """【Windows安装】【记录完整性】无传参，额外 Setup 缺少验证记录时不能进入发布，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            package = directory / "verified.zip"
            package.write_bytes(b"verified")
            context = Context("v0.2.0", "0.2.0", "a" * 40, True)
            finalize(package, context.version, "x86_64", context.commit, "windows")
            (directory / "unverified-setup.exe").write_bytes(b"unverified")
            with self.assertRaisesRegex(ValueError, "records"):
                assets(directory, context)

    def test_windows_build_uploads_setup_after_installation_verification(self):
        """【Windows安装】【构建门槛】无传参，编译和安装验证成功后才上传 Setup 及其摘要，无返回值"""
        steps = workflow("windows-build")["jobs"]["windows"]["steps"]
        names = [step.get("name") for step in steps]
        verify = names.index("Build and verify Windows setup installer")
        upload = names.index("Upload Windows build (not a release)")
        self.assertLess(names.index("Package and verify standalone build"), verify)
        self.assertLess(verify, upload)
        self.assertIn("-setup.exe\n", steps[upload]["with"]["path"])
        self.assertIn("-setup.exe.sha256", steps[upload]["with"]["path"])
        self.assertIn(
            "windows-setup-smoke.ps1", (ROOT / "scripts/windows-setup.ps1").read_text()
        )

    def test_release_passes_version_and_records_both_windows_assets(self):
        """【Windows安装】【版本一致】无传参，安装器采用标签版本并保留 Setup 独立验证记录，无返回值"""
        jobs = workflow("release-binaries")["jobs"]
        self.assertEqual(
            jobs["windows"]["with"]["package_version"],
            "${{ needs.context.outputs.version }}",
        )
        collect = next(
            step["run"]
            for step in jobs["collect-windows"]["steps"]
            if step.get("name") == "Version the verified Windows asset"
        )
        self.assertIn("sha256sum --check", collect)
        self.assertIn("--format windows\n", collect)
        self.assertIn("--format windows-setup", collect)


if __name__ == "__main__":
    unittest.main()
