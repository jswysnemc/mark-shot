"""【原生发布测试】【交付回归】核验来源、草稿重试、公开资产和工作流权限边界"""

import copy
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import Mock, patch

from native_release_manifest import ARCHIVE_NAME, ARTIFACT_NAME, BRANCH, REPOSITORY
from native_release_publish import GitHub, publish, verify_source, verify_tag
from native_release_test_support import fixture


ENVIRONMENT = {"GITHUB_ACTIONS": "true", "GITHUB_REPOSITORY": REPOSITORY, "GITHUB_REF": f"refs/heads/{BRANCH}"}


class FakeGitHub:
    """【原生发布测试】【远程替身】保留草稿状态，模拟任务和发行包下载，不访问网络"""

    def __init__(self, root, manifest, notes, verification):
        """【原生发布测试】【替身构造】保存 root、manifest、notes 和 verification，初始化远程状态，无返回值"""
        self.root, self.manifest, self.verification = root, manifest, verification
        self.release = None
        self.tag_exists = False
        self.notes = notes.read_text()
        self.calls = []

    def api(self, endpoint, method="GET", payload=None, missing_ok=False):
        """【原生发布测试】【接口模拟】根据 endpoint、method 和 payload 返回状态，missing_ok 容忍缺失对象"""
        self.calls.append((method, endpoint, payload))
        if "/actions/runs/" in endpoint:
            return {"status": "completed", "conclusion": "success", "head_sha": self.manifest.source_commit, "head_branch": BRANCH, "repository": {"full_name": REPOSITORY}, "path": ".github/workflows/native-build.yml", "event": "push"}
        if "/compare/" in endpoint:
            return {"status": "ahead", "merge_base_commit": {"sha": self.manifest.source_commit}}
        if "/actions/artifacts/" in endpoint:
            return {"name": ARTIFACT_NAME, "expired": False, "workflow_run": {"id": self.manifest.run_id, "head_sha": self.manifest.source_commit}}
        if "/git/ref/tags/" in endpoint:
            return {"object": {"type": "commit", "sha": self.manifest.source_commit}} if self.tag_exists else None
        if method == "DELETE":
            return None
        if method == "POST":
            self.release = {"id": 56, "draft": True, "prerelease": True, "body": self.notes, "html_url": "https://example.invalid/native-preview", "assets": []}
        if method == "PATCH":
            self.release.update(payload)
            self.tag_exists = True
        return copy.deepcopy(self.release)

    def command(self, arguments):
        """【原生发布测试】【命令模拟】根据 arguments 下载样本或上传资产，无外部命令，返回空字节串"""
        self.calls.append(("command", arguments, None))
        if arguments[:2] == ["release", "upload"]:
            self.release["assets"] = [{"name": Path(name).name, "id": index} for index, name in enumerate(arguments[5:], 1)]
        else:
            destination = Path(arguments[arguments.index("--dir") + 1])
            for name in (ARCHIVE_NAME, ARCHIVE_NAME + ".sha256"):
                shutil.copyfile(self.root / name, destination / name)
            if arguments[0] == "release":
                shutil.copyfile(self.verification, destination / "native-preview-verification.json")
        return b""


class PublishTests(unittest.TestCase):
    """【原生发布测试】【发布场景】覆盖首次发布、失败重试和不可变发行包"""

    def test_publish_then_verify_without_overwrite(self):
        """【原生发布测试】【首次交付】草稿下载校验完成后公开，再次执行不覆盖资产，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory, patch.dict(os.environ, ENVIRONMENT, clear=True):
            root = Path(directory)
            _, verification, notes, manifest = fixture(root)
            github = FakeGitHub(root, manifest, notes, verification)
            result = publish(github, manifest, notes, verification, root / "first", "b" * 40)
            self.assertFalse(result["already_published"])
            mutation = [(method, payload) for method, _, payload in github.calls if method in ("POST", "PATCH")]
            self.assertTrue(mutation[0][1]["draft"])
            self.assertFalse(mutation[-1][1]["draft"])
            self.assertTrue(all(payload["make_latest"] == "false" and payload["prerelease"] for _, payload in mutation))
            github.calls.clear()
            result = publish(github, manifest, notes, verification, root / "second", "b" * 40)
            self.assertTrue(result["already_published"])
            self.assertFalse(any(method in ("POST", "PATCH", "DELETE") for method, _, _ in github.calls))

    def test_draft_retry(self):
        """【原生发布测试】【草稿恢复】此前失败留下的草稿重新上传并校验，不创建第二个发行版，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory, patch.dict(os.environ, ENVIRONMENT, clear=True):
            root = Path(directory)
            _, verification, notes, manifest = fixture(root)
            github = FakeGitHub(root, manifest, notes, verification)
            github.release = {"id": 56, "draft": True, "prerelease": True, "body": notes.read_text(), "assets": [{"id": 7, "name": "partial.tar.gz"}], "html_url": "https://example.invalid/native-preview"}
            publish(github, manifest, notes, verification, root / "retry", "b" * 40)
            self.assertFalse(any(method == "POST" for method, _, _ in github.calls))
            self.assertTrue(any(method == "DELETE" for method, _, _ in github.calls))
            self.assertFalse(github.release["draft"])

    def test_versioned_verification_keeps_public_asset_name(self):
        """【原生发布测试】【版本归档】仓库使用版本化证据文件时公开资产仍采用固定名称，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory, patch.dict(os.environ, ENVIRONMENT, clear=True):
            root = Path(directory)
            _, verification, notes, manifest = fixture(root)
            versioned = verification.with_name("native-preview-2-verification.json")
            verification.rename(versioned)
            github = FakeGitHub(root, manifest, notes, versioned)
            publish(github, manifest, notes, versioned, root / "versioned", "b" * 40)
            self.assertEqual({asset["name"] for asset in github.release["assets"]}, {ARCHIVE_NAME, ARCHIVE_NAME + ".sha256", "native-preview-verification.json"})

    def test_corrupt_download_does_not_create_release(self):
        """【原生发布测试】【损坏产物】下载文件摘要错误时禁止创建草稿，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory, patch.dict(os.environ, ENVIRONMENT, clear=True):
            root = Path(directory)
            _, verification, notes, manifest = fixture(root)
            github = FakeGitHub(root, manifest, notes, verification)
            (root / ARCHIVE_NAME).write_bytes(b"corrupt")
            with self.assertRaisesRegex(ValueError, "archive digest"):
                publish(github, manifest, notes, verification, root / "bad", "b" * 40)
            self.assertIsNone(github.release)

    def test_reject_stable_release(self):
        """【原生发布测试】【稳定版保护】已经存在的非预览发布不得修改，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory, patch.dict(os.environ, ENVIRONMENT, clear=True):
            root = Path(directory)
            _, verification, notes, manifest = fixture(root)
            github = FakeGitHub(root, manifest, notes, verification)
            github.release = {"prerelease": False, "body": notes.read_text()}
            with self.assertRaisesRegex(ValueError, "metadata mismatch"):
                publish(github, manifest, notes, verification, root / "stable", "b" * 40)

    def test_reject_non_workflow_context(self):
        """【原生发布测试】【权限保护】本地、其他仓库或稳定分支环境不能发布，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            _, verification, notes, manifest = fixture(root)
            for environment in ({}, dict(ENVIRONMENT, GITHUB_REPOSITORY="another/repo"), dict(ENVIRONMENT, GITHUB_REF="refs/heads/main")):
                with self.subTest(environment=environment), patch.dict(os.environ, environment, clear=True):
                    with self.assertRaises(ValueError):
                        publish(Mock(), manifest, notes, verification, root / "forbidden", "b" * 40)


class ProvenanceTests(unittest.TestCase):
    """【原生发布测试】【来源场景】验证任务、祖先关系、产物编号和标签对象"""

    def test_reject_wrong_source_metadata(self):
        """【原生发布测试】【来源拒绝】失败任务、错误提交、过期产物和无关提交全部拒绝，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            _, verification, notes, manifest = fixture(root)
            fake = FakeGitHub(root, manifest, notes, verification)
            for section, changes in ((0, {"conclusion": "failure"}), (0, {"head_sha": "b" * 40}), (0, {"path": "another.yml"}), (0, {"event": "pull_request"}), (1, {"status": "diverged"}), (2, {"expired": True}), (2, {"workflow_run": {"id": 99, "head_sha": manifest.source_commit}})):
                with self.subTest(section=section, changes=changes):
                    values = [fake.api(f"repos/{REPOSITORY}/actions/runs/12"), fake.api(f"repos/{REPOSITORY}/compare/a...b"), fake.api(f"repos/{REPOSITORY}/actions/artifacts/34")]
                    values[section].update(changes)
                    with self.assertRaises(ValueError):
                        verify_source(Mock(api=Mock(side_effect=values)), manifest, "b" * 40)

    def test_tag_commit_and_annotated_tag(self):
        """【原生发布测试】【标签对象】支持匹配提交和注释标签，拒绝不同来源，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            *_, manifest = fixture(directory)
            target = {"type": "commit", "sha": manifest.source_commit}
            self.assertFalse(verify_tag(Mock(api=Mock(return_value=None)), manifest))
            self.assertTrue(verify_tag(Mock(api=Mock(return_value={"object": target})), manifest))
            self.assertTrue(verify_tag(Mock(api=Mock(side_effect=[{"object": {"type": "tag", "sha": "c" * 40}}, {"object": target}])), manifest))
            with self.assertRaises(ValueError):
                verify_tag(Mock(api=Mock(return_value={"object": {"type": "commit", "sha": "b" * 40}})), manifest)

    def test_api_empty_delete_and_404(self):
        """【原生发布测试】【接口边界】删除空响应和明确 404 可以处理，其他错误不得吞掉，无参数、无返回值"""
        with patch("native_release_publish.subprocess.run", return_value=subprocess.CompletedProcess([], 0, b"", b"")):
            self.assertIsNone(GitHub().api("endpoint", method="DELETE"))
        for code in (404, 403, 500):
            response = subprocess.CompletedProcess([], 1, b"", f"gh: failure (HTTP {code})".encode())
            with self.subTest(code=code), patch("native_release_publish.subprocess.run", return_value=response):
                if code == 404:
                    self.assertIsNone(GitHub().api("endpoint", missing_ok=True))
                else:
                    with self.assertRaises(RuntimeError):
                        GitHub().api("endpoint", missing_ok=True)


if __name__ == "__main__":
    unittest.main()
