"""【RPM兼容性测试】【安装症状】覆盖用户报告的 OpenSSL 4 与 glibc 2.44 依赖冲突"""

from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts/packaging"))
from rpm_compatibility import validate


class RpmCompatibility(unittest.TestCase):
    def test_reported_development_dependencies_are_rejected(self):
        """【RPM兼容性测试】【原始错误】无参数，逐项拒绝用户安装日志中的不可满足依赖，无返回值"""
        for requirement in [
            "libcrypto.so.4()(64bit)",
            "libcrypto.so.4(OPENSSL_4.0.0)(64bit)",
            "libssl.so.4()(64bit)",
            "libm.so.6(GLIBC_2.44)(64bit)",
        ]:
            with self.subTest(requirement=requirement), self.assertRaises(ValueError):
                validate(requirement)

    def test_supported_stable_dependencies_are_accepted(self):
        """【RPM兼容性测试】【稳定依赖】无参数，接受基线与较早版本，保留正常系统库要求，无返回值"""
        validate(
            "libssl.so.3(OPENSSL_3.5.0)(64bit)\nlibcrypto.so.3()(64bit)\nlibm.so.6(GLIBC_2.42)(64bit)\nlibc.so.6(GLIBC_2.2.5)(64bit)"
        )


if __name__ == "__main__":
    unittest.main()
