#!/usr/bin/env python3
"""【打包】【OCR依赖】从构建所用开发包解析动态 ONNX 运行依赖，不猜测发行版 ABI 包名。"""

import re
import subprocess
import sys


def runtime_dependency(dependencies: str) -> str:
    """从开发包 dependencies 中返回 ONNX 运行包及最低版本；缺失或歧义时抛出 ValueError。"""
    matches = []
    for relation in dependencies.split(","):
        match = re.fullmatch(
            r"\s*(libonnxruntime[0-9][a-z0-9.+-]*(?::[a-z0-9-]+)?)"
            r"(?:\s*\((>=|=)\s*([^()\s]+)\))?\s*", relation
        )
        if match:
            name, _, version = match.groups()
            matches.append(f"{name} (>= {version})" if version else name)
    if len(matches) != 1:
        raise ValueError("libonnxruntime-dev must declare exactly one unambiguous runtime dependency")
    return matches[0]


def main() -> int:
    """查询本机开发包并输出 Debian 依赖关系，失败返回 1，无参数。"""
    try:
        result = subprocess.run(
            ["dpkg-query", "--show", "--showformat=${Depends}", "libonnxruntime-dev"],
            check=True, text=True, capture_output=True,
        )
        print(runtime_dependency(result.stdout))
        return 0
    except (OSError, subprocess.CalledProcessError, ValueError) as error:
        print(f"Cannot determine ONNX Runtime package dependency: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
