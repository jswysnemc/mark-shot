"""【GNOME虚拟机】【识别与长图】以独立纹理和灰色控件区分图像识别、窗口识别与滚动捕获"""

import json
import time

from PIL import Image, ImageChops
from annotations import config
from gnome_capture import prime
from scrolling import run as scrolling_run
from window_detection import hover_bounds


def region_hover(session, expected):
    """【GNOME虚拟机】【悬停呈现】按 session、expected 核对已经呈现的候选，正确返回边界，旧帧返回空值"""
    bounds = hover_bounds(session.screenshot("detection-hover"))
    return bounds if bounds and all(abs(a - b) <= 1 for a, b in zip(bounds, expected)) else None


def detection(session, binary, overrides, case):
    """【GNOME虚拟机】【识别流程】按 session、binary、overrides、case 核对控件候选和原始导出，返回支持范围"""
    assert case in ("region", "window", "disabled"), case
    session.setup(config(session) + '[region_detection]\nenabled=' + str(case == "region").lower()
                  + '\n[window_detection]\nenabled=' + str(case == "window").lower() + '\n')
    session.guest("""
root=Path(values);(root/'runtime').mkdir(mode=0o700,exist_ok=True)
(root/'runtime/runtime.json').write_text(json.dumps({'private_gnome_vm':True}))
""", session.remote)
    session.launch("/tmp/region-detection-target", [session.remote + "/target.json"], name="target", with_config=False,
                   overrides={"XDG_RUNTIME_DIR": session.remote + "/runtime", "WAYLAND_DISPLAY": "/run/user/1000/wayland-0"})
    try:
        window = session.wait(lambda: session.windows("mark-shot-region-detection-target"), "识别来源窗口未出现")[0]
        trace = json.loads(session.wait(lambda: session.guest("p=Path(values);print(p.read_text() if p.exists() else '')", session.remote + "/target.json"), "来源几何未写入"))
        child = trace["child"]
        assert (trace["width"], trace["height"]) == (window["width"], window["height"]), (trace, window)
        left, top = window["x"] + child[0], window["y"] + child[1]
        point = (left + child[2] / 2, top + child[3] / 2)
        session.move(1100, 650)
        time.sleep(0.3)
        source = session.screenshot("detection-source")
        pid = session.launch(binary, ["--speed"], overrides=overrides)
        session.wait(lambda: any(w["pid"] == pid for w in session.windows()), "识别截图界面未出现")
        if case == "region":
            session.wait(lambda: "【截图区域识别】【完成接入】" in session.log("capture"), "图像区域识别任务没有完成", timeout=12)
        time.sleep(0.5)
        session.move(*point)
        (session.root / "pointer.json").write_text(json.dumps({"expected": point, "actual": session.evaluate("global.get_pointer()"),
                                                              "windows": session.windows()}, ensure_ascii=False, indent=2))
        bounds = hover_bounds(session.screenshot("detection-hover"))
        if case != "region":
            assert not bounds, f"GNOME 不应把图像控件伪装为窗口识别: {bounds}"
            session.key("esc")
            return {"region_detection": False, "window_detection": case == "window",
                    "native_window_detection_supported": False, "no_hover_candidate": True}
        expected = [left - 1, top - 1, left + child[2] + 1, top + child[3] + 1]
        session.wait(lambda: region_hover(session, expected), "GNOME 控件悬停框没有正确呈现", timeout=12)
        session.click(*point)
        session.key("ctrl+c")
        session.wait(lambda: not session.alive(pid), "识别复制后截图未退出")
        with Image.open(session.clipboard()) as result, Image.open(source) as original:
            crop = original.convert("RGB").crop((left, top, left + child[2], top + child[3]))
            assert result.size == crop.size and not ImageChops.difference(result.convert("RGB"), crop).getbbox(), "区域识别导出像素不一致"
        return {"region_detection": True, "window_detection": False, "child_hover_geometry": True,
                "click_selected_region": True, "copied_size": list(crop.size), "exact_pixels": True}
    finally:
        session.log("capture")
        session.log("target")
        session.cleanup()


def run(session, binary, overrides):
    """【GNOME虚拟机】【流程入口】按 session、binary、overrides 完成门户准备，返回长图或独立识别检查结果"""
    case = overrides.pop("MARK_SHOT_TEST_DETECTION", "scroll")
    prime(session, binary, overrides)
    if case == "scroll":
        return scrolling_run(session, binary, overrides, "/tmp/scroll-page", "/run/user/1000/wayland-0", hidden_controls=True)
    return detection(session, binary, overrides, case)
