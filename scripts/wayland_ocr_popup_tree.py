"""【更多菜单测试】【协议父链】核对真实表面、活动抓取和逆序销毁，不以业务状态代替协议证据"""

import re


def popup_tree(text, closed=False):
    """【更多菜单测试】【父链解析】按 text 返回历次节点，closed 要求全部关闭；拒绝兄弟角色、复用根抓取和逆序错误"""
    records, live, surfaces, serials, used = [], [], {}, set(), set()
    for line in text.splitlines():
        event = re.search(r"wl_(?:pointer|keyboard)@\d+\.(?:button|key), \((\d+), \d+, \d+, 1\)", line)
        if event:
            serials.add(int(event[1]))
        surface = re.search(r"get_xdg_surface\(xdg_surface@(\d+), wl_surface@(\d+)\)", line)
        if surface:
            surfaces[int(surface[1])] = int(surface[2])
        created = re.search(r"xdg_surface@(\d+)\.get_popup\(xdg_popup@(\d+), xdg_surface@(\d+),", line)
        if created:
            xdg, role, parent = map(int, created.groups())
            if any(item["role"] == role for item in live):
                raise AssertionError("a popup role was reused while still alive")
            if live and parent != live[-1]["xdg"]:
                raise AssertionError("submenu is not a child of the deepest live popup")
            if xdg not in surfaces or parent not in surfaces:
                raise AssertionError("popup has no real parent or child wl_surface")
            node = {"role": role, "xdg": xdg, "surface": surfaces[xdg], "parent": parent,
                    "depth": len(live), "grab": None, "rect": None, "frames": 0, "destroyed": False}
            records.append(node)
            live.append(node)
        grab = re.search(r"xdg_popup@(\d+)\.grab\(wl_seat@(\d+), (\d+)\)", line)
        if grab:
            role, seat, serial = map(int, grab.groups())
            if not live or live[-1]["role"] != role or live[-1]["grab"] is not None:
                raise AssertionError("grab does not belong to a new deepest popup")
            if len(live) > 1:
                if live[-2]["grab"] != [seat, serial]:
                    raise AssertionError("submenu did not inherit the live parent grab")
            elif serial not in serials or (seat, serial) in used:
                raise AssertionError("root has no fresh real input grab")
            else:
                used.add((seat, serial))
            live[-1]["grab"] = [seat, serial]
        configured = re.search(r"xdg_popup@(\d+)\.configure(?:, \(|\()(-?\d+), (-?\d+), (\d+), (\d+)\)", line)
        if configured:
            role, *rect = map(int, configured.groups())
            node = next((item for item in live if item["role"] == role), None)
            if node:
                node["rect"] = rect
        attached = re.search(r"wl_surface@(\d+)\.attach\(wl_buffer@", line)
        if attached:
            for node in live:
                if node["surface"] == int(attached[1]):
                    node["frames"] += 1
        destroyed = re.search(r"xdg_popup@(\d+)\.destroy\(\)", line)
        if destroyed:
            if not live or live[-1]["role"] != int(destroyed[1]):
                raise AssertionError("popup ancestor was destroyed before its child")
            live.pop()["destroyed"] = True
    if any(node["grab"] is None for node in records):
        raise AssertionError("popup has no explicit native grab")
    if closed and live:
        raise AssertionError("popup tree retained a live grab after closing")
    return records


def active_tree(text):
    """【更多菜单测试】【活动节点】按 text 返回未销毁节点；解析同时校验完整父链，不复用关闭会话的角色编号"""
    return [node for node in popup_tree(text) if not node["destroyed"]]


def pointer_position(text, surface):
    """【更多菜单测试】【首个移动】按 text 返回当前 surface 的真实指针位置，离开或未送达为空值，不以第二次移动补偿首个事件"""
    focus, position = {}, None
    for line in text.splitlines():
        enter = re.search(r"wl_pointer@(\d+)\.enter, \(\d+, wl_surface@(\d+), (-?[\d.]+), (-?[\d.]+)\)", line)
        if enter:
            pointer, target = map(int, enter.groups()[:2])
            focus[pointer] = target
            if target == surface:
                position = tuple(map(float, enter.groups()[2:]))
        motion = re.search(r"wl_pointer@(\d+)\.motion, \(\d+, (-?[\d.]+), (-?[\d.]+)\)", line)
        if motion and focus.get(int(motion[1])) == surface:
            position = tuple(map(float, motion.groups()[1:]))
        leave = re.search(r"wl_pointer@(\d+)\.leave, \(\d+, wl_surface@(\d+)\)", line)
        if leave:
            focus.pop(int(leave[1]), None)
            if int(leave[2]) == surface:
                position = None
    return position
