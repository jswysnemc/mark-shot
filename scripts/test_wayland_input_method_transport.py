"""【输入法测试】【证据解析】协议广告不能当作实际原生投递，Rust 请求与事件格式分别识别"""

import unittest

from wayland_input_method_transport import InputMethod, protocol_evidence


class ProtocolEvidenceTests(unittest.TestCase):
    """【输入法测试】【真实格式】验证 Wayland 客户端请求与分派日志，不连接桌面"""

    def test_rust_dispatched_events_are_not_misparsed_as_requests(self):
        """【输入法测试】【Rust 事件】实际事件使用逗号分隔，返回投递证据而非假阴性，无参数"""
        text = """[1][rs] -> zwp_text_input_manager_v3@17.get_text_input(zwp_text_input_v3@19, wl_seat@4)
[2][rs] -> zwp_text_input_v3@19.enable()
[3][rs] -> zwp_text_input_v3@19.set_surrounding_text(Some(""), 0, 0)
[4][rs] -> zwp_text_input_v3@19.commit()
[5][rs] <- zwp_text_input_v3@19.preedit_string, (Some("ni"), 2, 2)
[6][rs] <- zwp_text_input_v3@19.done, (1)
"""
        self.assertEqual(
            protocol_evidence(text),
            {"native_protocol": True, "preedit_delivered": True, "protocol_commits": 1},
        )

    def test_protocol_advertisement_alone_does_not_prove_binding(self):
        """【输入法测试】【广告排除】只含注册表全局名称不得宣称原生输入可用，无参数、无返回值"""
        text = 'wl_registry@2.global, (15, "zwp_text_input_manager_v3", 1)'
        self.assertEqual(
            protocol_evidence(text),
            {
                "native_protocol": False,
                "preedit_delivered": False,
                "protocol_commits": 0,
            },
        )

    def test_record_preserves_delivered_field_type_and_utf8_selection(self):
        """【输入法测试】【字段记录】用途与状态按交付顺序关联，字节光标不转换为字符数，无参数、无返回值"""
        method = InputMethod.__new__(InputMethod)
        method.records, method.states = [], []
        method.content = {"hint": 0, "purpose": 0}
        method.record("content\t0\t2")
        method.record("state\t7\t1\t2\t0\t3230207074")
        method.record("content\t3\t0")
        method.record("state\t8\t1\t3\t0\te4bda0")
        self.assertEqual(method.states[0], {"serial": 7, "active": True, "cursor": 2, "anchor": 0, "text": "20 pt", "hint": 0, "purpose": 2})
        self.assertEqual(method.states[1]["text"], "你")
        self.assertEqual(method.states[1]["cursor"], 3)
        self.assertEqual(method.states[1]["purpose"], 0)


if __name__ == "__main__":
    unittest.main()
