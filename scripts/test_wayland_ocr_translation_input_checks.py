"""【OCR 翻译输入测试】【证据回归】只测试帧、协议日志与选区编码，不启动合成器、不读取用户配置"""

import io
import json
from pathlib import Path
import struct
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

from PIL import Image
from wayland_input_method_transport import protocol_evidence
from wayland_ocr_translation_input_checks import Checks, batch_delivery, popup_roles, result_frame, selection_utf16, text_input_serial


class TranslationInputEvidenceTests(unittest.TestCase):
    """【OCR 翻译输入测试】【严格解析】核对两端协议格式、编号复用和编码边界，错误证据不能伪装为通过"""

    def test_frame_uses_the_production_ocr_document_and_transparent_image(self):
        """【OCR 翻译输入测试】【帧协议】检查实际输入帧的长度、确认正文与透明图像，不请求模型，无参数、无返回值"""
        frame = result_frame()
        self.assertEqual(frame[:8], b"MSOCR001")
        header_size, png_size = struct.unpack("<II", frame[8:16])
        self.assertEqual(len(frame), 16 + header_size + png_size)
        self.assertEqual(json.loads(frame[16:16 + header_size]), {"text": "base", "lines": []})
        image = Image.open(io.BytesIO(frame[16 + header_size:]))
        self.assertEqual(image.size, (2, 2))
        self.assertEqual(image.tobytes(), bytes(16))

    def test_popup_roles_handle_qt_and_rust_creation_configuration_and_reuse(self):
        """【OCR 翻译输入测试】【角色复用】两端实际配置语法都有效，销毁后复用的编号必须重新配置，无参数、无返回值"""
        log = """xdg_surface#35.get_popup(new id xdg_popup#48, xdg_surface#35, xdg_positioner#47)
xdg_popup#48.configure(501, 118, 225, 229)
xdg_surface@35.get_popup(xdg_popup@49, xdg_surface@35, xdg_positioner@47)
xdg_popup@49.configure, (501, 118, 225, 229)
"""
        self.assertEqual(popup_roles(log), {48: True, 49: True})
        log += "xdg_popup#48.destroy()\nxdg_surface#35.get_popup(new id xdg_popup#48, xdg_surface#35, xdg_positioner#47)\n"
        self.assertEqual(popup_roles(log), {49: True, 48: False})

    def test_popup_configuration_without_creation_does_not_prove_a_live_role(self):
        """【OCR 翻译输入测试】【孤立配置】没有实际创建的配置或接口广播不代表存活菜单，无参数、无返回值"""
        self.assertEqual(popup_roles('wl_registry@2.global(5, "xdg_popup", 1)\nxdg_popup@48.configure, (0, 0, 10, 10)'), {})

    def test_batch_delivery_checks_the_exact_serial_and_commits_after_delivery(self):
        """【OCR 翻译输入测试】【精确序号】只识别当前批次交付后的上下文提交，不能把较早提交算入当前批次，无参数、无返回值"""
        for symbol, event in [("#", "done("), ("@", "done, (")]:
            prefix = f"zwp_text_input_v3{symbol}18."
            log = prefix + "commit()\n" + prefix + event + "13)\n"
            self.assertEqual(batch_delivery(log, 3), (False, False))
            self.assertEqual(batch_delivery(log, 13), (True, False))
            self.assertEqual(batch_delivery(log + prefix + "commit()", 13), (True, True))

    def test_utf16_selection_preserves_direction_and_supplementary_characters(self):
        """【OCR 翻译输入测试】【补充平面】辅助平面码点计为两个 UTF-16 单元，反向选区保留方向，无参数、无返回值"""
        self.assertEqual(selection_utf16("a𠮷你", 8, 1), [4, 1])
        self.assertEqual(selection_utf16("a𠮷你", 1, 8), [1, 4])

    def test_text_input_serial_counts_only_the_current_instance_requests(self):
        """【OCR 翻译输入测试】【独立序号】当前实例只计自己的 commit 请求，销毁后创建或复用实例从零开始，无参数、无返回值"""
        log = """zwp_text_input_manager_v3#11.get_text_input(new id zwp_text_input_v3#18, wl_seat#16)
zwp_text_input_v3#18.commit()
zwp_input_method_v2#3.done()
zwp_text_input_v3#18.commit()
"""
        self.assertEqual(text_input_serial(log), 2)
        log += "zwp_text_input_v3#18.destroy()\nzwp_text_input_manager_v3@11.get_text_input(zwp_text_input_v3@18, wl_seat@16)\n"
        self.assertEqual(text_input_serial(log), 0)
        with self.assertRaisesRegex(AssertionError, "no actual text-input-v3"):
            text_input_serial("zwp_input_method_v2#3.done()")

    def test_utf16_selection_rejects_split_codepoints_and_out_of_range_indices(self):
        """【OCR 翻译输入测试】【边界拒绝】码点中间及正文范围外的位置均报错，不静默裁剪，无参数、无返回值"""
        with self.assertRaises(UnicodeDecodeError):
            selection_utf16("𠮷", 2, 0)
        for index in (-1, 5):
            with self.assertRaises(ValueError):
                selection_utf16("𠮷", index, 0)

    def test_protocol_evidence_requires_real_qt_or_rust_requests_and_events(self):
        """【OCR 翻译输入测试】【实际绑定】两种对象语法均需实际绑定、启用、上下文和交付，只有接口广播不能通过，无参数、无返回值"""
        for symbol, event in [("#", "("), ("@", ", (")]:
            log = f"""zwp_text_input_manager_v3{symbol}11.get_text_input(new id zwp_text_input_v3{symbol}18, wl_seat{symbol}16)
zwp_text_input_v3{symbol}18.enable()
zwp_text_input_v3{symbol}18.set_surrounding_text("actual", 6, 6)
zwp_text_input_v3{symbol}18.preedit_string{event}"ni", 2, 2)
zwp_text_input_v3{symbol}18.done{event}13)
zwp_text_input_v3{symbol}18.commit()
"""
            self.assertEqual(protocol_evidence(log), {"native_protocol": True, "preedit_delivered": True, "protocol_commits": 1})
        self.assertFalse(protocol_evidence('wl_registry@2.global(11, "zwp_text_input_manager_v3", 1)')["native_protocol"])

    def test_reference_context_gap_keeps_actual_protocol_separate_from_widget_query(self):
        """【OCR 翻译输入测试】【缓存缺失】只记录聚焦原版控件的实际查询，不能把控件文字写入协议状态，无参数、无返回值"""
        state = {"active": True, "text": "", "cursor": 0, "anchor": 0}
        method = SimpleNamespace(states=[state], read=Mock(return_value=None))
        checks = Checks(method, {}, Path("private"), Path("private/log"), {}, reference=True)
        checks.field = "translation"
        checks.checkpoint = Mock(return_value={"qt": {"focus": "ocrTranslationEditor", "translation": {"visible": True, "input_enabled": True, "text": "actual"}}})
        with patch("wayland_ocr_translation_input_checks.time.monotonic", side_effect=[0, 0]):
            self.assertIs(checks.context("actual"), state)
        self.assertEqual(state["text"], "")
        self.assertEqual(checks.protocol_delays[0]["kind"], "context_missing_after_activation")

    def test_reference_matching_cached_text_without_widget_focus_is_rejected(self):
        """【OCR 翻译输入测试】【旧焦点拒绝】协议缓存文字吻合但实际字段未聚焦时仍失败，不能提前发送确认，无参数、无返回值"""
        method = SimpleNamespace(states=[{"active": True, "text": "actual"}], read=Mock(return_value=None))
        checks = Checks(method, {}, Path("private"), Path("private/log"), {}, reference=True)
        checks.field = "translation"
        checks.checkpoint = Mock(return_value={"qt": {"focus": "ocrEditor", "translation": {"visible": True, "input_enabled": True, "text": "actual"}}})
        with patch("wayland_ocr_translation_input_checks.time.monotonic", side_effect=[0, 0, 3]):
            with self.assertRaisesRegex(AssertionError, "activate the requested editor"):
                checks.context("actual")
        self.assertEqual(checks.protocol_delays, [])

    def test_native_context_never_uses_a_qt_widget_fallback(self):
        """【OCR 翻译输入测试】【原生严格】原生上下文缺失必须失败，不允许使用原版查询豁免，无参数、无返回值"""
        method = SimpleNamespace(state=Mock(side_effect=AssertionError("missing native context")))
        checks = Checks(method, {}, Path("private"), Path("private/log"), {})
        checks.checkpoint = Mock()
        with self.assertRaisesRegex(AssertionError, "missing native context"):
            checks.context("actual")
        checks.checkpoint.assert_not_called()

    def test_reference_batch_gap_requires_exact_delivery_and_actual_editor_query(self):
        """【OCR 翻译输入测试】【延迟批次】原版查询只能解释已交付批次的上下文缺失，错误序号仍失败，无参数、无返回值"""
        for delivered in (13, 14):
            with tempfile.TemporaryDirectory() as temporary:
                log = Path(temporary) / "process.log"
                initial = "zwp_text_input_manager_v3#11.get_text_input(new id zwp_text_input_v3#18, wl_seat#16)\n" + "zwp_text_input_v3#18.commit()\n" * 13
                log.write_text(initial)
                method = SimpleNamespace(states=[{"active": True, "serial": 5, "text": "cached"}], read=Mock(return_value=None))
                method.batch = Mock(side_effect=lambda **arguments: (log.write_text(initial + f"zwp_text_input_v3#18.done({delivered})\n"), "batch\t14")[1])
                checks = Checks(method, {}, Path(temporary), log, {}, reference=True)
                checks.field = "translation"
                checks.checkpoint = Mock(return_value={"qt": {"translation": {"input_enabled": True, "text": "actual", "cursor": 6, "anchor": 6}}})
                with patch("wayland_ocr_translation_input_checks.time.monotonic", side_effect=[0, 0, 3]):
                    if delivered == 13:
                        self.assertIs(checks.batch("actual", commit="actual"), method.states[-1])
                        self.assertEqual(checks.protocol_delays[0]["kind"], "uncommitted_surrounding_update")
                        self.assertEqual(method.states[-1]["text"], "cached")
                    else:
                        with self.assertRaisesRegex(AssertionError, "did not update its actual editor"):
                            checks.batch("actual", commit="actual")
                        self.assertEqual(checks.protocol_delays, [])


if __name__ == "__main__":
    unittest.main()
