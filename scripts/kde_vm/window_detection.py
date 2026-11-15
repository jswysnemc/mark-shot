"""【KDE虚拟机】【窗口识别】关闭图像区域识别后，以真实 KWin 窗口边界验证悬停和短点击"""

import json
import time

from PIL import Image, ImageChops

from annotations import config


def hover_bounds(path):
    """【KDE虚拟机】【悬停边界】按 path 查找识别候选的薄荷色描边，返回像素边界；没有描边时返回空值"""
    with Image.open(path) as frame:
        image = frame.convert("RGB")
    points = [(x, y) for y in range(image.height) for x in range(image.width)
              if image.getpixel((x, y)) == (94, 234, 212)]
    return ([min(p[0] for p in points), min(p[1] for p in points),
             max(p[0] for p in points) + 1, max(p[1] for p in points) + 1] if points else None)


def check_hover(session, window, name, point=None):
    """【KDE虚拟机】【窗口边界】按 session、window、name、point 移动并核对实际描边，返回空值，容许一像素光栅取整"""
    session.move(*(point or (window["x"] + 200, window["y"] + 150)))
    bounds = hover_bounds(session.screenshot(name))
    assert bounds, "关闭区域识别后，KDE 窗口没有悬停选框"
    expected = [window["x"] - 1, window["y"] - 1,
                window["x"] + window["width"] + 1, window["y"] + window["height"] + 1]
    assert all(abs(a - b) <= 1 for a, b in zip(bounds, expected)), f"窗口识别边界错误: {bounds} != {expected}"


def position(session, window, x, y):
    """【KDE虚拟机】【来源定位】按 session、window、x、y 仅移动独立测试窗口并给焦点，返回更新后的窗口记录"""
    session.kwin("const w=workspace.windowList().find(w=>w.internalId.toString()===" + json.dumps(window["id"]) + ");w.frameGeometry={x:" + str(x) + ",y:" + str(y) + ",width:420,height:320};workspace.activeWindow=w;return true;")
    time.sleep(0.25)
    return next(w for w in session.windows() if w["id"] == window["id"])


def run(session, binary, overrides):
    """【KDE虚拟机】【窗口流程】按 session、binary、overrides 仅启用窗口识别，核对悬停与复制；返回结果并清理本次进程"""
    case = overrides.pop("MARK_SHOT_TEST_WINDOW_CASE", "only")
    configuration = overrides.pop("MARK_SHOT_TEST_CONFIG", None)
    assert case in ("only", "disabled", "overlap", "minimized"), case
    session.setup(config(session) + "[region_detection]\nenabled=false\n[window_detection]\nenabled=" + ("false" if case == "disabled" else "true") + "\n")
    # 1. 【KDE虚拟机】【锁屏排除】构建期间虚拟机可能自动锁屏，先解除本次已授权测试会话再核对窗口
    session.guest("subprocess.run(['sudo','-n','loginctl','unlock-session','2'],check=True)")
    session.key("shift")
    session.wait(lambda: session.guest("print(subprocess.check_output(['qdbus6','org.freedesktop.ScreenSaver','/ScreenSaver','org.freedesktop.ScreenSaver.GetActive'],text=True).strip())") == "false", "虚拟机仍然锁屏")
    session.guest("""
root=Path(values);(root/'runtime').mkdir(mode=0o700,exist_ok=True)
(root/'runtime/runtime.json').write_text(json.dumps({'private_kde_vm':True}))
""", session.remote)
    target = session.launch("/tmp/mark-shot-kde-detection-target", [session.remote + "/target.json"], name="target", with_config=False,
                            overrides={"XDG_RUNTIME_DIR": session.remote + "/runtime", "WAYLAND_DISPLAY": "/run/user/1000/wayland-1"})
    pid = None
    try:
        window = session.wait(lambda: session.windows("mark-shot-region-detection-target"), "测试窗口没有出现")[0]
        lower = position(session, window, 650, 140)
        window = lower
        if case in ("overlap", "minimized"):
            session.launch("/tmp/mark-shot-kde-detection-target", [session.remote + "/target-top.json"], name="target-top", with_config=False,
                           overrides={"XDG_RUNTIME_DIR": session.remote + "/runtime", "WAYLAND_DISPLAY": "/run/user/1000/wayland-1"})
            upper = session.wait(lambda: next((w for w in session.windows("mark-shot-region-detection-target") if w["id"] != lower["id"]), None), "第二个测试窗口没有出现")
            upper = position(session, upper, 730, 210)
            if case == "overlap":
                window = upper
            else:
                session.kwin("const w=workspace.windowList().find(w=>w.internalId.toString()===" + json.dumps(upper["id"]) + ");w.minimized=true;return true;")
                session.wait(lambda: any(w["id"] == upper["id"] and w["minimized"] for w in session.windows()), "测试窗口未最小化")
        session.move(600, 600)
        source = session.screenshot("source-window")
        (session.root / "expected-window.json").write_text(json.dumps(window, ensure_ascii=False, indent=2) + "\n")
        session.move(window["x"] + 100, window["y"] + 100)
        arguments = ["--config", configuration, "--speed"] if configuration else ["--speed"]
        pid = session.launch(binary, arguments, overrides=overrides, with_config=configuration is None)
        time.sleep(0.8)
        if case == "disabled":
            session.move(window["x"] + 200, window["y"] + 150)
            assert not hover_bounds(session.screenshot("window-hover")), "两个识别开关关闭后仍然出现候选"
            session.key("esc")
            return {"region_detection": False, "window_detection": False, "no_hover_candidate": True}
        if case in ("overlap", "minimized"):
            check_hover(session, lower, "lower-exposed", (lower["x"] + 30, lower["y"] + 30))
        point = (800, 300) if case in ("overlap", "minimized") else None
        check_hover(session, window, "window-hover", point)
        session.click(*(point or (window["x"] + 200, window["y"] + 150)))
        session.screenshot("window-selected")
        session.key("ctrl+c")
        session.wait(lambda: not session.alive(pid), "短点击选窗后复制没有完成")
        copied = session.clipboard()
        with Image.open(copied) as image:
            assert image.size == (window["width"], window["height"]), f"窗口选区尺寸不正确: {image.size}"
            with Image.open(source) as frame:
                expected = frame.convert("RGB").crop((window["x"], window["y"], window["x"] + window["width"], window["y"] + window["height"]))
                assert not ImageChops.difference(image.convert("RGB"), expected).getbbox(), "窗口复制像素与独立桌面裁切不一致"
        return {"case": case, "configuration": configuration or session.remote + "/config.toml",
                "region_detection": False, "window_detection": True, "hover_matches_kwin": True,
                "exact_pixels": True, "topmost_overlap": case == "overlap", "minimized_excluded": case == "minimized",
                "click_selected_window": True, "copied_size": [window["width"], window["height"]]}
    finally:
        if pid:
            session.stop(pid)
            session.log("capture")
        session.stop(target)
        session.log("target")
        session.cleanup()
