"""【OCR 置顶测试】【协议证据】读取实际创建、销毁、配置和边距，不以按钮标志代替真实置顶"""

import re
import time
import tomllib


def roles(text):
    """【OCR 置顶测试】【角色解析】从 text 返回存活的普通窗口和浮层，编号复用必须重新配置，不把接口广播当作角色"""
    live = {}
    surfaces = {}
    for line in text.splitlines():
        created = re.search(r"\.get_xdg_surface\((?:new id )?xdg_surface[@#](\d+), wl_surface[@#](\d+)\)", line)
        if created:
            surfaces[int(created[1])] = int(created[2])
        for kind, pattern in [("window", r"\.get_toplevel\((?:new id )?xdg_toplevel[@#](\d+)"), ("layer", r"\.get_layer_surface\((?:new id )?zwlr_layer_surface_v1[@#](\d+)")]:
            match = re.search(pattern, line)
            if match:
                value = {"kind": kind, "id": int(match[1]), "configured": False, "drawn": False, "keyboard_focused": False}
                parent = re.search(r"xdg_surface[@#](\d+)\.get_toplevel", line)
                layer = re.search(r"\.get_layer_surface\((?:new id )?zwlr_layer_surface_v1[@#]\d+, wl_surface[@#](\d+)", line)
                value["surface"] = surfaces.get(int(parent[1])) if parent else int(layer[1]) if layer else None
                level = re.search(r"\.get_layer_surface\(.*?, (?:None|nil|<anonymous>[@#]0|wl_output[@#]\d+|Some\(wl_output[@#]\d+\)), (\d+),", line)
                if level:
                    value["level"] = int(level[1])
                live[(kind, int(match[1]))] = value
        for kind, protocol in [("window", "xdg_toplevel"), ("layer", "zwlr_layer_surface_v1")]:
            match = re.search(rf"{protocol}[@#](\d+)\.destroy\(\)", line)
            if match:
                live.pop((kind, int(match[1])), None)
        match = re.search(r"zwlr_layer_surface_v1[@#](\d+)\.configure(?:, \(|\()\d+, (\d+), (\d+)\)", line)
        if match and ("layer", int(match[1])) in live:
            live[("layer", int(match[1]))].update(configured=True, size=[int(match[2]), int(match[3])])
        match = re.search(r"zwlr_layer_surface_v1[@#](\d+)\.set_margin\((-?\d+), -?\d+, -?\d+, (-?\d+)\)", line)
        if match and ("layer", int(match[1])) in live:
            live[("layer", int(match[1]))]["position"] = [int(match[3]), int(match[2])]
        configured = re.search(r"xdg_toplevel[@#](\d+)\.configure(?:, \(|\()", line)
        if configured and ("window", int(configured[1])) in live:
            live[("window", int(configured[1]))]["configured"] = True
        attached = re.search(r"wl_surface[@#](\d+)\.attach\((?:Some\()?wl_buffer[@#]", line)
        entered = re.search(r"wl_keyboard[@#]\d+\.enter(?:, \(|\()\d+, wl_surface[@#](\d+)", line)
        left = re.search(r"wl_keyboard[@#]\d+\.leave(?:, \(|\()\d+, (?:Some\()?wl_surface[@#](\d+)", line)
        level = re.search(r"zwlr_layer_surface_v1[@#](\d+)\.set_layer\((\d+)\)", line)
        if level and ("layer", int(level[1])) in live:
            live[("layer", int(level[1]))]["level"] = int(level[2])
        for value in live.values():
            if attached and value["surface"] == int(attached[1]):
                value["drawn"] = True
            if entered and value["surface"] == int(entered[1]):
                value["keyboard_focused"] = True
            if left and value["surface"] == int(left[1]):
                value["keyboard_focused"] = False
    return list(live.values())


def wait_role(log, top):
    """【OCR 置顶测试】【实际切换】等待 log 只保留 top 对应主角色，返回存活角色，不能用尚未销毁的旧窗口认定成功"""
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        current = roles(log.read_text())
        if len(current) == 1 and current[0]["kind"] == ("layer" if top else "window") and current[0]["configured"] and current[0]["drawn"] and (current[0]["keyboard_focused"] if not top else "position" in current[0] and current[0].get("level") == 2):
            return current[0]
        time.sleep(0.03)
    raise AssertionError(f"expected one actual {'layer' if top else 'xdg'} role: {current}")


def layer_window(log):
    """【OCR 置顶测试】【真实浮层位置】从 log 读取单输出会话的实际配置及最后边距，返回逻辑窗口矩形，不通过窗口管理器强制调整尺寸"""
    state = wait_role(log, True)
    return {"x": state["position"][0], "y": state["position"][1], "width": state["size"][0], "height": state["size"][1]}


def wait_config(config, top):
    """【OCR 置顶测试】【实际保存】等待 config 中独立开关成为 top，返回完整 TOML；贴图开关必须一直保持 true"""
    deadline = time.monotonic() + 4
    while time.monotonic() < deadline:
        value = tomllib.loads(config.read_text())
        if value.get("ocr_result_window", {}).get("always_on_top") is top:
            assert value["pinned"]["always_on_top"] is True
            return value
        time.sleep(0.02)
    raise AssertionError(f"independent OCR stacking was not saved: {value}")
