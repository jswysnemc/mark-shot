"""【原生发布测试】【原文菜单】第五版必须包含同指纹的两个原生菜单回归，不能复用旧预览证据"""

from dataclasses import replace
import json
import tempfile
import unittest

from native_release_manifest import HISTORY_DIALOG_GROUP, HISTORY_GROUPS, SOURCE_MENU_FLAGS, SOURCE_MENU_GROUPS, WINDOW_GROUPS, sha256_file
from native_release_test_support import fixture


class SourceMenuReleaseTests(unittest.TestCase):
    """【原生发布测试】【原文证据约束】拒绝缺失菜单组、错误二进制、失败动作和不匹配几何"""

    def test_fifth_preview_requires_native_source_menu_evidence(self):
        """【原生发布测试】【第五版约束】缺少或篡改任一菜单证据时发布失败，完整同指纹记录通过，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            _, verification, notes, manifest = fixture(directory)
            sha = manifest.binary_sha256
            report = json.loads(verification.read_text())
            report["window_gui_groups"] = [{"name": name, "passed": True, "scenario_count": 2, "binary_sha256": sha, "native_protocol": True, "no_ocr": True, "no_idle_surface_requests": True} for name in WINDOW_GROUPS]
            for window in report["window_gui_groups"]:
                window["source"] = {"source_menu_preedit_canceled": True, "source_menu_stale_commit_ignored": True}
            report["history_gui_groups"] = [{"name": name, "passed": True, "binary_sha256": sha, "evidence_sha256": "c" * 64} for name in HISTORY_GROUPS]
            flags = ("same_process_child", "native_parent_relationship", "default_enter_canceled", "wm_close_canceled", "parent_copy_delete_isolated", "parent_close_reclaimed_child", "space_release_confirmed_once", "unrelated_files_and_links_preserved")
            report["history_gui_groups"].append({"name": HISTORY_DIALOG_GROUP, "passed": True, "binary_sha256": sha, "evidence_sha256": "c" * 64, "result": {"sha256": sha, **dict.fromkeys(flags, True)}})
            verification.write_text(json.dumps(report))
            current = replace(manifest, tag="native-v0.2.0-preview.5", verification_sha256=sha256_file(verification))
            with self.assertRaisesRegex(ValueError, "source menu verification"):
                current.verify_documents(verification, notes)
            result = {"sha256": sha, "bytes": manifest.binary_bytes, "qt_menu_size": [225, 229], **dict.fromkeys(SOURCE_MENU_FLAGS, True)}
            report["source_menu_gui_groups"] = [{"name": name, "passed": True, "binary_sha256": sha, "evidence_sha256": "d" * 64, "result": result} for name in SOURCE_MENU_GROUPS]
            verification.write_text(json.dumps(report))
            current = replace(current, verification_sha256=sha256_file(verification))
            current.verify_documents(verification, notes)
            for key, value in (("sha256", "b" * 64), ("bytes", 1), ("qt_menu_size", [226, 229]), *[(key, False) for key in SOURCE_MENU_FLAGS]):
                changed = json.loads(json.dumps(report))
                changed["source_menu_gui_groups"][0]["result"][key] = value
                verification.write_text(json.dumps(changed))
                invalid = replace(current, verification_sha256=sha256_file(verification))
                with self.subTest(key=key), self.assertRaises(ValueError):
                    invalid.verify_documents(verification, notes)
            for key, value in (("binary_sha256", "b" * 64), ("passed", False), ("name", "unverified"), ("evidence_sha256", "")):
                changed = json.loads(json.dumps(report))
                changed["source_menu_gui_groups"][0][key] = value
                verification.write_text(json.dumps(changed))
                invalid = replace(current, verification_sha256=sha256_file(verification))
                with self.subTest(key=key), self.assertRaises(ValueError):
                    invalid.verify_documents(verification, notes)


if __name__ == "__main__":
    unittest.main()
