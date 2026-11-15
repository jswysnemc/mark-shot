"""【X11虚拟机】【桌面集成】核对根窗口按键抓取、焦点和托盘切换"""

import json
import time

from annotations import config
from desktop import status


def trigger(session):
    """【X11虚拟机】【真实按键】按 session 用 F8 触发截图并取消，核对焦点，返回窗口身份"""
    before = {w["id"] for w in session.windows()}
    session.key("f8")
    shown = session.wait(lambda: next((w for w in session.windows() if w["id"] not in before and w["app"].startswith("mark-shot")), None), "真实 F8 没有打开截图")
    session.wait(lambda: any(w["id"] == shown["id"] and w["focused"] for w in session.windows()), "快捷键截图没有获得焦点")
    session.screenshot("shortcut-triggered")
    session.key("esc")
    session.wait(lambda: not any(w["id"] == shown["id"] for w in session.windows()), "取消后截图未关闭")
    return shown["id"]


def run(session, binary, overrides):
    """【X11虚拟机】【桌面流程】按 session、binary、overrides 验证快捷键、锁定修饰键、托盘和注销，返回结果"""
    original = config(session)
    configured = original.replace("tray_enabled=false", "tray_enabled=true").replace("shortcuts_enabled=false", 'shortcuts_enabled=true\nscreenshot_shortcut="F8"')
    session.setup(configured)
    pid = session.launch(binary, ["--background"], overrides=overrides)
    try:
        registered = session.wait(lambda: (value if value.get("shortcuts", {}).get("state") in ("registered", "failed") else None)
                                  if (value := status(session)) else None, "X11 快捷键没有完成注册")
        assert registered["shortcuts"] == {"state": "registered", "detail": "F8"}, registered
        assert registered["tray"]["state"] == "registered", registered
        (session.root / "registration.json").write_text(json.dumps(registered, ensure_ascii=False, indent=2) + "\n")
        trigger(session)
        session.key("caps_lock")
        try:
            trigger(session)
        finally:
            session.key("caps_lock")
        session.guest("p=Path(values[0]);n=p.with_suffix('.next');n.write_text(values[1]);n.replace(p)",
                      [session.remote + "/config.toml", configured.replace("tray_enabled=true", "tray_enabled=false")])
        session.wait(lambda: status(session).get("tray", {}).get("state") == "disabled", "关闭托盘未生效")
        assert status(session)["shortcuts"] == registered["shortcuts"], "托盘切换改变全局快捷键"
        trigger(session)
        session.guest("p=Path(values[0]);n=p.with_suffix('.next');n.write_text(values[1]);n.replace(p)", [session.remote + "/config.toml", original])
        session.wait(lambda: not session.alive(pid), "关闭后台配置后进程没有退出")
        before = {w["id"] for w in session.windows()}
        session.key("f8")
        time.sleep(0.4)
        assert not any(w["id"] not in before and w["app"].startswith("mark-shot") for w in session.windows()), "退出后按键仍然触发截图"
        return {"native_x11_binding": "F8", "real_key_triggered": True, "focused_cancelled": True,
                "caps_lock_ignored": True, "tray_registered": True, "tray_disable_preserves_binding": True, "binding_released_on_exit": True}
    finally:
        session.log("capture")
        session.cleanup()
