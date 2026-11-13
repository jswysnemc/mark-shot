#!/usr/bin/env python3
"""【Windows测试】【原生主库】在新Wine和不可见Xvfb中运行主库测试，逐组保存实际结果"""

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

from windows_ocr_actions import snapshot
from windows_test_runtime import run


GROUPS = [
    "features::desktop::tray::",
    "backend::windows::native::",
    "backend::windows::overlay::scroll::tests::",
    "backend::windows::clipboard::",
    "config::tests::",
    "features::capture_history::tests::storage::",
    "ui::file_picker::save::tests::",
    "features::ocr_result::control_input::tests::",
    "features::ocr_result::input_method::tests::",
]


def execute(command, environment, root, log):
    """【Windows测试】【实际分组】按command的测试程序与过滤器逐组执行，返回总状态，root保存输出、数量和二进制摘要"""
    binary, *groups = command
    report = {"binary_sha256": hashlib.sha256(Path(binary).read_bytes()).hexdigest(), "groups": [],
              "real_windows_validation": False, "passed": False}
    try:
        for index, group in enumerate(groups):
            path = root / f"group-{index + 1}.log"
            with path.open("wb") as output:
                result = subprocess.run(["wine", binary, group, "--test-threads=1", "--include-ignored"], env=environment, cwd=root,
                                        stdout=output, stderr=output, timeout=60)
            text = path.read_text(errors="replace")
            count = re.search(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored", text)
            entry = {"filter": group, "exit_code": result.returncode, "log": path.name, "passed": False}
            if count:
                entry.update(tests_passed=int(count[2]), tests_failed=int(count[3]), tests_ignored=int(count[4]))
                entry["passed"] = result.returncode == 0 and count[1] == "ok" and int(count[2]) > 0
            report["groups"].append(entry)
            log.write(json.dumps(entry) + "\n")
            log.flush()
            if not entry["passed"]:
                snapshot(environment, root, f"group-{index + 1}-failure")
        report["passed"] = all(entry["passed"] for entry in report["groups"])
        return subprocess.CompletedProcess(command, 0 if report["passed"] else 1)
    finally:
        (root / "tests.json").write_text(json.dumps(report, indent=2) + "\n")


def main():
    """【Windows测试】【私有入口】读取测试binary、新root和可选filter，输出实际分组状态，无参数、无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--filter", action="append")
    options = parser.parse_args()
    command = [str(options.binary.resolve()), *(options.filter or GROUPS)]
    report = run(options.root.resolve(), command, execute=execute)
    print(json.dumps(report, indent=2))
    raise SystemExit(report["exit_code"])


if __name__ == "__main__":
    main()
