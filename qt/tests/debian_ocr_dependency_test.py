#!/usr/bin/env python3
"""【打包测试】【OCR依赖】验证动态运行库包名及版本关系，不依赖本机 Debian 数据库。"""

import importlib.util
from pathlib import Path
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "scripts/debian-ocr-runtime-dependency.py"
SPEC = importlib.util.spec_from_file_location("debian_ocr_dependency", SCRIPT)
DEPENDENCY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DEPENDENCY)


class DebianOcrDependencyTest(unittest.TestCase):
    """覆盖不同发行版 ABI 包名及拒绝不明确元数据。"""

    def test_preserves_abi_name_and_minimum_version(self):
        """保留包名与版本下限，不将安全更新锁死在构建时的等号版本，无返回值。"""
        for name in ("libonnxruntime1", "libonnxruntime1.21", "libonnxruntime1t64:amd64"):
            with self.subTest(name=name):
                result = DEPENDENCY.runtime_dependency(f"libc6-dev, {name} (= 1.29.0-3), libprotobuf-dev")
                self.assertEqual(result, f"{name} (>= 1.29.0-3)")

    def test_accepts_runtime_without_version(self):
        """上游无版本关系时不自造约束，无返回值。"""
        self.assertEqual(DEPENDENCY.runtime_dependency("libonnxruntime1"), "libonnxruntime1")

    def test_rejects_missing_or_ambiguous_runtime(self):
        """缺少依赖、多个 ABI 或替代依赖不明确时阻止打包，无返回值。"""
        for relation in ("", "libonnxruntime-dev", "libprotobuf1", "libonnxruntime1 | libonnxruntime2",
                         "libonnxruntime1, libonnxruntime2", "libonnxruntime1 (<< 1.20)"):
            with self.subTest(relation=relation):
                with self.assertRaises(ValueError):
                    DEPENDENCY.runtime_dependency(relation)


if __name__ == "__main__":
    unittest.main()
