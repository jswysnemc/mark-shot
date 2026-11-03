"""【输入法测试】【私有焦点】固定测试合成器客户端，不使用用户 PATH 中的兼容脚本"""

import importlib.util
import json
import unittest
from pathlib import Path
from unittest.mock import Mock, patch

SPEC = importlib.util.spec_from_file_location(
    "native_input_method_regression",
    Path(__file__).with_name("wayland-input-method.py"),
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class PrivateFocusTests(unittest.TestCase):
    """【输入法测试】【焦点命令】核验私有环境、唯一窗口身份和不改变窗口尺寸的工作区切换"""

    def invoke(self, shown, duplicate=False):
        """【输入法测试】【命令收集】按 shown 和 duplicate 构造树并执行焦点切换，返回客户端调用；身份重复时抛出断言"""
        window = {"app_id": "mark-shot", "id": 5}
        tree = {
            "nodes": [
                {
                    "type": "workspace",
                    "name": "1",
                    "nodes": [window, window] if duplicate else [window],
                }
            ]
        }
        process = Mock()
        process.poll.return_value = None
        with (
            patch.object(
                MODULE.session.sway,
                "environment",
                return_value={"SWAYSOCK": "/private/test.sock"},
            ),
            patch.object(
                MODULE.subprocess,
                "check_output",
                side_effect=[json.dumps(tree), '[{"success": true}]'],
            ) as output,
        ):
            MODULE.focus(process, shown)
            return output.call_args_list

    def test_focus_away_uses_the_test_sway_client_and_private_workspace(self):
        """【输入法测试】【固定客户端】失焦命令只使用测试工具路径和已校验环境，不依赖用户兼容脚本，无参数、无返回值"""
        calls = self.invoke(False)
        expected = str(Path("target/test-tools/sway/usr/bin/swaymsg").resolve())
        self.assertEqual(calls[0].args[0], [expected, "-r", "-t", "get_tree"])
        self.assertEqual(
            calls[1].args[0],
            [
                expected,
                "-r",
                'workspace --no-auto-back-and-forth "__mark_shot_ime_focus_test__"',
            ],
        )
        self.assertEqual(calls[1].kwargs["env"], {"SWAYSOCK": "/private/test.sock"})

    def test_focus_return_uses_the_actual_editor_workspace(self):
        """【输入法测试】【原工作区恢复】恢复命令采用当前窗口树中的工作区，不移动或缩放窗口，无参数、无返回值"""
        self.assertEqual(
            self.invoke(True)[1].args[0][-1], 'workspace --no-auto-back-and-forth "1"'
        )

    def test_ambiguous_editor_identity_rejects_focus_commands(self):
        """【输入法测试】【重复拒绝】同名窗口不唯一时拒绝焦点操作，无参数、无返回值"""
        with self.assertRaisesRegex(AssertionError, "ambiguous"):
            self.invoke(False, duplicate=True)


if __name__ == "__main__":
    unittest.main()
