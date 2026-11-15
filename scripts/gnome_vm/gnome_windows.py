"""【GNOME虚拟机】【窗口交互】验证贴图编辑交接、原生控制、设置真实值和菜单滚轮"""

import base64
import io
import time

from PIL import Image, ImageChops
from annotations import config
from gnome_capture import prime
from windows import editor


def open_editor(session, pin):
    """【GNOME虚拟机】【贴图菜单】按 session、pin 使用真实键盘进入编辑，跳过禁用的置顶项，无返回值"""
    session.click(pin["x"] + 50, pin["y"] + 50, "right")
    session.key("home")
    for _ in range(5):
        session.key("down")
    session.key("ret")
    session.wait(lambda: editor(session), "贴图编辑菜单未打开编辑器")


def run(session, binary, overrides):
    """【GNOME虚拟机】【窗口流程】按 session、binary、overrides 进入贴图编辑并检查控制、复制、设置及菜单，返回结果"""
    prime(session, binary, overrides)
    session.setup(config(session) + '[keys]\nextensions="Ctrl+E"\n')
    session.guest("""
root=Path(values);path=root/'config/mark-shot/extensions.json';path.parent.mkdir(parents=True,exist_ok=True)
path.write_text(json.dumps([{'name':f'Controlled {i:02d}','command':'true','closeOnStart':False} for i in range(20)]))
""", session.remote)
    source = Image.new("RGB", (600, 340), (50, 100, 160))
    payload = io.BytesIO()
    source.save(payload, format="PNG")
    session.guest("import base64\nPath(values[0]).write_bytes(base64.b64decode(values[1]))",
                  [session.remote + "/source.png", base64.b64encode(payload.getvalue()).decode()])
    before = {w["id"] for w in session.windows()}
    pin_pid = session.launch(binary, ["--pin-image", session.remote + "/source.png"])
    try:
        pin = session.wait(lambda: next((w for w in session.windows() if w["id"] not in before and w["width"] == 600), None), "贴图没有打开")
        session.screenshot("pin")
        open_editor(session, pin)
        session.wait(lambda: not any(w["id"] == pin["id"] for w in session.windows()), "来源贴图没有隐藏")
        session.screenshot("editor-from-pin")
        shown = editor(session)
        session.click(shown["x"] + shown["width"] - 55, shown["y"] + 21)
        shown = session.wait(lambda: (w if w and w["width"] >= 1270 else None) if (w := editor(session)) else None, "最大化未生效")
        session.screenshot("maximized")
        session.click(shown["x"] + shown["width"] - 89, shown["y"] + 21)
        session.wait(lambda: editor(session)["minimized"], "最小化未生效")
        session.screenshot("minimized")
        session.restore(shown["id"])
        time.sleep(0.25)
        shown = editor(session)
        session.click(shown["x"] + shown["width"] - 25, shown["y"] + 21)
        session.wait(lambda: not editor(session), "关闭未生效")
        pin = session.wait(lambda: next((w for w in session.windows() if w["width"] == 600), None), "关闭编辑后贴图未恢复")
        session.screenshot("pin-restored")
        open_editor(session, pin)
        session.key("ctrl+e")
        initial = session.screenshot("extensions-before")
        shown = editor(session)
        session.move(shown["x"] + 800, shown["y"] + 100)
        session.scroll(steps=4)
        down = session.screenshot("extensions-down")
        session.scroll(down=False, steps=4)
        up = session.screenshot("extensions-up")
        with Image.open(initial) as a, Image.open(down) as b, Image.open(up) as c:
            area = (shown["x"] + 750, shown["y"] + 70, shown["x"] + 1034, shown["y"] + 285)
            initial, down, up = [image.convert("RGB").crop(area) for image in (a, b, c)]
            assert ImageChops.difference(initial, down).getbbox(), "下滚没有显示后续条目"
            assert not ImageChops.difference(initial, up).getbbox(), "上滚没有恢复列表"
        session.key("esc")
        session.key("ctrl+c")
        session.wait(lambda: not editor(session), "编辑复制后未关闭")
        with Image.open(session.clipboard()) as image:
            assert image.size == source.size and image.convert("RGB").tobytes() == source.tobytes(), "原图复制像素变化"
        session.stop(pin_pid)
        session.wait(lambda: not session.alive(pin_pid), "贴图进程未退出")
        pid = session.launch(binary, ["--speed"], name="settings-capture")
        session.wait(lambda: any(w["pid"] == pid for w in session.windows()), "截图编辑器未出现")
        time.sleep(0.3)
        session.drag((150, 100), (950, 550))
        capture_ids = {w["id"] for w in session.windows() if w["pid"] == pid}
        session.click(976, 483)
        settings = session.wait(lambda: session.windows("mark-shot-settings"), "侧栏未打开设置")[0]
        assert not any(w["id"] in capture_ids for w in session.windows()), "打开设置后截图表面仍存在"
        for index, name in [(1, "settings-capture"), (7, "settings-storage"), (10, "settings-log")]:
            session.click(settings["x"] + 70, settings["y"] + 56 + index * 36 + 16)
            session.screenshot(name)
        session.click(settings["x"] + settings["width"] - 26, settings["y"] + 24)
        session.wait(lambda: not session.windows("mark-shot-settings"), "设置关闭失败")
        return {"pin_edit_hide_restore": True, "maximize": True, "minimize": True, "close": True,
                "extensions_scroll_direction": True, "original_png_equal": True,
                "settings_closed_capture_first": True, "settings_effective_values": True}
    finally:
        session.log("capture")
        session.log("settings-capture")
        session.cleanup()
