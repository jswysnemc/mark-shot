"""【KDE虚拟机】【桌面集成】检查门户真实绑定、后台托盘与实际键盘触发"""

import json

from annotations import config


def shortcut_keys(session, replacement=None):
    """【KDE虚拟机】【临时绑定】按 session 读取测试身份的系统按键，提供 replacement 时设置并返回原值，用于取消与恢复测试"""
    return json.loads(session.guest("""
identity=['io.github.jswysnemc.MarkShot','screenshot','Mark Shot','Mark Shot — Screenshot']
command=['busctl','--user','--json=short','call','org.kde.kglobalaccel','/kglobalaccel','org.kde.KGlobalAccel']
before=json.loads(subprocess.check_output(command+['shortcut','as','4',*identity],text=True))['data'][0]
if values is not None:
 subprocess.run(command+['setForeignShortcut','asai','4',*identity,str(len(values)),*map(str,values)],check=True,capture_output=True)
print(json.dumps(before))
""", replacement))


def configure_window(session, before):
    """【KDE虚拟机】【系统配置】按 session、before 返回门户本次打开的配置窗口，已有系统设置窗口不能作为操作目标"""
    return next((window for window in session.windows("systemsettings") if window["id"] not in before), None)


def close_configuration(session, window):
    """【KDE虚拟机】【取消配置】按 session、window 关闭本次门户打开的系统窗口，无返回值，不结束其他设置窗口"""
    session.kwin("const w=workspace.windowList().find(w=>w.internalId.toString()===" + json.dumps(window["id"]) + ");workspace.activeWindow=w;return true;")
    session.key("alt+f4")
    session.wait(lambda: not any(w["id"] == window["id"] for w in session.windows()), "快捷键配置窗口没有关闭")


def status(session):
    """【KDE虚拟机】【注册结果】按 session 读取独立缓存中的后台状态，返回字典；尚未写入时返回空值"""
    return json.loads(session.guest("""
paths=list((Path(values)/'cache/mark-shot/desktop').glob('*/status.json'))
print(paths[0].read_text() if paths else '{}')
""", session.remote))


def run(session, binary, overrides):
    """【KDE虚拟机】【快捷键流程】按 session、binary、overrides 检查空绑定及真实触发，返回结果；最后清理独立后台"""
    case = overrides.pop("MARK_SHOT_TEST_SHORTCUT_CASE", "binding")
    desktop_config = config(session).replace("tray_enabled=false", "tray_enabled=true")
    desktop_config = desktop_config.replace("shortcuts_enabled=false", 'shortcuts_enabled=true\nscreenshot_shortcut="Ctrl+Shift+F10"')
    session.setup(desktop_config)
    before = {window["id"] for window in session.windows()}
    saved_keys = shortcut_keys(session, []) if case in ("empty", "recover") else None
    try:
        # 1. 【KDE虚拟机】【状态初始化】清除本次独立目录中的旧结果，重复执行时不能提前读取上次注册状态
        session.guest("for path in (Path(values)/'cache/mark-shot/desktop').glob('*/status.json'):path.unlink(missing_ok=True)", session.remote)
        session.launch(binary, ["--background"], overrides=overrides)
        registered = session.wait(lambda: (value if value.get("shortcuts", {}).get("state") in ("registered", "failed") else None)
                                  if (value := status(session)) else None, "门户没有返回注册结果")
        (session.root / "registration.json").write_text(json.dumps(registered, ensure_ascii=False, indent=2) + "\n")
        session.screenshot("shortcut-registration")
        assert registered["tray"]["state"] == "registered", "KDE 没有注册系统托盘"
        shortcut = registered["shortcuts"]
        assert not (shortcut["state"] == "registered" and not shortcut["detail"].strip()), "空绑定被误报为注册成功"
        if case in ("empty", "recover"):
            assert shortcut["state"] == "failed", "空绑定没有显示明确失败"
            window = session.wait(lambda: configure_window(session, before), "门户没有打开系统快捷键配置")
            session.screenshot("shortcut-configuration")
            if case == "empty":
                close_configuration(session, window)
                session.key("ctrl+shift+f10")
                assert status(session)["shortcuts"]["state"] == "failed", "取消配置后又误报注册成功"
                assert not any(w["id"] not in before and w["app"] == binary.rsplit("/", 1)[-1]
                               for w in session.windows()), "空绑定触发了截图"
                return {"empty_binding_not_registered": True, "cancel_kept_unbound": True, "tray_registered": True}
            # 2. 【KDE虚拟机】【系统录入】通过门户打开的 KDE 窗口录入按键，不直接发送应用激活事件
            session.click(window["x"] + window["width"] - 55, window["y"] + 204)
            session.key("ctrl+shift+f10")
            session.click(window["x"] + window["width"] - 55, window["y"] + window["height"] - 26)
            shortcut = session.wait(lambda: (value if value["state"] == "registered" else None)
                                    if (value := status(session).get("shortcuts")) else None, "系统配置没有恢复绑定状态")
            session.screenshot("shortcut-recovered")
            close_configuration(session, window)
        assert shortcut["state"] == "registered", shortcut
        before = {window["id"] for window in session.windows()}
        session.key("ctrl+shift+f10")
        session.wait(lambda: any(w["id"] not in before and w["app"] == binary.rsplit("/", 1)[-1]
                                for w in session.windows()), "实际快捷键没有打开截图")
        session.screenshot("shortcut-triggered")
        session.key("esc")
        return {"shortcut_binding": shortcut["detail"], "shortcut_triggered_capture": True,
                "configured_without_restart": case == "recover", "tray_registered": True}
    finally:
        session.log("capture")
        session.cleanup()
        if window := configure_window(session, before):
            close_configuration(session, window)
        if saved_keys is not None:
            shortcut_keys(session, saved_keys)
