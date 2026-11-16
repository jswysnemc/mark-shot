"""【RPM兼容性】【依赖基线】拒绝只能安装在开发版系统上的共享库版本要求"""

import re
import subprocess


def validate(requirements):
    """【RPM兼容性】【版本检查】按 requirements 检查 Fedora 43 的 OpenSSL 3 和 glibc 2.42 基线，无返回值，越界时抛出错误"""
    for requirement in requirements.splitlines():
        if re.search(r"lib(?:ssl|crypto)\.so\.(?!3(?:\D|$))\d+", requirement):
            raise ValueError(f"RPM requires a newer OpenSSL ABI: {requirement}")
        for major, minor in re.findall(r"GLIBC_(\d+)\.(\d+)", requirement):
            if (int(major), int(minor)) > (2, 42):
                raise ValueError(
                    f"RPM exceeds the Fedora 43 glibc baseline: {requirement}"
                )


def verify(package):
    """【RPM兼容性】【产物验证】读取 package 的真实 RPM 依赖并检查基线，返回原始依赖文本供构建证据保存"""
    requirements = subprocess.check_output(
        ["rpm", "-qp", "--requires", str(package)], text=True
    )
    validate(requirements)
    return requirements
