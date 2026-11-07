"""【文字会话测试】【隔离选择】验证后端记录、进程身份和测试键盘生命周期，不连接真实桌面"""

import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock, patch

import wayland_text_format_session as session
import wayland_text_sway_session as sway
import wayland_sway_keyboard as keyboard


class FormatSessionChecks(unittest.TestCase):
    """【文字会话测试】【后端边界】私有运行记录决定输入后端，外部环境不能替换本次会话"""

    def test_private_sway_record_selects_sway_adapter(self):
        """【文字会话测试】【Sway 选择】本次记录标识 Sway 时只使用 Sway 校验器，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory) / "runtime.json").write_text(json.dumps({"compositor": "sway"}))
            with patch.dict(os.environ, XDG_RUNTIME_DIR=directory), patch.object(sway, "environment", return_value={"owned": "sway"}) as adapter:
                self.assertEqual(session.environment(), {"owned": "sway"})
                adapter.assert_called_once_with()

    def test_external_sway_socket_does_not_replace_niri_record(self):
        """【文字会话测试】【来源保护】外部 SWAYSOCK 不覆盖私有 niri 记录，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory) / "runtime.json").write_text(json.dumps({"outputs": {}}))
            with patch.dict(os.environ, XDG_RUNTIME_DIR=directory, SWAYSOCK="/external/socket"), patch.object(session.niri, "environment", return_value={"owned": "niri"}), patch.object(sway, "environment") as other:
                self.assertEqual(session.environment(), {"owned": "niri"})
                other.assert_not_called()

    def test_invalid_sway_process_identity_is_rejected(self):
        """【文字会话测试】【进程保护】系统初始进程不能成为测试目标，校验失败不访问输入服务，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory) / "runtime.json").write_text(json.dumps({"compositor": "sway", "sway_pid": 1}))
            with patch.dict(os.environ, XDG_RUNTIME_DIR=directory), patch.object(sway.os, "kill") as kill:
                with self.assertRaisesRegex(RuntimeError, "verified private runtime"):
                    sway.environment()
                kill.assert_not_called()

    def test_sway_socket_outside_runtime_is_rejected(self):
        """【文字会话测试】【套接字保护】记录中的套接字不在本次目录时拒绝输入，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory) / "runtime.json").write_text(json.dumps({"compositor": "sway", "sway_pid": 42, "sway_socket": "/external/socket"}))
            with patch.dict(os.environ, XDG_RUNTIME_DIR=directory, SWAYSOCK="/external/socket"), patch.object(sway.os, "kill") as kill:
                with self.assertRaisesRegex(RuntimeError, "private Sway socket"):
                    sway.environment()
                kill.assert_not_called()

    def test_persistent_keyboard_is_started_once_and_reclaimed(self):
        """【文字会话测试】【键盘释放】多次输入共用本次设备，退出时终止并回收，不保留等待进程，无参数、无返回值"""
        process = Mock()
        process.poll.return_value = None
        with patch.object(sway, "environment", return_value={"owned": "sway"}), patch.object(keyboard, "build", return_value=Path("/owned/keyboard")), patch.object(keyboard.Keyboard, "wait") as acknowledge, patch.object(keyboard.subprocess, "Popen", return_value=process) as start, patch.object(sway.subprocess, "run") as run, patch.object(sway.time, "sleep"):
            with sway.input_context():
                sway.key("ctrl+z")
                sway.key("ctrl+shift+z")
            start.assert_called_once()
            self.assertEqual(start.call_args.args[0], ["/owned/keyboard"])
            self.assertEqual([call.args[0] for call in process.stdin.write.call_args_list], [b"44 1\n", b"44 3\n"])
            self.assertEqual([call.args[0] for call in acknowledge.call_args_list], ["ready", "done", "done"])
            run.assert_not_called()
            process.stdin.close.assert_called_once()
            process.stdout.close.assert_called_once()
            process.terminate.assert_not_called()
            process.wait.assert_called_once_with(timeout=3)
            self.assertIsNone(sway.KEYBOARD.get())

    def test_keyboard_shutdown_timeout_only_kills_its_owned_process(self):
        """【文字会话测试】【回收超时】本次键盘拒绝正常退出时只强制回收该对象，不保留上下文，无参数、无返回值"""
        process = Mock()
        process.poll.return_value = None
        process.wait.side_effect = [sway.subprocess.TimeoutExpired("private-keyboard", 3), None]
        with patch.object(sway, "environment", return_value={"owned": "sway"}), patch.object(keyboard, "build", return_value=Path("/owned/keyboard")), patch.object(keyboard.Keyboard, "wait"), patch.object(keyboard.subprocess, "Popen", return_value=process), patch.object(sway.time, "sleep"):
            with sway.input_context():
                sway.keyboard_ready()
            process.kill.assert_called_once()
            self.assertEqual(process.wait.call_count, 2)
            process.stdin.close.assert_called_once()
            process.stdout.close.assert_called_once()
            self.assertIsNone(sway.KEYBOARD.get())


if __name__ == "__main__":
    unittest.main()
