"""【GNOME虚拟机】【桌面集成】验证真实全局按键、焦点、托盘状态与会话注销"""

import json
import time

from annotations import config
from desktop import status
from gnome_capture import prime


def registration(session):
    """【GNOME虚拟机】【注册检查】按 session 读取独立状态或确认本次授权，返回最终状态，未完成返回空值"""
    current = status(session)
    if current.get("shortcuts", {}).get("state") in ("registered", "failed"):
        return current
    session.shortcut_permission()
    return None


def capture(session):
    """【GNOME虚拟机】【快捷键触发】按 session 用真实 F8 创建本次截图，检查焦点和取消，返回空值"""
    before = {w["id"] for w in session.windows()}
    session.key("f8")
    shown = session.wait(lambda: next((w for w in session.windows("mark-shot") if w["id"] not in before), None), "真实全局快捷键没有打开截图")
    session.wait(lambda: any(w["id"] == shown["id"] and w["focused"] for w in session.windows()), "快捷键截图没有获得焦点")
    session.screenshot("shortcut-triggered")
    session.key("esc")
    session.wait(lambda: not any(w["id"] == shown["id"] for w in session.windows()), "快捷键截图取消后没有关闭")


def run(session, binary, overrides):
    """【GNOME虚拟机】【临时身份】按 session、binary、overrides 提供系统可发现的桌面身份，返回测试结果并移除本次条目"""
    preferences = json.loads(session.guest("""
import ast
schema='org.gnome.settings-daemon.global-shortcuts'
apps=ast.literal_eval(subprocess.check_output(['gsettings','get',schema,'applications'],text=True).removeprefix('@as '))
value=subprocess.check_output(['dconf','read','/org/gnome/settings-daemon/global-shortcuts/io.github.jswysnemc.MarkShot/shortcuts'],text=True).strip()
print(json.dumps({'apps':apps,'value':value}))
"""))
    identity = json.loads(session.guest("""
path=Path.home()/'.local/share/applications/io.github.jswysnemc.MarkShot.desktop'
if path.exists():print('null')
else:
 path.parent.mkdir(parents=True,exist_ok=True)
 executable=values.replace('\\\\','\\\\\\\\').replace('"','\\\\"')
 content='[Desktop Entry]\\nType=Application\\nName=Mark Shot\\nExec="'+executable+'" --settings\\nIcon=mark-shot\\nNoDisplay=true\\nTerminal=false\\n'
 with path.open('x') as stream:stream.write(content)
 print(json.dumps({'path':str(path),'content':content}))
""", binary))
    try:
        return verify(session, binary, overrides)
    finally:
        session.guest("""
import ast
schema='org.gnome.settings-daemon.global-shortcuts';app='io.github.jswysnemc.MarkShot'
key='/org/gnome/settings-daemon/global-shortcuts/'+app+'/shortcuts'
if values['value']:subprocess.run(['dconf','write',key,values['value']],check=True)
else:subprocess.run(['dconf','reset',key],check=True)
if app not in values['apps']:
 current=ast.literal_eval(subprocess.check_output(['gsettings','get',schema,'applications'],text=True).removeprefix('@as '))
 subprocess.run(['gsettings','set',schema,'applications',repr([entry for entry in current if entry!=app])],check=True)
""", preferences)
        if identity:
            session.guest("p=Path(values['path']);p.unlink() if p.exists() and p.read_text()==values['content'] else None", identity)


def verify(session, binary, overrides):
    """【GNOME虚拟机】【桌面流程】按 session、binary、overrides 检查快捷键及托盘切换，返回结果，最终关闭本次服务"""
    prime(session, binary, overrides)
    original = config(session)
    configured = original.replace("shortcuts_enabled=false", 'shortcuts_enabled=true\nscreenshot_shortcut="F8"')
    configured = configured.replace("tray_enabled=false", "tray_enabled=true")
    session.setup(configured)
    pid = session.launch(binary, ["--background"], overrides=overrides)
    try:
        registered = session.wait(lambda: registration(session), "GNOME 快捷键授权未完成", timeout=20)
        (session.root / "registration.json").write_text(json.dumps(registered, ensure_ascii=False, indent=2) + "\n")
        session.screenshot("shortcut-registration")
        assert registered["shortcuts"]["state"] == "registered", registered
        assert registered["shortcuts"]["detail"].strip(), "系统返回空快捷键绑定"
        assert registered["tray"]["state"] == "unavailable", registered
        capture(session)
        # 1. 【GNOME虚拟机】【配置交接】原子替换独立配置以触发监视器，托盘切换不得改变快捷键会话
        session.guest("p=Path(values[0]);n=p.with_suffix('.next');n.write_text(values[1]);n.replace(p)",
                      [session.remote + "/config.toml", configured.replace("tray_enabled=true", "tray_enabled=false")])
        session.wait(lambda: status(session).get("tray", {}).get("state") == "disabled", "关闭托盘未生效")
        assert status(session)["shortcuts"] == registered["shortcuts"], "关闭托盘改变了全局按键绑定"
        capture(session)
        session.guest("p=Path(values[0]);n=p.with_suffix('.next');n.write_text(values[1]);n.replace(p)",
                      [session.remote + "/config.toml", original])
        session.wait(lambda: not session.alive(pid), "禁用后台后服务未退出")
        before = {w["id"] for w in session.windows()}
        session.key("f8")
        time.sleep(0.5)
        assert not any(w["id"] not in before and w["app"] == "mark-shot" for w in session.windows()), "后台退出后全局快捷键仍触发截图"
        return {"binding": registered["shortcuts"]["detail"], "global_key_triggered": True, "capture_focused": True,
                "capture_cancelled": True, "tray_without_extension": "unavailable",
                "tray_disable_preserves_shortcut": True, "portal_session_released": True}
    finally:
        session.log("capture")
        session.cleanup()
