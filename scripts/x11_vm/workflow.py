"""【X11虚拟机】【验证入口】按职责复用通用交互流程，记录结果并清理本次进程"""

import argparse
import importlib
import json
from pathlib import Path
import sys

from x11_session import X11Session

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "gnome_vm"))


def main():
    """【X11虚拟机】【流程执行】无参数，解析 case、root、host、binary 运行真实虚拟机验证，返回值为空"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--host")
    parser.add_argument("--binary", default="/tmp/mark-shot-x11-current")
    parser.add_argument("--case", choices=("capture", "windows", "wheel", "recording", "scrolling", "desktop", "clipboard", "settings", "ocr", "detection"), required=True)
    parser.add_argument("--override", action="append", default=[])
    options = parser.parse_args()
    session = X11Session(options.root, options.host)
    session.setup("")
    result = {"case": options.case}
    try:
        module = {"settings": "gnome_settings"}.get(options.case, "x11_" + options.case)
        result.update(importlib.import_module(module).run(session, options.binary, dict(value.split("=", 1) for value in options.override)))
    except Exception as error:
        result["error"] = str(error)
        session.screenshot("failure")
        raise
    finally:
        session.cleanup()
        (session.root / "result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(result, ensure_ascii=False))


if __name__ == "__main__":
    main()
