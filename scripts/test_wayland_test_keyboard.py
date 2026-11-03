"""【测试键盘】【回归】验证固定键码、私有会话拒绝规则和输入目标，不连接显示服务器"""

import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import wayland_test_keyboard as keyboard


class KeyboardTests(unittest.TestCase):
    """【测试键盘】【用例集合】所有外部输入命令均使用替代实现，真实桌面保持不变"""

    def test_letter_shortcuts_do_not_implicitly_hold_shift(self):
        """【键盘测试】【单字母】大写工具名仍发送未修饰字母，显式 Shift 则保持修饰"""
        for value in "ABCDEFGHIJKLMNOPQRSTUVWXYZ":
            self.assertEqual(keyboard.canonical_chord(value), value.lower())
        self.assertEqual(keyboard.canonical_chord("Shift+N"), "shift+n")

    def test_modifiers_and_special_keys_are_normalized(self):
        """【键盘测试】【命名按键】验证修饰别名、回车、翻页和功能键的固定映射"""
        for value, expected in {
            " ctrl + SHIFT + Z ": "ctrl+shift+z", "Control+Enter": "ctrl+Return",
            "Super+Tab": "super+Tab", "Meta+F24": "super+F24", "Esc": "Escape",
            "Page_Up": "Prior", "PageDown": "Next", "Backspace": "BackSpace",
            "Ctrl+Ctrl+S": "ctrl+s", "[": "[", "bracketright": "bracketright",
        }.items():
            self.assertEqual(keyboard.canonical_chord(value), expected)

    def test_invalid_chords_are_rejected_before_input(self):
        """【键盘测试】【非法组合】空组合、未知修饰和非法功能键在执行外部命令前失败"""
        for value in ["", "Ctrl+", "+N", "Ctrl++N", "Hyper+N", "F0", "F25", "未知", "!"]:
            with self.subTest(value=value), self.assertRaises(ValueError):
                keyboard.canonical_chord(value)

    def fixture(self, root):
        """【键盘测试】【私有记录】在 root 建立替代运行记录；返回仅供测试使用的环境，无进程启动"""
        socket = root / "niri.wayland-1.456.sock"
        socket.touch()
        record = {
            "x11_display": ":321", "wayland_display": "wayland-1", "niri_socket": str(socket),
            "xvfb_pid": 123, "niri_pid": 456,
        }
        (root / "runtime.json").write_text(json.dumps(record))
        return {
            "XDG_RUNTIME_DIR": str(root), "DISPLAY": ":321",
            "MARK_SHOT_TEST_X11_DISPLAY": ":321", "WAYLAND_DISPLAY": "wayland-1",
        }

    def test_unmarked_or_mismatched_display_cannot_receive_input(self):
        """【键盘测试】【显示器保护】缺少独立标记或显示器不匹配时禁止任何外部输入命令"""
        with patch.object(keyboard.subprocess, "run") as run:
            for environment in [{}, {"DISPLAY": ":0", "MARK_SHOT_TEST_X11_DISPLAY": ":321"}]:
                with patch.dict(os.environ, environment, clear=True), self.assertRaises(RuntimeError):
                    keyboard.send("Return")
            run.assert_not_called()

    def test_runtime_record_and_wayland_display_must_match(self):
        """【键盘测试】【运行记录】X11 标记、Wayland 显示器和私有套接字必须属于同一运行器"""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            environment = self.fixture(root)
            for key, value in [("DISPLAY", ":0"), ("WAYLAND_DISPLAY", "wayland-2"), ("MARK_SHOT_TEST_X11_DISPLAY", ":0")]:
                with patch.dict(os.environ, environment | {key: value}, clear=True), self.assertRaises(RuntimeError):
                    keyboard.runtime_environment()
            (root / "niri.wayland-1.456.sock").unlink()
            with patch.dict(os.environ, environment, clear=True), self.assertRaises(RuntimeError):
                keyboard.runtime_environment()

    def test_dead_runtime_process_is_rejected(self):
        """【键盘测试】【进程存活】私有显示服务器已经退出时不得把旧记录用于输入"""
        with tempfile.TemporaryDirectory() as directory:
            environment = self.fixture(Path(directory))
            with patch.dict(os.environ, environment, clear=True), patch.object(keyboard.os, "kill", side_effect=ProcessLookupError), self.assertRaises(ProcessLookupError):
                keyboard.runtime_environment()

    def test_valid_runtime_checks_both_processes(self):
        """【键盘测试】【身份确认】正常私有记录必须同时确认显示服务器和合成器仍然存活"""
        with tempfile.TemporaryDirectory() as directory:
            environment = self.fixture(Path(directory))
            with patch.dict(os.environ, environment, clear=True), patch.object(keyboard.os, "kill") as check:
                actual, pid = keyboard.runtime_environment()
            self.assertEqual(actual, environment)
            self.assertEqual(pid, 456)
            self.assertEqual([call.args for call in check.call_args_list], [(123, 0), (456, 0)])

    def test_ambiguous_windows_are_rejected_before_focusing(self):
        """【键盘测试】【窗口歧义】没有唯一合成器窗口时不设置焦点，也不发送按键"""
        with patch.object(keyboard, "runtime_environment", return_value=({"DISPLAY": ":321"}, 456)), patch.object(keyboard.subprocess, "check_output") as search, patch.object(keyboard.subprocess, "run") as run:
            for result in ["", "1\n2\n", "not-a-window\n", "0\n"]:
                search.return_value = result
                with self.assertRaises(RuntimeError):
                    keyboard.send("Return")
            run.assert_not_called()

    def test_input_targets_only_the_verified_compositor(self):
        """【键盘测试】【输入分派】先聚焦私有窗口，再用相同私有环境发送规范快捷键"""
        environment = {"DISPLAY": ":321"}
        with patch.object(keyboard, "runtime_environment", return_value=(environment, 456)), patch.object(keyboard.subprocess, "check_output", return_value="987\n") as search, patch.object(keyboard.subprocess, "run") as run:
            self.assertEqual(keyboard.send("Ctrl+Shift+N"), "ctrl+shift+n")
        self.assertEqual(search.call_args.args[0], ["xdotool", "search", "--pid", "456"])
        self.assertEqual([call.args[0] for call in run.call_args_list], [
            ["xdotool", "windowfocus", "--sync", "987"],
            ["xdotool", "key", "--clearmodifiers", "ctrl+shift+n"],
        ])
        for call in [search.call_args, *run.call_args_list]:
            self.assertEqual(call.kwargs["env"], environment)

    def test_held_modifiers_use_the_verified_environment_and_are_released(self):
        """【键盘测试】【保持释放】修饰键只在已校验环境中按下和释放，鼠标操作异常不留下按键状态"""
        environment = {"DISPLAY": ":321"}
        with patch.object(keyboard, "focused_environment", return_value=environment), patch.object(keyboard.subprocess, "run") as run:
            with self.assertRaisesRegex(RuntimeError, "drag failed"):
                with keyboard.hold("Control") as modifier:
                    self.assertEqual(modifier, "ctrl")
                    raise RuntimeError("drag failed")
        self.assertEqual([call.args[0] for call in run.call_args_list], [
            ["xdotool", "keydown", "ctrl"], ["xdotool", "keyup", "ctrl"],
        ])
        self.assertTrue(all(call.kwargs["env"] == environment for call in run.call_args_list))

    def test_invalid_or_unverified_modifier_holds_never_send_input(self):
        """【键盘测试】【保持保护】未知修饰键或私有校验失败时不执行任何输入，也不进入保持作用域"""
        with patch.object(keyboard, "focused_environment", side_effect=RuntimeError("not private")), patch.object(keyboard.subprocess, "run") as run:
            with self.assertRaises(ValueError):
                with keyboard.hold("Ctrl+A"):
                    self.fail("an invalid modifier hold was entered")
            with self.assertRaises(RuntimeError):
                with keyboard.hold("Shift"):
                    self.fail("an unverified modifier hold was entered")
            run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
