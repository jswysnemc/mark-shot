"""【测试会话测试】【剪贴板输入】验证任意文本不会成为命令选项，复制失败或失效会话不发送粘贴键"""

import subprocess
import unittest
from unittest.mock import patch

import wayland_test_session as session


class ClipboardInputTests(unittest.TestCase):
    """【测试会话测试】【输入保护】覆盖负数强度、失败反馈和私有会话校验"""

    def test_negative_text_cannot_become_a_copy_option(self):
        """【测试会话测试】【负数文本】复制负九使用选项终止符，粘贴键在成功复制之后发送"""
        private = {"WAYLAND_DISPLAY": "private-test"}
        completed = subprocess.CompletedProcess([], 0, stdout="")
        with patch.object(session, "environment", return_value=private), patch.object(session.subprocess, "run", return_value=completed) as run, patch.object(session, "key") as key:
            session.paste_text("-9")
            self.assertEqual(run.call_args.args[0], ["wl-copy", "--", "-9"])
            self.assertIs(run.call_args.kwargs["env"], private)
            key.assert_called_once_with("ctrl+v")

    def test_failed_copy_never_sends_the_paste_key(self):
        """【测试会话测试】【复制失败】剪贴板命令失败必须报告错误，不能粘贴之前的内容"""
        completed = subprocess.CompletedProcess([], 1, stdout="")
        with patch.object(session, "environment", return_value={}), patch.object(session.subprocess, "run", return_value=completed), patch.object(session, "key") as key:
            with self.assertRaises(RuntimeError):
                session.paste_text("-9")
            key.assert_not_called()

    def test_invalid_private_session_never_starts_a_copy(self):
        """【测试会话测试】【隔离失效】私有会话校验失败时禁止启动复制进程与发送键盘输入"""
        with patch.object(session, "environment", side_effect=RuntimeError("private session is unavailable")), patch.object(session.subprocess, "run") as run, patch.object(session, "key") as key:
            with self.assertRaises(RuntimeError):
                session.paste_text("text")
            run.assert_not_called()
            key.assert_not_called()


if __name__ == "__main__":
    unittest.main()
