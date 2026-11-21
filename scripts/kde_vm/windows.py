"""【KDE虚拟机】【窗口验证】从置顶贴图进入编辑器，检查原生控制与原图交付"""

import base64
import io
import time

from PIL import Image, ImageChops
from annotations import config


def editor(session):
    """【KDE虚拟机】【编辑窗口】按 session 返回真实图片编辑窗口，缺少时返回空值"""
    values = session.windows("mark-shot-editor")
    return values[0] if values else None


def open_editor(session, pin):
    """【KDE虚拟机】【贴图菜单】按 session、pin 从右键菜单进入编辑，无返回值，使用真实键盘导航"""
    session.click(pin["x"] + 50, pin["y"] + 50, "right")
    session.key("home")
    for _ in range(6):
        session.key("down")
    session.key("ret")
    session.wait(lambda: editor(session), "贴图编辑菜单未打开编辑器")


def run(session, binary, overrides):
    """【KDE虚拟机】【窗口流程】按 session、binary、overrides 检查遮挡、控制和导出，返回结果；最后清理派生进程"""
    session.setup(config(session) + '[keys]\nextensions="Ctrl+E"\n')
    session.guest("""
path=Path(values)/'config/mark-shot/extensions.json';path.parent.mkdir(parents=True,exist_ok=True)
path.write_text(json.dumps([{'name':f'Controlled {i:02d}','command':'true','closeOnStart':False} for i in range(20)]))
""", session.remote)
    image = Image.new("RGB", (600, 340), (50, 100, 160))
    data = io.BytesIO()
    image.save(data, format="PNG")
    session.guest("import base64\nPath(values[0]).write_bytes(base64.b64decode(values[1]))",
                  [session.remote + "/source.png", base64.b64encode(data.getvalue()).decode()])
    before = {window["id"] for window in session.windows()}
    pid = session.launch(binary, ["--pin-image", session.remote + "/source.png"], overrides=overrides)
    try:
        pin = session.wait(lambda: next((w for w in session.windows() if w["id"] not in before and w["width"] == 600), None), "没有打开贴图")
        pin_identity = (pin["app"], pin["caption"])
        session.screenshot("pin")
        open_editor(session, pin)
        session.screenshot("opened-from-pin")
        session.wait(lambda: not any(w["id"] == pin["id"] for w in session.windows()), "置顶贴图仍遮挡编辑器")
        shown = editor(session)
        session.click(shown["x"] + shown["width"] - 55, shown["y"] + 21)
        shown = session.wait(lambda: (w if w and w["width"] >= 1270 else None) if (w := editor(session)) else None, "最大化按钮没有扩大窗口")
        session.screenshot("maximized")
        session.click(shown["x"] + shown["width"] - 89, shown["y"] + 21)
        session.wait(lambda: editor(session)["minimized"], "最小化按钮未生效")
        session.screenshot("minimized")
        session.restore(shown["id"])
        time.sleep(0.2)
        shown = editor(session)
        session.click(shown["x"] + shown["width"] - 25, shown["y"] + 21)
        session.wait(lambda: not editor(session), "关闭按钮未关闭编辑器")
        pin = session.wait(lambda: next((w for w in session.windows() if (w["width"], w["height"]) == image.size
                                       and (w["app"], w["caption"]) == pin_identity), None),
                           "编辑器关闭后贴图未恢复")
        session.screenshot("pin-restored")
        open_editor(session, pin)
        session.key("ctrl+e")
        shown = editor(session)
        session.move(shown["x"] + 700, shown["y"] + 100)
        time.sleep(0.2)
        before_menu = session.screenshot("extensions-before")
        session.move(shown["x"] + 800, shown["y"] + 100)
        session.scroll(steps=4)
        session.move(shown["x"] + 700, shown["y"] + 100)
        time.sleep(0.2)
        down_menu = session.screenshot("extensions-down")
        session.move(shown["x"] + 800, shown["y"] + 100)
        session.scroll(down=False, steps=4)
        session.move(shown["x"] + 700, shown["y"] + 100)
        time.sleep(0.2)
        up_menu = session.screenshot("extensions-up")
        with Image.open(before_menu) as a, Image.open(down_menu) as b, Image.open(up_menu) as c:
            area = (shown["x"] + 750, shown["y"] + 70, shown["x"] + 1034, shown["y"] + 285)
            before, down, up = [value.convert("RGB").crop(area) for value in (a, b, c)]
            assert ImageChops.difference(before, down).getbbox(), "向下滚轮没有显示后续扩展"
            assert not ImageChops.difference(before, up).getbbox(), "向上滚轮没有回到列表开头"
        session.key("esc")
        session.key("ctrl+c")
        session.wait(lambda: not editor(session), "复制后编辑器未关闭")
        copied = session.clipboard()
        with Image.open(copied) as result:
            assert result.size == image.size and result.convert("RGB").tobytes() == image.tobytes(), "编辑器改变了原图导出"
        # 1. 【KDE虚拟机】【设置交接】关闭原贴图后从截图侧栏进入设置，截图窗口必须先退出
        session.stop(pid)
        session.wait(lambda: not session.alive(pid), "贴图进程未退出")
        pid = session.launch(binary, ["--speed"], name="settings", overrides=overrides)
        time.sleep(0.8)
        session.drag((150, 100), (950, 550))
        session.screenshot("settings-selection")
        capture_ids = {w["id"] for w in session.windows() if w.get("pid") == pid or w["app"] == binary.rsplit("/", 1)[-1]}
        assert capture_ids, "没有截图表面"
        session.click(976, 499)
        session.wait(lambda: session.windows("mark-shot-settings"), "侧栏设置按钮未打开设置窗口")
        session.wait(lambda: not any(w["id"] in capture_ids for w in session.windows()), "打开设置后截图表面仍存活")
        session.screenshot("settings-opened")
        return {"pin_editor_not_obscured": True, "pin_restored_on_close": True,
                "maximize": True, "minimize": True, "close": True, "original_export_equal": True,
                "sidebar_settings_closed_capture": True, "extensions_wheel_down_up": True}
    finally:
        session.stop(pid)
        session.log("capture")
        session.cleanup()
