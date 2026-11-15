"""【GNOME虚拟机】【设置事务】验证真实默认值、颜色编辑和配置持久化，不改变用户设置"""

import time
import tomllib

from annotations import config


def contents(session):
    """【GNOME虚拟机】【配置读取】按 session 读取独立 TOML 原文，返回字符串，不读取用户配置"""
    return session.guest("print(Path(values).read_text())", session.remote + "/config.toml")


def search(session, name):
    """【GNOME虚拟机】【字段检索】按 session、name 使用原生设置搜索框，返回空值"""
    session.key("ctrl+f")
    session.key("ctrl+a")
    session.text(name)
    time.sleep(0.2)


def picker(session, window):
    """【GNOME虚拟机】【取色组件】按 session、window 点击首个搜索字段，返回真实取色窗口"""
    session.click(window["x"] + 700, window["y"] + 190)
    return session.wait(lambda: session.windows("mark-shot-color"), "颜色设置未打开共享取色组件")[0]


def run(session, binary, overrides):
    """【GNOME虚拟机】【设置流程】按 session、binary、overrides 浏览默认值并检查保存、取消和恢复，返回结果"""
    original = config(session).split("[save]")[0]
    session.setup(original)
    try:
        session.launch(binary, ["--settings"], overrides=overrides)
        window = session.wait(lambda: session.windows("mark-shot-settings"), "设置未出现")[0]
        baseline = contents(session)
        for index, name in [(1, "capture"), (2, "recording"), (7, "storage"), (8, "pin"), (10, "advanced")]:
            session.click(window["x"] + 70, window["y"] + 56 + index * 36 + 16)
            session.screenshot("defaults-" + name)
        search(session, "save.directory")
        session.screenshot("default-directory")
        session.click(window["x"] + 700, window["y"] + 190)
        session.key("ret")
        assert contents(session) == baseline, "进入默认路径编辑并确认后写入了继承值"
        search(session, "startup.default_color")
        color = picker(session, window)
        session.screenshot("default-color")
        session.click(color["x"] + 100, color["y"] + 207)
        session.key("ctrl+a")
        session.text("#00bb7799")
        session.key("ret")
        session.wait(lambda: tomllib.loads(contents(session)).get("startup", {}).get("default_color") == "#00bb7799", "颜色和透明度未保存")
        session.screenshot("color-saved")
        color = picker(session, window)
        session.click(color["x"] + 100, color["y"] + 207)
        session.key("ctrl+a")
        session.text("#112233ff")
        session.key("esc")
        assert tomllib.loads(contents(session))["startup"]["default_color"] == "#00bb7799", "取消颜色编辑仍修改配置"
        color = picker(session, window)
        session.click(color["x"] + 30, color["y"] + 300)
        session.wait(lambda: "default_color" not in tomllib.loads(contents(session)).get("startup", {}), "恢复默认没有删除颜色覆盖")
        search(session, "save.directory")
        session.click(window["x"] + 700, window["y"] + 190)
        session.key("ctrl+a")
        destination = session.remote + "/output"
        session.text(destination)
        session.key("ret")
        session.wait(lambda: tomllib.loads(contents(session)).get("save", {}).get("directory") == destination, "文字路径没有持久化")
        session.click(window["x"] + window["width"] - 26, window["y"] + 24)
        session.wait(lambda: not session.windows("mark-shot-settings"), "设置关闭失败")
        return {"defaults_visually_verified": True, "unchanged_default_edit_preserved_config": True,
                "rgba_saved": True, "color_cancel_preserved_config": True, "color_reset_removed_override": True,
                "text_path_persisted": True}
    finally:
        session.log("capture")
        session.cleanup()
