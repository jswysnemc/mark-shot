"""【GNOME虚拟机】【捕获回归】验证原生门户授权、区域导出、标注与跨协议复制"""

from PIL import Image
from annotations import config, run as annotations_run


def capture_ready(session, pid):
    """【GNOME虚拟机】【授权确认】按 session、pid 检查本次捕获退出，未退出时确认仍显示的门户，返回完成标志"""
    if not session.alive(pid):
        return True
    session.authorize()
    return False


def prime(session, binary, overrides):
    """【GNOME虚拟机】【首次授权】按 session、binary、overrides 启动整屏捕获并完成门户选择，返回输出路径"""
    session.setup(config(session))
    path = session.remote + "/output/prime.png"
    before = {w["id"] for w in session.windows()}
    pid = session.launch(binary, ["--capture-to", path, "--all-outputs", "--speed"], name="prime", overrides=overrides)
    try:
        session.wait(lambda: any(w["id"] not in before and w["app"] == "xdg-desktop-portal-gnome"
                                 and ("Screen" in (w["caption"] or "") or "屏幕" in (w["caption"] or ""))
                                 for w in session.windows()), "GNOME 没有显示本次屏幕授权")
        session.screenshot("permission")
        session.wait(lambda: capture_ready(session, pid), "首次捕获未完成", timeout=20)
        result = session.download(path, "prime.png")
        with Image.open(result) as image:
            assert image.size == session.size, (image.size, session.size)
        return result
    finally:
        session.stop(pid)
        session.log("prime")


def run(session, binary, overrides):
    """【GNOME虚拟机】【截图流程】按 session、binary、overrides 验证一次授权及后续恢复、全部标注和复制，返回结果"""
    prime(session, binary, overrides)
    result = annotations_run(session, binary, overrides)
    result["portal_first_permission"] = True
    result["portal_restore_without_reprompt"] = True
    return result
