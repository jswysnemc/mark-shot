"""【X11虚拟机】【独立识别】区分图像候选和原生可见窗口，验证堆叠及最小化过滤"""

import json
import time

from PIL import Image, ImageChops
from annotations import config
from window_detection import hover_bounds


def position(session, window, x, y):
    """【X11虚拟机】【来源位置】按 session、window、x、y 移动测试客户区并聚焦，返回最新窗口记录"""
    session.guest("""
environment=os.environ.copy()
for line in subprocess.check_output(['systemctl','--user','show-environment'],text=True).splitlines():
 key,_,value=line.partition('=');environment[key]=value
subprocess.run(['xdotool','windowmove',values['id'],str(values['x']),str(values['y'])],env=environment,check=True)
subprocess.run(['wmctrl','-ia',values['id']],env=environment,check=True)
""", {"id": window["id"], "x": x, "y": y})
    time.sleep(0.3)
    return next(w for w in session.windows() if w["id"] == window["id"])


def hover(session, expected, point, name):
    """【X11虚拟机】【悬停核对】按 session、expected、point、name 等待正确边界呈现，返回实际边界"""
    session.move(*point)

    def shown():
        """【X11虚拟机】【边界等待】无参数，匹配期望边界则返回边界，过渡帧返回空值"""
        bounds = hover_bounds(session.screenshot(name))
        return bounds if bounds and all(abs(a - b) <= 1 for a, b in zip(bounds, expected)) else None

    return session.wait(shown, f"悬停边界不等于原生来源: {expected}", timeout=12)


def run(session, binary, overrides):
    """【X11虚拟机】【候选流程】按 session、binary、overrides 验证识别开关、窗口堆叠和导出像素，返回结果"""
    case = overrides.pop("MARK_SHOT_TEST_DETECTION", "window")
    assert case in ("region", "window", "disabled", "overlap", "minimized")
    session.setup(config(session) + '[region_detection]\nenabled=' + str(case == "region").lower()
                  + '\n[window_detection]\nenabled=' + str(case not in ("region", "disabled")).lower() + '\n')
    session.guest("""
root=Path(values);(root/'runtime').mkdir(mode=0o700,exist_ok=True)
(root/'runtime/runtime.json').write_text(json.dumps({'private_x11_vm':True}))
""", session.remote)
    session.launch("/tmp/region-detection-target", [session.remote + "/target.json"], name="target", with_config=False,
                   overrides={"XDG_RUNTIME_DIR": session.remote + "/runtime"})
    try:
        lower = position(session, session.wait(lambda: session.windows("mark-shot-region-detection-target"), "识别来源未出现")[0], 620, 120)
        chosen = lower
        if case in ("overlap", "minimized"):
            session.launch("/tmp/region-detection-target", [session.remote + "/upper.json"], name="upper", with_config=False,
                           overrides={"XDG_RUNTIME_DIR": session.remote + "/runtime"})
            upper = session.wait(lambda: next((w for w in session.windows("mark-shot-region-detection-target") if w["id"] != lower["id"]), None), "第二个识别来源未出现")
            upper = position(session, upper, 710, 200)
            if case == "minimized":
                session.window_command(upper["id"], ["xdotool", "windowminimize"])
                session.wait(lambda: next(w for w in session.windows() if w["id"] == upper["id"])["minimized"], "上层来源未最小化")
            else:
                chosen = upper
        if case == "region":
            trace = json.loads(session.wait(lambda: session.guest("p=Path(values);print(p.read_text() if p.exists() else '')", session.remote + "/target.json"), "控件边界未记录"))
            child = trace["child"]
            chosen = dict(x=lower["x"] + child[0], y=lower["y"] + child[1], width=child[2], height=child[3])
        session.move(1100, 650)
        time.sleep(0.3)
        source = session.screenshot("detection-source")
        pid = session.launch(binary, ["--speed"], overrides=overrides)
        session.wait(lambda: any(w["pid"] == pid for w in session.windows()), "识别截图界面未出现")
        if case == "region":
            session.wait(lambda: "【截图区域识别】【完成接入】" in session.log("capture"), "图像候选没有完成", timeout=12)
        time.sleep(0.4)
        point = (chosen["x"] + chosen["width"] / 2, chosen["y"] + chosen["height"] / 2)
        if case == "disabled":
            session.move(*point)
            assert not hover_bounds(session.screenshot("disabled-hover")), "两个识别开关关闭后仍然出现候选"
            session.key("esc")
            return {"case": case, "no_hover_candidate": True}
        if case in ("overlap", "minimized"):
            hover(session, [lower["x"] - 1, lower["y"] - 1, lower["x"] + lower["width"] + 1, lower["y"] + lower["height"] + 1],
                  (lower["x"] + 30, lower["y"] + 30), "lower-exposed")
            point = (800, 320)
        expected = [chosen["x"] - 1, chosen["y"] - 1, chosen["x"] + chosen["width"] + 1, chosen["y"] + chosen["height"] + 1]
        bounds = hover(session, expected, point, "chosen-hover")
        session.click(*point)
        session.key("ctrl+c")
        session.wait(lambda: not session.alive(pid), "点击识别区域后复制没有退出")
        with Image.open(session.clipboard()) as result, Image.open(source) as original:
            crop = original.convert("RGB").crop((chosen["x"], chosen["y"], chosen["x"] + chosen["width"], chosen["y"] + chosen["height"]))
            assert result.size == crop.size and not ImageChops.difference(result.convert("RGB"), crop).getbbox(), "识别区域导出不等于真实桌面裁切"
        return {"case": case, "region_detection": case == "region", "window_detection": case != "region",
                "hover_bounds": bounds, "copied_size": list(crop.size), "exact_pixels": True,
                "topmost_overlap": case == "overlap", "minimized_excluded": case == "minimized"}
    finally:
        session.log("capture")
        session.log("target")
        session.cleanup()
