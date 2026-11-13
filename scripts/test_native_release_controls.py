"""【原生发布测试】【结果按钮】第六版必须包含同指纹的按钮释放和静止资源证据"""

from dataclasses import replace
import json
from pathlib import Path
import tempfile
import unittest

from native_release_manifest import CONTROL_FLAGS, CONTROL_GROUPS, ReleaseManifest, sha256_file


class ControlReleaseTests(unittest.TestCase):
    """【原生发布测试】【按钮门槛】以第五版完整证据作为固定基准，验证新增第六版约束"""

    def test_sixth_preview_requires_both_native_control_groups(self):
        """【原生发布测试】【第六版约束】缺少组、错误二进制、失败状态和资源异常全部拒绝，完整同指纹记录通过，无参数、无返回值"""
        release = Path(__file__).resolve().parents[1] / "release"
        manifest = ReleaseManifest.load(release / "native-preview-5.json")
        report = json.loads((release / "native-preview-5-verification.json").read_text())
        with tempfile.TemporaryDirectory() as directory:
            verification = Path(directory) / "verification.json"
            notes = Path(directory) / "notes.md"
            notes.write_text("Native control verification fixture.\n")
            verification.write_text(json.dumps(report))
            current = replace(manifest, tag="native-v0.2.0-preview.6", verification_sha256=sha256_file(verification), notes_sha256=sha256_file(notes))
            with self.assertRaisesRegex(ValueError, "OCR control verification"):
                current.verify_documents(verification, notes)
            result = {"sha256": manifest.binary_sha256, "bytes": manifest.binary_bytes,
                "idle_before": {"threads": 1, "cpu_ticks": 100}, "idle_after": {"threads": 1, "cpu_ticks": 100},
                **dict.fromkeys(CONTROL_FLAGS, True)}
            report["ocr_control_gui_groups"] = [{"name": name, "passed": True, "binary_sha256": manifest.binary_sha256, "evidence_sha256": "d" * 64, "result": result} for name in CONTROL_GROUPS]
            verification.write_text(json.dumps(report))
            current = replace(current, verification_sha256=sha256_file(verification))
            current.verify_documents(verification, notes)
            for key, value in (("sha256", "b" * 64), ("bytes", 1), ("idle_after", {"threads": 2, "cpu_ticks": 100}), ("idle_after", {"threads": 1, "cpu_ticks": 101}), *[(key, False) for key in CONTROL_FLAGS]):
                changed = json.loads(json.dumps(report))
                changed["ocr_control_gui_groups"][0]["result"][key] = value
                verification.write_text(json.dumps(changed))
                invalid = replace(current, verification_sha256=sha256_file(verification))
                with self.subTest(key=key), self.assertRaises(ValueError):
                    invalid.verify_documents(verification, notes)
            for key, value in (("binary_sha256", "b" * 64), ("passed", False), ("name", "unverified"), ("evidence_sha256", "")):
                changed = json.loads(json.dumps(report))
                changed["ocr_control_gui_groups"][0][key] = value
                verification.write_text(json.dumps(changed))
                invalid = replace(current, verification_sha256=sha256_file(verification))
                with self.subTest(key=key), self.assertRaises(ValueError):
                    invalid.verify_documents(verification, notes)


if __name__ == "__main__":
    unittest.main()
