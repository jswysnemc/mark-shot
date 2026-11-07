"""【OCR 翻译输入测试】【实际证据】读取平台状态、候选字形、原生弹层和确认正文，不读取用户桌面"""

import io
import json
import re
import struct
import time

from PIL import Image
import wayland_text_format_session as session
from wayland_ocr_translation_checks import point
from wayland_text_pointer import click
from wayland_window_input_session import candidate_pixels, canceled_candidate_pixels, cursor_rect, newer_state, presented_candidate, screenshot


def result_frame():
    """【OCR 翻译输入测试】【相同文档】返回和原版窗口相同的 base、空位置记录及透明二乘二图像，不初始化模型，无参数"""
    image = io.BytesIO()
    Image.new("RGBA", (2, 2)).save(image, format="PNG")
    png = image.getvalue()
    header = json.dumps({"text": "base", "lines": []}).encode()
    return b"MSOCR001" + struct.pack("<II", len(header), len(png)) + header + png


def popup_roles(text):
    """【OCR 翻译输入测试】【弹层生命周期】解析 text 中 Qt 和 Rust 的真实创建、配置与销毁，返回存活编号到配置资格的映射，不把接口广播当作实例"""
    live = {}
    for line in text.splitlines():
        created = re.search(r"\.get_popup\((?:new id )?xdg_popup[@#](\d+)", line)
        configured = re.search(r"xdg_popup[@#](\d+)\.configure(?:, \(|\()", line)
        destroyed = re.search(r"xdg_popup[@#](\d+)\.destroy\(\)", line)
        if created:
            live[int(created[1])] = False
        if configured and int(configured[1]) in live:
            live[int(configured[1])] = True
        if destroyed:
            live.pop(int(destroyed[1]), None)
    return live


def batch_delivery(text, serial):
    """【OCR 翻译输入测试】【批次证据】按 text、serial 返回是否交付及交付后是否提交上下文，拒绝相近序号和交付前的无关提交"""
    delivered = re.search(rf"zwp_text_input_v3[@#]\d+\.done(?:, \(|\(){int(serial)}\)", text)
    committed = bool(delivered and re.search(r"zwp_text_input_v3[@#]\d+\.commit\(\)", text[delivered.end():]))
    return bool(delivered), committed


def text_input_serial(text):
    """【OCR 翻译输入测试】【独立计数】从 text 返回当前 text-input-v3 实例的提交计数，不能套用 input-method-v2 的独立 done 序号"""
    created = list(re.finditer(r"\.get_text_input\((?:new id )?zwp_text_input_v3[@#](\d+)", text))
    if not created:
        raise AssertionError("no actual text-input-v3 instance was created")
    current = created[-1]
    return len(re.findall(rf"zwp_text_input_v3[@#]{current[1]}\.commit\(\)", text[current.end():]))


def selection_utf16(text, cursor, anchor):
    """【OCR 翻译输入测试】【选区编码】把 text 的 UTF-8 cursor、anchor 转为 Qt UTF-16 位置，返回二项列表，非法边界报错而不截断"""
    encoded = text.encode()
    if not all(0 <= index <= len(encoded) for index in (cursor, anchor)):
        raise ValueError("selection exceeds actual surrounding text")
    return [len(encoded[:index].decode().encode("utf-16-le")) // 2 for index in (cursor, anchor)]


def popup(log, count):
    """【OCR 翻译输入测试】【两端弹层】等待 log 中 count 个原生角色完成配置，返回存活编号，兼容 Qt 和 Rust 的真实协议日志"""
    deadline = time.monotonic() + 3
    while time.monotonic() < deadline:
        live = popup_roles(log.read_text())
        if len(live) == count and all(live.values()):
            return list(live)
        time.sleep(0.02)
    raise AssertionError(f"expected {count} actual popup roles, found {live}")


class Checks:
    """【OCR 翻译输入测试】【字段证据】共享真实输入客户端，分别保存原版观察和原生断言，不以内部状态替代像素"""

    def __init__(self, method, window, root, log, environment, reference=False, expected=None):
        """【OCR 翻译输入测试】【状态构造】保存 method、window、root、log、environment，reference 标记原版观察，expected 为冻结平台结果，无返回值"""
        self.method, self.window, self.root, self.log = method, window, root, log
        self.environment, self.reference, self.expected = environment, reference, expected
        self.states, self.clipboard = {}, {}
        self.field, self.protocol_delays = None, []

    def checkpoint(self, name):
        """【OCR 翻译输入测试】【逐步状态】在 name 保存实际客户端状态及原版控件查询，返回记录，不构造预期正文作为证据"""
        value = {"protocol": self.method.states[-1].copy() if self.method.states else None}
        if self.reference:
            deadline = time.monotonic() + 2
            while time.monotonic() < deadline:
                try:
                    value["qt"] = json.loads((self.root / "reference-state.json").read_text())
                    break
                except (FileNotFoundError, json.JSONDecodeError):
                    time.sleep(0.01)
            assert "qt" in value, "Qt did not publish its actual editor state"
        self.states[name] = value
        directory = self.root / "checkpoints"
        directory.mkdir(exist_ok=True)
        (directory / (name + ".json")).write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")
        return value

    def context(self, text):
        """【OCR 翻译输入测试】【上下文核对】等待 text 的实际平台上下文，返回真实客户端状态；原版缓存缺失只用已聚焦控件查询确认，缺失信息单独记录"""
        if not self.reference:
            return self.method.state(text)
        deadline = time.monotonic() + 2
        names = {"source": "ocrEditor", "translation": "ocrTranslationEditor", "language": "ocrLanguageCombo"}
        while time.monotonic() < deadline:
            self.method.read(0.02)
            while self.method.read(0) is not None:
                pass
            state = self.method.states[-1]
            value = self.checkpoint("last-context")["qt"]
            observed = value[self.field]
            if state["active"] and observed["visible"] and observed["input_enabled"] and observed["text"] == text and value["focus"] == names[self.field]:
                if state["text"] != text:
                    self.protocol_delays.append({"kind": "context_missing_after_activation", "field": self.field, "protocol": state.copy(), "queried_qt": observed})
                return state
        raise AssertionError(f"Qt did not activate the requested editor context: {self.method.states[-1]}")

    def focus(self, field, text, end=True):
        """【OCR 翻译输入测试】【实际焦点】点击 field 并等待 text 的原生上下文，end 选择移动到末尾，返回当前协议状态"""
        click(point(self.window, field))
        self.field = field
        try:
            self.context(text)
        except AssertionError:
            self.checkpoint("focus-" + field + "-failed")
            raise
        if end:
            session.key("End" if field == "language" else "ctrl+End")
            self.at_end(text)
        return self.method.states[-1]

    def select_all(self, text):
        """【OCR 翻译输入测试】【选区同步】全文选择 text 后等待实际选区完成，返回空值，避免后续输入法批次早于快捷键处理"""
        session.key("ctrl+a")
        deadline = time.monotonic() + 2
        while time.monotonic() < deadline:
            while self.method.read(0.02) is not None:
                pass
            state = self.checkpoint("select-all")["qt"][self.field] if self.reference else self.method.states[-1]
            length = len(text.encode("utf-16-le")) // 2 if self.reference else len(text.encode())
            if state["text"] == text and sorted([state["cursor"], state["anchor"]]) == [0, length]:
                return
        raise AssertionError(f"editor did not select its actual confirmed text: {state}")

    def at_end(self, text):
        """【OCR 翻译输入测试】【末尾同步】等待 text 的实际光标和锚点到达末尾，原版读取控件 UTF-16，原生读取协议 UTF-8，无返回值"""
        deadline = time.monotonic() + 2
        while time.monotonic() < deadline:
            while self.method.read(0.02) is not None:
                pass
            state = self.checkpoint("end-selection")["qt"][self.field] if self.reference else self.method.states[-1]
            length = len(text.encode("utf-16-le")) // 2 if self.reference else len(text.encode())
            if state["text"] == text and state["cursor"] == state["anchor"] == length:
                return
        raise AssertionError(f"editor did not reach its actual end: {state}")

    def batch(self, text, allow_no_update=False, **arguments):
        """【OCR 翻译输入测试】【原子确认】向 method 发送 arguments，text 核对正文，allow_no_update 只允许空候选清理，返回实际状态；原版延迟提交单独记录"""
        previous = self.method.states[-1]["serial"]
        initial_wire = self.log.read_text()
        offset = len(initial_wire)
        input_serial = text_input_serial(initial_wire)
        sent = int(self.method.batch(**arguments).split("\t")[1])
        if not self.reference and not allow_no_update:
            return newer_state(self.method, previous, text)
        deadline = time.monotonic() + 2
        while time.monotonic() < deadline:
            self.method.read(0.03)
            while self.method.read(0) is not None:
                pass
            state = self.method.states[-1]
            if state["serial"] > previous and state["active"] and state["text"] == text:
                assert state["cursor"] == state["anchor"] == len(text.encode())
                return state
            wire = self.log.read_text()[offset:]
            delivered, committed = batch_delivery(wire, input_serial)
            if self.reference and delivered:
                value = self.checkpoint("last-batch")
                observed = value["qt"][self.field]["text"]
                if observed == text and value["qt"][self.field]["input_enabled"]:
                    assert value["qt"][self.field]["cursor"] == value["qt"][self.field]["anchor"] == len(text.encode("utf-16-le")) // 2
                    if state["text"] != observed:
                        self.protocol_delays.append({"kind": "uncommitted_surrounding_update" if not committed else "context_cache_not_resent", "field": self.field, "sent_serial": sent, "text_input_serial": input_serial, "protocol_text": state["text"], "observed_qt_text": observed})
                    return state
            if allow_no_update and delivered and state["active"] and state["text"] == text:
                return state
        raise AssertionError(f"batch did not update its actual editor: {self.method.states[-1]}")

    def ctrl_y(self, name, text):
        """【OCR 翻译输入测试】【原版别名】发送 Ctrl+Y 并读取所有实际回复，name 保存状态，确认 text 不变，不用旧缓存掩盖错误重做"""
        session.key("ctrl+y")
        while self.method.read(0.1) is not None:
            pass
        assert self.method.states[-1]["text"] == text, "Ctrl+Y unexpectedly redid a Qt multiline edit"
        self.checkpoint(name)

    def full_copy(self, name, text):
        """【OCR 翻译输入测试】【确认复制】当前字段通过原版快捷键全文复制，name 保存真实剪贴板，text 指定要核对的确认正文，无返回值"""
        session.key("ctrl+shift+c")
        session.wait_clipboard(self.environment, text)
        self.clipboard[name] = session.read_clipboard(self.environment)

    def candidate(self, name, text, value):
        """【OCR 翻译输入测试】【候选字形】在 name 保存 text 上的 value 候选真实画面，返回无候选基准、逻辑光标及字形像素数"""
        before = screenshot(self.root, name + "-baseline")
        old = cursor_rect(self.log)
        self.batch(text, preedit=value)
        _, pixels = presented_candidate(self.root, name + "-preedit", before, self.window, old, self.log)
        current = cursor_rect(self.log)
        self.checkpoint(name)
        return {"baseline": before, "old": old, "candidate": current, "pixels": pixels}

    def restored_candidate(self, name, profile):
        """【OCR 翻译输入测试】【恢复观察】按 name 和 profile 检查恢复后的候选字形，返回是否可见；原生结果逐项匹配真实 Qt 观察"""
        after = screenshot(self.root, name)
        pixels = candidate_pixels(profile["baseline"], after, self.window, profile["old"], profile["candidate"])
        visible = pixels > 6
        if not self.reference:
            assert visible == self.expected[name]["candidate_visible"], f"{name} candidate visibility differs from real Qt"
        return {"candidate_visible": visible, "pixels": pixels}

    def menu(self, field, text, name):
        """【OCR 翻译输入测试】【菜单候选】对 field 的 text 执行实际右键菜单及过期批次，name 保存两态基准，返回原版或原生候选观察"""
        self.focus(field, text)
        old = cursor_rect(self.log)
        click(point(self.window, field), "right")
        popup(self.log, 1)
        if not self.reference:
            self.method.state(active=False)
        else:
            while self.method.read(0.03) is not None:
                pass
        time.sleep(0.1)
        inactive = screenshot(self.root, name + "-inactive-baseline")
        session.key("Escape")
        popup(self.log, 0)
        self.context(text)
        time.sleep(0.08)
        restored = self.checkpoint(name + "-selection-restored")
        selection = restored["qt"][field] if self.reference else restored["protocol"]
        restored_selection = [selection["cursor"], selection["anchor"]]
        if not self.reference:
            expected = self.expected[name]["restored_selection_utf16"]
            actual = selection_utf16(text, *restored_selection)
            assert actual == expected, f"{name} restored selection differs from Qt: {actual} != {expected}"
        # 1. 【OCR 翻译输入测试】【独立候选】先记录菜单选区再移动到末尾，避免测试候选意外删除恢复选区
        session.key("End" if field == "language" else "ctrl+End")
        self.at_end(text)
        profile = self.candidate(name, text, "menu-candidate") if field != "language" else None
        previous = self.method.states[-1]["serial"]
        click(point(self.window, field), "right")
        popup(self.log, 1)
        if not self.reference:
            newer_state(self.method, previous, None, active=False)
        else:
            while self.method.read(0.03) is not None:
                pass
        time.sleep(0.1)
        after = screenshot(self.root, name + "-open")
        pixels = canceled_candidate_pixels(inactive, after, self.window, old, profile["candidate"]) if profile else 0
        state = self.checkpoint(name + "-open")
        if self.reference:
            after_text = state["qt"][field]["text"]
        else:
            after_text = text
            assert (pixels > 6) == self.expected[name]["candidate_or_committed_glyphs_visible"], f"{name} menu glyphs differ from Qt"
        self.method.batch(commit="stale-menu")
        session.key("Escape")
        popup(self.log, 0)
        time.sleep(0.08)
        while self.method.read(0.03) is not None:
            pass
        final = self.checkpoint(name + "-closed")
        ignored = True
        if self.reference:
            final_text = final["qt"][field]["text"]
            ignored = final_text == after_text
            after_text = final_text
        self.context(after_text)
        if not self.reference:
            if field != "language":
                self.full_copy(name + "-stale-commit-rejected", text)
            assert not state["protocol"]["active"]
        return {"after_text": after_text, "candidate_tested": profile is not None, "candidate_or_committed_glyphs_visible": pixels > 6, "pixels": pixels, "stale_commit_ignored": ignored, "input_active_while_menu": state["protocol"]["active"], "restored_selection_utf16": restored_selection if self.reference else actual}
