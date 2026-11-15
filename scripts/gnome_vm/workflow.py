"""【GNOME虚拟机】【回归入口】运行指定原生流程并保存结果，结束后恢复 Shell 观察权限"""

import argparse
import importlib
import json
from pathlib import Path

from gnome_session import GnomeSession


def main():
    """【GNOME虚拟机】【测试入口】无参数，解析 case、root、host 和 binary 执行流程，返回值为空，失败仍清理本次进程"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--host", required=True)
    parser.add_argument("--case", choices=("capture", "windows", "recording", "scrolling", "desktop", "clipboard", "settings", "ocr"), default="capture")
    parser.add_argument("--binary", default="/tmp/mark-shot-gnome-current")
    parser.add_argument("--override", action="append", default=[])
    options = parser.parse_args()
    session = GnomeSession(options.root, options.host)
    session.setup("")
    changed = False
    result = {"case": options.case}
    try:
        changed = session.observe()
        module = importlib.import_module("gnome_" + options.case)
        result.update(module.run(session, options.binary, dict(v.split("=", 1) for v in options.override)))
    except Exception as error:
        result["error"] = str(error)
        session.screenshot("failure")
        raise
    finally:
        session.cleanup()
        if changed:
            session.evaluate("global.context.unsafe_mode = false")
        (session.root / "result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
