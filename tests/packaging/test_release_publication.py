"""【原生发布】【交付测试】验证草稿发布、不可变重试、摘要核验与内部文件隔离"""

from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/packaging"))
from artifact_metadata import checksum, finalize
from publish_assets import assets, publish
from release_api import request, verify_asset
from release_context import Context


class ReleasePublication(unittest.TestCase):
    def setUp(self):
        """【原生发布】【样本准备】无传参，创建预发布包、内部摘要、元数据与更新描述，无返回值"""
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.asset = self.root / "mark-shot-native-preview.tar.gz"
        self.asset.write_bytes(b"verified native package")
        self.context = Context(
            "native-v0.2.0-preview.6", "0.2.0-preview.6", "a" * 40, True
        )
        finalize(
            self.asset, self.context.version, "x86_64", self.context.commit, "portable"
        )
        self.notes = self.root / "notes.md"
        self.notes.write_text("## Update\n\nNative prerelease.\n")
        self.metadata = {
            "id": 123,
            "tag_name": self.context.tag,
            "prerelease": True,
            "draft": True,
            "body": "",
            "html_url": "https://github.com/owner/project/releases/tag/preview",
            "assets": [],
        }

    def remote_asset(self, digest=None):
        """【原生发布】【远程样本】按可选 digest 返回当前安装包的服务器摘要记录"""
        return {
            "name": self.asset.name,
            "digest": digest or f"sha256:{checksum(self.asset)}",
        }

    def test_first_publication_uploads_only_package_then_opens_draft(self):
        """【原生发布】【首次发布】无传参，核对草稿、上传文件、全部远程摘要和最后公开顺序，无返回值"""
        self.metadata["assets"] = [self.remote_asset()]

        def response(endpoint, method="GET", payload=None, missing_ok=False):
            """【原生发布】【接口样本】按 endpoint、method、payload 和 missing_ok 模拟发布响应，返回远程记录"""
            if missing_ok:
                return None
            if method == "POST":
                self.assertTrue(payload["draft"])
                self.assertEqual(payload["make_latest"], "false")
                return {**self.metadata, "assets": []}
            if method == "PATCH":
                self.assertFalse(payload["draft"])
                self.assertIn("<details>", payload["body"])
                self.assertIn(checksum(self.asset), payload["body"])
                return {**self.metadata, **payload}
            return self.metadata

        with (
            patch("publish_assets.request", side_effect=response) as api,
            patch("publish_assets.subprocess.run") as command,
        ):
            url = publish(self.root, self.context, self.notes, "owner/project")
        self.assertEqual(url, self.metadata["html_url"])
        command.assert_called_once_with(
            ["gh", "release", "upload", self.context.tag, str(self.asset)], check=True
        )
        self.assertEqual(
            [
                call.args[1] if len(call.args) > 1 else "GET"
                for call in api.call_args_list
            ],
            ["GET", "POST", "GET", "PATCH"],
        )

    def test_retry_does_not_upload_existing_equal_package(self):
        """【原生发布】【重试测试】无传参，相同远程安装包不再上传，不覆盖已存在文件，无返回值"""
        self.metadata["assets"] = [self.remote_asset()]
        with (
            patch(
                "publish_assets.request",
                side_effect=[
                    self.metadata,
                    self.metadata,
                    {**self.metadata, "draft": False},
                ],
            ) as api,
            patch("publish_assets.subprocess.run") as command,
        ):
            api.side_effect = lambda endpoint, method="GET", payload=None, **kwargs: {
                **self.metadata,
                **(payload or {}),
            }
            publish(self.root, self.context, self.notes, "owner/project")
        command.assert_not_called()

    def test_mismatched_existing_asset_stops_before_publication(self):
        """【原生发布】【不可变门槛】无传参，已有文件摘要不同时拒绝上传和公开，无返回值"""
        self.metadata["assets"] = [self.remote_asset("sha256:" + "b" * 64)]
        with (
            patch("publish_assets.request", return_value=self.metadata) as api,
            patch("publish_assets.subprocess.run") as command,
        ):
            with self.assertRaisesRegex(ValueError, "checksum mismatch"):
                publish(self.root, self.context, self.notes, "owner/project")
        self.assertEqual(api.call_count, 1)
        command.assert_not_called()

    def test_failed_upload_leaves_release_in_draft(self):
        """【原生发布】【失败门槛】无传参，上传失败时不调用公开发布接口，无返回值"""
        with (
            patch("publish_assets.request", side_effect=[None, self.metadata]) as api,
            patch(
                "publish_assets.subprocess.run",
                side_effect=subprocess.CalledProcessError(1, "gh"),
            ),
        ):
            with self.assertRaises(subprocess.CalledProcessError):
                publish(self.root, self.context, self.notes, "owner/project")
        self.assertEqual(api.call_count, 2)

    def test_unrecorded_packages_and_checksum_tampering_are_rejected(self):
        """【原生发布】【完整性门槛】无传参，拒绝缺少元数据的安装包和被修改的摘要文件，无返回值"""
        extra = self.root / "unverified.zip"
        extra.write_bytes(b"unverified")
        with self.assertRaisesRegex(ValueError, "records"):
            assets(self.root, self.context)
        extra.unlink()
        Path(str(self.asset) + ".sha256").write_text("incorrect digest\n")
        with self.assertRaisesRegex(ValueError, "sidecar"):
            assets(self.root, self.context)

    def test_legacy_asset_without_server_digest_is_downloaded(self):
        """【原生发布】【旧资产兼容】无传参，没有服务器摘要时下载原始字节核验，无返回值"""

        def download(command, **kwargs):
            """【原生发布】【下载样本】按 command 和 kwargs 写入安装包字节，返回空值"""
            directory = Path(command[command.index("--dir") + 1])
            (directory / self.asset.name).write_bytes(self.asset.read_bytes())

        with patch("release_api.subprocess.run", side_effect=download) as command:
            verify_asset(self.context.tag, self.asset, {"digest": None})
        self.assertEqual(command.call_count, 1)

    def test_only_missing_release_is_tolerated(self):
        """【原生发布】【接口错误】无传参，仅允许 404 返回空值，认证及网络失败必须报错，无返回值"""
        for message in ("HTTP 401", "TLS connect error", "HTTP 403"):
            with patch(
                "release_api.subprocess.run",
                return_value=subprocess.CompletedProcess([], 1, "", message),
            ):
                with self.assertRaises(RuntimeError):
                    request(
                        "repos/owner/project/releases/tags/preview", missing_ok=True
                    )
        with patch(
            "release_api.subprocess.run",
            return_value=subprocess.CompletedProcess(
                [], 1, "", "gh: Not Found (HTTP 404)"
            ),
        ):
            self.assertIsNone(
                request("repos/owner/project/releases/tags/preview", missing_ok=True)
            )


if __name__ == "__main__":
    unittest.main()
