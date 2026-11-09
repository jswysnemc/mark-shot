"""【CI下载测试】【失败边界】验证网关回退仍校验摘要，错误文件不能替代既有资产"""

import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("ci_download", Path(__file__).with_name("ci-download.py"))
DOWNLOAD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DOWNLOAD)


class DownloadTests(unittest.TestCase):
    """【CI下载测试】【固定内容】全部请求使用本地模拟内容，不访问外部服务"""

    def test_gateway_failure_uses_official_asset_api_with_original_digest(self):
        """【CI下载测试】【官方回退】无参数，验证发布网关失败后仍交付同一摘要文件，无返回值"""
        content = b"verified release archive"
        calls = []

        def invoke(arguments, check):
            """【CI下载测试】【请求夹具】按arguments与check模拟失败和成功响应，返回进程结果"""
            calls.append(arguments)
            if len(calls) == 1:
                return subprocess.CompletedProcess(arguments, 22)
            Path(arguments[arguments.index("--output") + 1]).write_bytes(content)
            return subprocess.CompletedProcess(arguments, 0)

        with tempfile.TemporaryDirectory() as directory, patch.object(DOWNLOAD.subprocess, "run", invoke), patch.object(
            DOWNLOAD.subprocess, "check_output", return_value=json.dumps({"assets": [{"name": "tool.zip", "url": "https://api.github.com/repos/org/tool/releases/assets/123"}]}).encode()
        ):
            destination = Path(directory) / "asset.zip"
            DOWNLOAD.download("https://github.com/org/tool/releases/download/v1/tool.zip", destination, hashlib.sha256(content).hexdigest())
            self.assertEqual(destination.read_bytes(), content)
            self.assertTrue(calls[1][-1].startswith("https://api.github.com/repos/org/tool/releases/assets/123?"))

    def test_invalid_bytes_never_replace_existing_asset(self):
        """【CI下载测试】【校验拒绝】无参数，错误内容保留既有资产并清除半份文件，无返回值"""
        def invoke(arguments, check):
            """【CI下载测试】【错误内容】按arguments和check写入错误字节，返回HTTP成功"""
            Path(arguments[arguments.index("--output") + 1]).write_bytes(b"invalid")
            return subprocess.CompletedProcess(arguments, 0)

        with tempfile.TemporaryDirectory() as directory, patch.object(DOWNLOAD.subprocess, "run", invoke):
            destination = Path(directory) / "asset.zip"
            destination.write_bytes(b"existing")
            with self.assertRaisesRegex(RuntimeError, "SHA-256"):
                DOWNLOAD.download("https://example.test/asset.zip", destination, "0" * 64)
            self.assertEqual(destination.read_bytes(), b"existing")
            self.assertFalse(destination.with_suffix(".zip.part").exists())


if __name__ == "__main__":
    unittest.main()
