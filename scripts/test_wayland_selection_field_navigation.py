"""【选区字段测试】【协议消费】已到达的状态必须进入不变断言，不能只观察旧缓存"""

import importlib.util
import os
from pathlib import Path
from types import SimpleNamespace
import unittest

from wayland_input_method_transport import InputMethod

spec = importlib.util.spec_from_file_location("field_navigation_verifier", Path(__file__).with_name("wayland-selection-field-navigation.py"))
NAV = importlib.util.module_from_spec(spec)
spec.loader.exec_module(NAV)


class UnchangedStateTests(unittest.TestCase):
    """【选区字段测试】【真实缓冲】使用客户端解析器和真实空管道复现已经缓冲但尚未消费的协议状态"""

    def setUp(self):
        """【选区字段测试】【连接替身】建立保持打开的空管道与真实解析器，返回空值，不启动显示服务器"""
        read, self.write = os.pipe()
        self.stream = os.fdopen(read, "rb", buffering=0)
        self.method = InputMethod.__new__(InputMethod)
        self.method.process = SimpleNamespace(stdout=self.stream, poll=lambda: None)
        self.method.records, self.method.states, self.method.buffer = [], [], b""
        self.method.content = {"hint": 7, "purpose": 0}
        self.method.record("state\t1\t1\t5\t5\t2b31323030")

    def tearDown(self):
        """【选区字段测试】【管道释放】关闭本次读写端，返回空值，不影响真实客户端"""
        self.stream.close()
        os.close(self.write)

    def test_pending_selection_change_rejects_unchanged_assertion(self):
        """【选区字段测试】【假通过拒绝】缓存之后到达的双击选择必须使不变断言失败，无参数、无返回值"""
        before = NAV.context(self.method)
        self.method.buffer = b"state\t2\t1\t5\t1\t2b31323030\n"
        with self.assertRaisesRegex(AssertionError, "field changed"):
            NAV.unchanged(self.method, before)

    def test_same_context_is_consumed_before_accepting_unchanged(self):
        """【选区字段测试】【同值确认】同值的新协议序号仍必须消费，消费后不改变字段上下文，无参数、无返回值"""
        before = NAV.context(self.method)
        self.method.buffer = b"state\t2\t1\t5\t5\t2b31323030\n"
        self.assertEqual(NAV.unchanged(self.method, before), before)
        self.assertEqual(self.method.states[-1]["serial"], 2)
        self.assertEqual(self.method.buffer, b"")


if __name__ == "__main__":
    unittest.main()
