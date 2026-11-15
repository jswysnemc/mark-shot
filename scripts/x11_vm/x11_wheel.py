"""【X11虚拟机】【滚轮方向】仅用编辑器扩展列表和真实鼠标刻度检查上下方向"""

import base64
import io
import time

from PIL import Image, ImageChops
from annotations import config


def run(session, binary, overrides):
    """【X11虚拟机】【单刻度回归】按 session、binary、overrides 核对下滚推进和上滚复位，返回正确方向标志"""
    session.setup(config(session) + '[keys]\nextensions="Ctrl+E"\n')
    data = io.BytesIO()
    Image.new("RGB", (600, 340), (50, 100, 160)).save(data, format="PNG")
    session.guest("""
import base64
root=Path(values[0]);(root/'source.png').write_bytes(base64.b64decode(values[1]))
path=root/'config/mark-shot/extensions.json';path.parent.mkdir(parents=True,exist_ok=True)
path.write_text(json.dumps([{'name':f'Controlled {i:02d}','command':'true','closeOnStart':False} for i in range(20)]))
""", [session.remote, base64.b64encode(data.getvalue()).decode()])
    try:
        session.launch(binary, [session.remote + "/source.png"], overrides=overrides)
        shown = session.wait(lambda: session.windows("mark-shot-editor"), "滚轮测试编辑器未出现")[0]
        session.click(shown["x"] + 40, shown["y"] + 20)
        session.key("ctrl+e")
        session.move(shown["x"] + 800, shown["y"] + 100)
        time.sleep(0.3)
        before = session.screenshot("before")
        session.scroll(down=True, steps=1)
        time.sleep(0.3)
        down = session.screenshot("down")
        session.scroll(down=False, steps=1)
        time.sleep(0.3)
        up = session.screenshot("up")
        with Image.open(before) as a, Image.open(down) as b, Image.open(up) as c:
            area = (shown["x"] + 755, shown["y"] + 44, shown["x"] + 1020, shown["y"] + 270)
            original, advanced, restored = [image.convert("RGB").crop(area) for image in (a, b, c)]
            assert ImageChops.difference(original, advanced).getbbox(), "向下一个刻度没有推进列表"
            assert not ImageChops.difference(original, restored).getbbox(), "向上一个刻度没有恢复初始列表"
        return {"wheel_down_advances": True, "wheel_up_restores": True, "one_physical_tick_each": True}
    finally:
        session.log("capture")
        session.cleanup()
