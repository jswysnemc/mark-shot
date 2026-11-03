"""【原生发布测试】【清单回归】拒绝错误来源、摘要和混用构建的界面证据"""

from dataclasses import replace
import json
import tempfile
import unittest

from native_release_manifest import HISTORY_GROUPS, ReleaseManifest, WINDOW_GROUPS, sha256_file
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

    def test_second_preview_requires_same_binary_window_evidence(self):
        """【原生发布测试】【窗口证据】第二版必须包含两种合成器的同指纹原生窗口验证，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            _, verification, notes, manifest = fixture(directory)
            current = replace(manifest, tag="native-v0.2.0-preview.2")
            with self.assertRaisesRegex(ValueError, "window input method verification"):
                current.verify_documents(verification, notes)
            report = json.loads(verification.read_text())
            report["window_gui_groups"] = [{"name": name, "passed": True, "scenario_count": 2, "binary_sha256": manifest.binary_sha256, "native_protocol": True, "no_ocr": True, "no_idle_surface_requests": True} for name in sorted(WINDOW_GROUPS)]
            verification.write_text(json.dumps(report))
            current = replace(current, verification_sha256=sha256_file(verification))
            current.verify_documents(verification, notes)
            for key, value in (("binary_sha256", "b" * 64), ("passed", False), ("scenario_count", 1), ("native_protocol", False), ("no_ocr", False), ("no_idle_surface_requests", False)):
                changed = json.loads(json.dumps(report))
                changed["window_gui_groups"][0][key] = value
                verification.write_text(json.dumps(changed))
                invalid = replace(current, verification_sha256=sha256_file(verification))
                with self.subTest(key=key), self.assertRaises(ValueError):
                    invalid.verify_documents(verification, notes)

    def test_third_preview_requires_same_binary_history_evidence(self):
        """【原生发布测试】【历史证据】第三版不得省略历史场景或混用旧构建的鼠标证据，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            _, verification, notes, manifest = fixture(directory)
            report = json.loads(verification.read_text())
            report["window_gui_groups"] = [{"name": name, "passed": True, "scenario_count": 2, "binary_sha256": manifest.binary_sha256, "native_protocol": True, "no_ocr": True, "no_idle_surface_requests": True} for name in sorted(WINDOW_GROUPS)]
            verification.write_text(json.dumps(report))
            current = replace(manifest, tag="native-v0.2.0-preview.3", verification_sha256=sha256_file(verification))
            with self.assertRaisesRegex(ValueError, "history verification"):
                current.verify_documents(verification, notes)
            report["history_gui_groups"] = [{"name": name, "passed": True, "binary_sha256": manifest.binary_sha256, "evidence_sha256": "c" * 64} for name in sorted(HISTORY_GROUPS)]
            verification.write_text(json.dumps(report))
            current = replace(current, verification_sha256=sha256_file(verification))
            current.verify_documents(verification, notes)
            for key, value in (("binary_sha256", "b" * 64), ("passed", False), ("name", "unverified"), ("evidence_sha256", "")):
                changed = json.loads(json.dumps(report))
                changed["history_gui_groups"][0][key] = value
                verification.write_text(json.dumps(changed))
                invalid = replace(current, verification_sha256=sha256_file(verification))
                with self.subTest(key=key), self.assertRaises(ValueError):
                    invalid.verify_documents(verification, notes)


if __name__ == "__main__":
    unittest.main()
