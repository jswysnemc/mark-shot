"""【全局快捷键回归】【GNOME门户】在已有私有GNOME会话验证授权、真实触发和独立托盘配置"""

import json
from pathlib import Path
import time


def verify(options, session, environment, evaluate, tap, wait, portal_action):
    """【全局快捷键回归】【实际授权】按私有会话参数验证门户结果与窗口焦点，返回检查记录"""
    root = options.root
    config = root / "config.toml"
    original = config.read_text()
    config.write_text(original + '\n[desktop]\nshortcuts_enabled=true\nscreenshot_shortcut="F8"\ntray_enabled=true\n')
    service = session.spawn([str(options.binary), "--config", str(config), "--background"], "desktop", environment)
    states = Path(environment["XDG_CACHE_HOME"]) / "mark-shot/desktop"

    def status():
        """【全局快捷键回归】【状态读取】无参数，读取私有服务状态，无文件时返回空字典"""
        files = list(states.glob("*/status.json"))
        return json.loads(files[0].read_text()) if files else {}

    wait(lambda: status().get("shortcuts", {}).get("state") in ("registered", "failed") or portal_action("Add") or portal_action("Allow"), "global shortcut permission did not appear", timeout=15)
    current = wait(lambda: status().get("shortcuts", {}).get("state") in ("registered", "failed") and status(), "global shortcut permission did not complete")
    (root / "desktop-status.json").write_text(json.dumps(current, indent=2) + "\n")
    assert current["shortcuts"]["state"] == "registered", current
    assert current["tray"]["state"] == "unavailable", "GNOME without extensions must not report a visible tray"
    tap(0xFFC5)
    window = wait(lambda: evaluate('global.get_window_actors().map(actor => ({title:actor.meta_window.get_title(),pid:actor.meta_window.get_pid()})).find(window => window.title === "mark-shot")'), "global shortcut did not open the editor")
    wait(lambda: evaluate(f'global.display.focus_window?.get_pid() === {window["pid"]}'), "global shortcut editor did not receive focus")
    tap(0xFF1B)
    wait(lambda: evaluate(f'!global.get_window_actors().some(actor => actor.meta_window.get_pid() === {window["pid"]})'), "global screenshot did not close")
    pending = config.with_suffix(".pending")
    pending.write_text(config.read_text().replace("tray_enabled=true", "tray_enabled=false"))
    pending.replace(config)
    wait(lambda: status().get("tray", {}).get("state") == "disabled", "tray disable did not apply")
    assert status()["shortcuts"] == current["shortcuts"], "tray changed the portal binding"
    tap(0xFFC5)
    wait(lambda: evaluate('global.display.focus_window?.get_title() === "mark-shot"'), "shortcut was lost after changing tray settings")
    tap(0xFF1B)
    wait(lambda: evaluate('!global.get_window_actors().some(actor => actor.meta_window.get_title() === "mark-shot")'), "second screenshot did not close")
    pending.write_text(original)
    pending.replace(config)
    service.wait(timeout=8)
    assert service.returncode == 0
    time.sleep(0.2)
    tap(0xFFC5)
    time.sleep(0.3)
    assert not evaluate('global.get_window_actors().some(actor => actor.meta_window.get_title() === "mark-shot")'), "portal shortcut survived service exit"
    return ["GlobalShortcuts portal permission", "Wayland global screenshot activation", "Wayland editor focus", "GNOME missing tray host status", "tray change preserves portal binding", "portal session release"]
