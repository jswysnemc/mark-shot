"""【原生发布测试】【清单回归】拒绝错误来源、摘要和混用构建的界面证据"""

from dataclasses import replace
import json
import tempfile
import unittest

from native_release_manifest import ReleaseManifest, sha256_file
from native_release_test_support import fixture


class ManifestTests(unittest.TestCase):
    """【原生发布测试】【清单场景】覆盖严格结构和同指纹验证限制"""

    def test_valid_documents(self):
        """【原生发布测试】【有效清单】匹配的隔离文件应通过并返回验证报告，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            path, verification, notes, expected = fixture(directory)
            actual = ReleaseManifest.load(path)
            self.assertEqual(actual, expected)
            self.assertFalse(actual.verify_documents(verification, notes)["original_goal_complete"])

    def test_invalid_manifest(self):
        """【原生发布测试】【非法字段】错误来源、标签、类型和未知字段全部拒绝，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            path, *_ = fixture(directory)
            original = json.loads(path.read_text())
            for key, value in (("schema", True), ("schema", 2), ("repository", "another/repo"), ("branch", "main"), ("tag", "v0.2.0"), ("tag", "native-v0.2.0-preview.1\n"), ("source_commit", "b" * 39), ("binary_sha256", "A" * 64), ("run_id", True), ("artifact_id", 0), ("binary_bytes", 129 * 1024 * 1024), ("unexpected", 1)):
                with self.subTest(key=key, value=value):
                    path.write_text(json.dumps(dict(original, **{key: value})))
                    with self.assertRaises(ValueError):
                        ReleaseManifest.load(path)
            original.pop("notes_sha256")
            path.write_text(json.dumps(original))
            with self.assertRaises(ValueError):
                ReleaseManifest.load(path)

    def test_document_digests(self):
        """【原生发布测试】【文档摘要】说明或报告变化后旧清单必须失效，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            _, verification, notes, manifest = fixture(directory)
            notes.write_text("changed")
            with self.assertRaisesRegex(ValueError, "notes digest"):
                manifest.verify_documents(verification, notes)
            notes.write_text("Native preview; full rewrite remains incomplete.\n")
            verification.write_text("{}")
            with self.assertRaisesRegex(ValueError, "verification document digest"):
                manifest.verify_documents(verification, notes)

    def test_report_boundaries(self):
        """【原生发布测试】【报告边界】不同构建、缺少场景和虚假完整声明全部拒绝，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            _, verification, notes, manifest = fixture(directory)
            original = json.loads(verification.read_text())
            changes = [("source_commit", "b" * 40), ("run_id", 13), ("binary", {}), ("original_goal_complete", True), ("user_configuration_modified", True), ("original_qt_sources_modified", True), ("gui_groups", original["gui_groups"][:-1])]
            for key, value in changes:
                with self.subTest(key=key):
                    verification.write_text(json.dumps(dict(original, **{key: value})))
                    current = replace(manifest, verification_sha256=sha256_file(verification))
                    with self.assertRaises(ValueError):
                        current.verify_documents(verification, notes)
            for key, value in (("passed", False), ("binary_sha256", "b" * 64), ("scenario_count", 4), ("name", "unverified")):
                with self.subTest(group_key=key):
                    changed = json.loads(json.dumps(original))
                    changed["gui_groups"][0][key] = value
                    verification.write_text(json.dumps(changed))
                    current = replace(manifest, verification_sha256=sha256_file(verification))
                    with self.assertRaises(ValueError):
                        current.verify_documents(verification, notes)


if __name__ == "__main__":
    unittest.main()
