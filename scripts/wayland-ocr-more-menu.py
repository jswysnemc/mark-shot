#!/usr/bin/env python3
"""【更多菜单测试】【真实桌面】验证独立父子弹层、键盘分层和单次悬停，不启动识别或翻译任务"""

import argparse
import hashlib
import json
from pathlib import Path
import select
import subprocess
import time

import wayland_text_format_session as session
from wayland_ocr_popup_tree import active_tree, pointer_position, popup_tree
from wayland_text_pointer import CURRENT, click, pointer_context
from wayland_virtual_pointer import transform_for_pointer
from wayland_window_input_session import find_node, frame, geometry

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = json.loads((ROOT / "tests/fixtures/qt-ocr-more-menu.json").read_text())


def move(point):
    """【更多菜单测试】【实际移动】向已校验的私有设备发送 point，不重复发送首个移动、不替换设备，无返回值"""
    pointer, logical, _, _ = CURRENT.get()
    pointer.move_abs(*transform_for_pointer(point[0] - logical["x"], point[1] - logical["y"],
        logical["width"], logical["height"], logical.get("pointer_transform", logical["transform"])))


def wait_tree(log, count):
    """【更多菜单测试】【实际呈现】等待 log 中 count 个存活节点均配置并提交，返回节点；错误或超时保留真实失败"""
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        nodes = active_tree(log.read_text())
        if len(nodes) == count and all(node["rect"] is not None and node["frames"] for node in nodes):
            return nodes
        time.sleep(0.015)
    raise AssertionError(f"expected {count} presented popup nodes, received {active_tree(log.read_text())}")


def row_point(window, nodes, depth=0):
    """【更多菜单测试】【实际行坐标】按 window、nodes 的 depth 返回首行全局内部点，子坐标累加真实父配置"""
    x, y = window["x"], window["y"]
    for node in nodes[:depth + 1]:
        x += node["rect"][0]
        y += node["rect"][1]
    rect = FIXTURE["root" if depth == 0 else "child"]["rows"][0]["rect"]
    return x + rect[0] + 20, y + rect[1] + rect[3] / 2


def child_geometry(nodes):
    """【更多菜单测试】【原版避让】按 nodes 校验两层尺寸、首行对齐和左右两像素重叠，无返回值"""
    root, child = nodes
    assert root["rect"][2:] == FIXTURE["root"]["size"]
    assert child["rect"][2:] == FIXTURE["child"]["size"]
    assert child["rect"][1] == 0, "submenu first row is not aligned with its owner"
    assert child["rect"][0] in (root["rect"][2] - 2, 2 - child["rect"][2]), "submenu overlap differs from Qt"
    assert root["surface"] != child["surface"]


def narrow_left_window(pid, environment):
    """【更多菜单测试】【右向空间】把 pid 对应私有窗口设为左侧窄窗口，返回稳定后的真实位置，不改动用户桌面"""
    if session.backend() is session.niri:
        window = session.niri.size_window(session.niri.wait_window(pid), 380, 340)
        session.niri.control("action", "move-floating-window", "--id", window["id"], "--x", "40", "--y", "80")
    else:
        binary = ROOT / "target/test-tools/sway/usr/bin/swaymsg"
        subprocess.run([str(binary), f"[pid={pid}] resize set width 380 px height 340 px, move position 40 80"],
            env=environment, check=True, stdout=subprocess.DEVNULL)
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        if session.backend() is session.niri:
            window = session.niri.wait_window(pid)
            left, top = window["floating_pos"]
            width, height = window["size"]
            bounds = {"x": left, "y": top, "width": width, "height": height}
        else:
            tree = json.loads(subprocess.check_output([str(binary), "-r", "-t", "get_tree"], env=environment))
            bounds = find_node(tree, pid)["rect"]
        if bounds["x"] == 40 and bounds["y"] == 80 and bounds["width"] == 380 and bounds["height"] == 340:
            time.sleep(0.08)
            return bounds
        time.sleep(0.02)
    raise AssertionError("private compositor did not apply the narrow left window geometry")


def run_reference(binary, root):
    """【更多菜单测试】【完整原版】在私有桌面执行只读 binary，root 保存真实 Qt 记录，初次打开必须经过外部点击，无返回值"""
    environment = session.environment()
    root.mkdir(parents=True, exist_ok=False)
    environment.update(MARK_SHOT_PRIVATE_QT_REFERENCE=environment["XDG_RUNTIME_DIR"], QT_QPA_PLATFORM="wayland")
    with session.input_context(), pointer_context(), (root / "process.log").open("w") as output:
        process = subprocess.Popen([str(binary), str(root), "--wayland"], env=environment, stdout=subprocess.PIPE, stderr=output)
        try:
            assert select.select([process.stdout], [], [], 8)[0] and process.stdout.readline() == b"ready\n"
            window = geometry(process.pid, environment)
            click((window["x"] + window["width"] - 85, window["y"] + 19))
            result = json.loads(process.communicate(timeout=10)[0])
            assert process.returncode == 0
            (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
            for name in ("root", "child", "states", "source", "triggered", "window_visible"):
                assert result[name] == FIXTURE[name], f"Qt Wayland reference changed: {name}"
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)


def run(binary, root):
    """【更多菜单测试】【实际窗口】在私有会话运行 binary，root 保存截图与协议；失败回收本次进程，不改变用户配置，无返回值"""
    environment = session.environment()
    root.mkdir(parents=True, exist_ok=False)
    environment.update(XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"), XDG_STATE_HOME=str(root / "state"))
    log = root / "process.log"
    with session.input_context(), pointer_context(), log.open("w") as output:
        if session.backend() is session.sway:
            session.sway.keyboard_ready()
        process = subprocess.Popen([str(binary), "--ocr-result", "--nonotify"], env=dict(environment, WAYLAND_DEBUG="client"),
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=output)
        try:
            process.stdin.write(frame())
            process.stdin.close()
            assert select.select([process.stdout], [], [], 5)[0] and process.stdout.readline() == b"\n"
            window = geometry(process.pid, environment)
            more = (window["x"] + window["width"] - 85, window["y"] + 19)
            subprocess.run(["wl-copy", "--", "more sentinel"], env=environment, check=True, timeout=3)
            # 1. 【更多菜单测试】【释放打开】右键和 Return 只聚焦，左键按下也不打开父菜单
            click(more, "right")
            session.key("Return")
            assert active_tree(log.read_text()) == []
            move(more)
            time.sleep(0.03)
            pointer = CURRENT.get()[0]
            pointer.press()
            time.sleep(0.1)
            assert active_tree(log.read_text()) == [], "More opened on press instead of release"
            pointer.release()
            nodes = wait_tree(log, 1)
            assert nodes[0]["rect"][2:] == FIXTURE["root"]["size"]
            assert nodes[0]["rect"][1] == 36, "root is not four pixels below the real More button"
            session.screenshot(root / "root.png")
            # 2. 【更多菜单测试】【逐层键盘】父 Left 保留；右方向键展开独立子层，禁用说明不执行也不写入正文
            session.key("Left")
            wait_tree(log, 1)
            session.key("Down")
            session.key("Right")
            nodes = wait_tree(log, 2)
            child_geometry(nodes)
            session.screenshot(root / "child.png")
            for key in ("Down", "Up", "Return", "Space", "ctrl+c", "ctrl+z"):
                session.key(key)
                wait_tree(log, 2)
            assert session.read_clipboard(environment) == "more sentinel"
            session.key("Left")
            wait_tree(log, 1)
            session.key("Right")
            wait_tree(log, 2)
            session.key("Escape")
            wait_tree(log, 1)
            session.key("Escape")
            wait_tree(log, 0)
            assert process.poll() is None, "root Escape leaked into the result window"
            # 3. 【更多菜单测试】【新抓取】原按钮保持焦点，Space 释放产生新的真实根会话，不依赖补发指针移动
            session.key("Space")
            nodes = wait_tree(log, 1)
            point = row_point(window, nodes)
            move(point)
            started = time.monotonic()
            time.sleep(0.05)
            assert len(active_tree(log.read_text())) == 1, "submenu opened before the hover deadline"
            actual = pointer_position(log.read_text(), nodes[0]["surface"])
            assert actual is not None and abs(actual[0] - 24) < 1 and abs(actual[1] - 18.5) < 1, "first post-close pointer move was not delivered; refusing a compensating move"
            move((point[0] + 1, point[1]))
            nodes = wait_tree(log, 2)
            hover_elapsed = time.monotonic() - started
            assert 0.16 <= hover_elapsed < 0.8, f"hover deadline differs from Qt: {hover_elapsed}"
            child_geometry(nodes)
            # 4. 【更多菜单测试】【说明隔离】禁用行左右点击不关闭，也不执行动作；空白父边延迟关闭子层
            click(row_point(window, nodes, 1))
            click(row_point(window, nodes, 1), "right")
            wait_tree(log, 2)
            assert session.read_clipboard(environment) == "more sentinel"
            blank = (window["x"] + nodes[0]["rect"][0] + 1, window["y"] + nodes[0]["rect"][1] + 1)
            move(blank)
            time.sleep(0.05)
            wait_tree(log, 2)
            move(row_point(window, nodes, 1))
            time.sleep(0.22)
            wait_tree(log, 2)
            session.key("Left")
            wait_tree(log, 1)
            # 5. 【更多菜单测试】【悬停取消】进入父行再离开整棵菜单，未执行期限不得打开子层
            move(point)
            time.sleep(0.05)
            move((window["x"] + 150, window["y"] + 180))
            time.sleep(0.25)
            wait_tree(log, 1)
            session.key("Escape")
            wait_tree(log, 0)
            # 6. 【更多菜单测试】【按下展开】新根使用 Space 打开，父行左键按下立刻展开；外部点击逆序关闭整树
            session.key("Space")
            nodes = wait_tree(log, 1)
            move(row_point(window, nodes))
            time.sleep(0.02)
            pointer.press()
            nodes = wait_tree(log, 2)
            pointer.release()
            wait_tree(log, 2)
            click((window["x"] + 180, window["y"] + 180))
            wait_tree(log, 0)
            assert session.read_clipboard(environment) == "more sentinel"
            # 7. 【更多菜单测试】【静止资源】菜单打开但无悬停期限时没有周期提交、等待线程或模型初始化
            click(more, "right")
            session.key("Space")
            wait_tree(log, 1)
            session.wait_single_thread(process.pid)
            before = session.resources(process.pid)
            frames = log.read_text().count("wl_surface")
            time.sleep(1.1)
            after = session.resources(process.pid)
            assert before["threads"] == after["threads"] == 1 and before["cpu_ticks"] == after["cpu_ticks"]
            assert log.read_text().count("wl_surface") == frames, "idle menu submitted periodic frames"
            session.key("Escape")
            wait_tree(log, 0)
            # 8. 【更多菜单测试】【双向避让】左侧窄窗口必须向右展开，不能只验证屏幕右侧的左向翻转
            window = narrow_left_window(process.pid, environment)
            session.key("Space")
            wait_tree(log, 1)
            session.key("Down")
            session.key("Right")
            rightward = wait_tree(log, 2)
            child_geometry(rightward)
            assert rightward[1]["rect"][0] == rightward[0]["rect"][2] - 2, "submenu did not use available right-side space"
            session.screenshot(root / "rightward-child.png")
            session.key("Escape")
            wait_tree(log, 1)
            session.key("Escape")
            wait_tree(log, 0)
            session.key("ctrl+shift+c")
            session.wait_clipboard(environment, "base")
            session.key("Escape")
            assert process.wait(timeout=3) == 0
            nodes = popup_tree(log.read_text(), closed=True)
            assert not (root / "data/mark-shot/models").exists()
            proof = {"sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "compositor": "sway" if session.backend() is session.sway else "niri",
                "nodes": nodes, "hover_elapsed": hover_elapsed, "independent_parent_child_surfaces": True,
                "layered_keyboard_close": True, "disabled_no_delivery": True, "hover_cancel_and_child_entry": True,
                "press_opens_child": True, "both_placement_directions": True, "fresh_root_grabs": True, "reverse_destruction": True, "source_unchanged": True,
                "idle_before": before, "idle_after": after, "no_idle_frames": True, "no_ocr": True}
            (root / "result.json").write_text(json.dumps(proof, indent=2) + "\n")
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=3)


def main():
    """【更多菜单测试】【入口】读取 binary、root 和 reference 模式后执行本地对照，无参数，失败返回非零状态"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--reference", action="store_true")
    args = parser.parse_args()
    (run_reference if args.reference else run)(args.binary.resolve(), args.root.resolve())


if __name__ == "__main__":
    main()
