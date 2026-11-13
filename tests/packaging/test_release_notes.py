"""【原生发布】【说明测试】验证更新描述、折叠下载列表和页面摘要"""

from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/packaging"))
from artifact_metadata import checksum
from release_asset_names import asset_name
from release_context import Context
from release_notes import render


class ReleaseNotes(unittest.TestCase):
    def test_render_keeps_description_and_collapses_sorted_hashes(self):
        """【原生发布】【页面测试】无传参，验证描述、源码、下载链接及完整 SHA-256 位于折叠列表，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            notes = root / "notes.md"
            notes.write_text("## 更新描述\n\n原生版本预发布。\n", encoding="utf-8")
            files = [root / "z.zip", root / "a~preview.deb"]
            for path in files:
                path.write_bytes(path.name.encode())
            context = Context(
                "native-v0.2.0-preview.6", "0.2.0-preview.6", "a" * 40, True
            )
            body = render(notes, context, files, "owner/project")
            self.assertTrue(body.startswith("## 更新描述"))
            self.assertIn(f"/commit/{context.commit}", body)
            start = body.index("<details>")
            end = body.index("</details>")
            for path in files:
                self.assertIn(checksum(path), body[start:end])
                self.assertIn(
                    f"/releases/download/{context.tag}/{asset_name(path)}",
                    body[start:end],
                )
            self.assertLess(body.index("[a.preview.deb]"), body.index("[z.zip]"))
            self.assertNotIn(".sha256]", body)

    def test_empty_update_description_is_rejected(self):
        """【原生发布】【说明门槛】无传参，拒绝空白更新描述，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            notes = Path(temporary) / "notes.md"
            notes.write_text(" \n")
            with self.assertRaisesRegex(ValueError, "description"):
                render(
                    notes,
                    Context("native-v0.2.0", "0.2.0", "a" * 40, True),
                    [],
                    "owner/project",
                )


if __name__ == "__main__":
    unittest.main()
