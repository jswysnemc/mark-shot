#!/usr/bin/env python3
"""【Windows DPI回归】【私有配置】仅在新Wine前缀设置DPI并运行真实窗口尺寸消息测试"""

import argparse
import json
from pathlib import Path
import subprocess

from windows_test_runtime import run
from windows_dpi_session import configure


def execute(command, environment, root, log):
    """【Windows DPI回归】【实际消息】按command中的测试文件与dpi配置私有注册表，返回执行结果，要求窗口实际DPI匹配"""
    binary, dpi = command
    environment = configure(environment, root, log, dpi)
    return subprocess.run(["wine", binary, "backend::windows::native::window_limits::tests::", "--include-ignored", "--nocapture", "--test-threads=1"],
                          env=environment, cwd=root, stdout=log, stderr=log, timeout=60)


def main():
    """【Windows DPI回归】【隔离入口】读取测试文件、dpi和新root，返回运行状态，无参数，不修改系统注册表"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--dpi", type=int, choices=[96, 120, 144, 192], required=True)
    parser.add_argument("--root", type=Path, required=True)
    options = parser.parse_args()
    result = run(options.root.resolve(), [str(options.binary.resolve()), str(options.dpi)], execute=execute)
    print(json.dumps(result, indent=2))
    raise SystemExit(result["exit_code"])


if __name__ == "__main__":
    main()
