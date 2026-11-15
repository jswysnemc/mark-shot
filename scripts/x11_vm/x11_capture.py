"""【X11虚拟机】【截图回归】验证真实帧缓冲、客户区、指针及完整标注流程"""

import json
import time

from PIL import Image, ImageChops
from annotations import config, run as annotations_run


def run(session, binary, overrides):
    """【X11虚拟机】【捕获流程】按 session、binary、overrides 核对客户区、指针和标注，返回实际结果"""
    session.setup(config(session))
    session.guest("""
root=Path(values);(root/'runtime').mkdir(mode=0o700,exist_ok=True)
(root/'runtime/runtime.json').write_text(json.dumps({'private_x11_vm':True}))
""", session.remote)
    target = session.launch("/tmp/region-detection-target", [session.remote + "/target.json"], name="target", with_config=False,
                            overrides={"XDG_RUNTIME_DIR": session.remote + "/runtime"})
    try:
        window = session.wait(lambda: session.windows("mark-shot-region-detection-target"), "活动窗口来源未出现")[0]
        session.window_command(window["id"], ["wmctrl", "-ia"])
        session.move(1100, 650)
        time.sleep(0.4)
        expected = session.screenshot("framebuffer")
        path = session.remote + "/output/active.png"
        session.guest("""
environment=os.environ.copy()
for line in subprocess.check_output(['systemctl','--user','show-environment'],text=True).splitlines():
 key,_,value=line.partition('=');environment[key]=value
environment.update(values['overrides'])
result=subprocess.run([values['binary'],'--config',values['config'],'--window','--to','o','--nonotify','--speed'],env=environment,capture_output=True,check=True,timeout=10)
Path(values['path']).write_bytes(result.stdout)
""", {"binary": binary, "config": session.remote + "/config.toml", "path": path, "overrides": overrides})
        with Image.open(session.download(path, "active.png")) as actual, Image.open(expected) as source:
            area = (window["x"], window["y"], window["x"] + window["width"], window["y"] + window["height"])
            crop = source.convert("RGB").crop(area)
            assert actual.size == crop.size and not ImageChops.difference(actual.convert("RGB"), crop).getbbox(), "活动窗口客户区像素不一致"
        before = session.remote + "/output/pointer-off.png"
        after = session.remote + "/output/pointer-on.png"
        session.command(binary, ["--all-outputs", "--capture-to", before, "--speed"], overrides)
        session.command(binary, ["--all-outputs", "--include-cursor", "--capture-to", after, "--speed"], overrides)
        with Image.open(session.download(before, "pointer-off.png")) as a, Image.open(session.download(after, "pointer-on.png")) as b, Image.open(expected) as desktop:
            assert not ImageChops.difference(a.convert("RGB"), desktop.convert("RGB")).getbbox(), "整屏捕获不等于实际虚拟显卡画面"
            difference = ImageChops.difference(a.convert("RGB"), b.convert("RGB")).getbbox()
            assert difference and 1080 <= difference[0] <= 1120 and 630 <= difference[1] <= 670, ("指针没有出现在真实位置", difference)
        displays = json.loads(session.command(binary, ["--list-displays"]))
        (session.root / "displays.json").write_text(json.dumps(displays, indent=2) + "\n")
    finally:
        session.stop(target)
    result = annotations_run(session, binary, overrides)
    result.update(framebuffer_exact_pixels=True, active_window_exact_pixels=True, native_pointer=True, display_enumeration=displays)
    return result
