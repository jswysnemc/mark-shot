"""【GNOME虚拟机】【识别引导】验证未安装模型的资源界面及取消恢复，不下载模型"""

import base64
import io

from PIL import Image
from annotations import config


def run(session, binary, overrides, menu_steps=7):
    """【虚拟机测试】【识别取消】按 session、binary、overrides、menu_steps 打开资源引导后取消，返回来源保留与无下载结果"""
    session.setup(config(session))
    output = io.BytesIO()
    Image.new("RGB", (600, 340), (80, 100, 120)).save(output, format="PNG")
    session.guest("import base64\nPath(values[0]).write_bytes(base64.b64decode(values[1]))",
                  [session.remote + "/source.png", base64.b64encode(output.getvalue()).decode()])
    try:
        pid = session.launch(binary, ["--pin-image", session.remote + "/source.png"], overrides=overrides)
        pin = session.wait(lambda: session.windows("mark-shot-pin"), "识别来源贴图未出现")[0]
        session.click(pin["x"] + 50, pin["y"] + 50, "right")
        session.key("home")
        for _ in range(menu_steps):
            session.key("down")
        session.key("ret")
        guide = session.wait(lambda: session.windows("mark-shot-ocr-model-guide") or session.windows("mark-shot-ocr-resources"), "未安装模型时没有显示识别引导")[0]
        session.screenshot("ocr-guide")
        session.key("esc")
        session.wait(lambda: not any(w["id"] == guide["id"] for w in session.windows()), "识别引导取消后没有关闭")
        session.wait(lambda: any(w["id"] == pin["id"] for w in session.windows()), "取消识别后来源贴图未保留")
        session.screenshot("ocr-cancelled")
        assert session.alive(pid), "取消识别错误关闭来源贴图"
        return {"missing_model_guide": True, "guide_cancelled": True, "source_pin_preserved": True,
                "models_downloaded": False, "recognition_service_tested": False}
    finally:
        session.log("capture")
        session.cleanup()
