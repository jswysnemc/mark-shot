"""【原生发布测试】【说明恢复】验证忽略文档后仍按固定摘要读取原始发布说明"""

import hashlib
from pathlib import Path
import subprocess
import tempfile
import unittest

from native_release_notes import resolve_notes


def git(repository, *arguments):
    """【原生发布测试】【历史样本】在 repository 执行 arguments 指定的 Git 操作，返回标准输出字节"""
    return subprocess.check_output(
        [
            "git",
            "-C",
            str(repository),
            "-c",
            "user.name=Test maintainer",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            *arguments,
        ],
        stderr=subprocess.PIPE,
    )


class ReleaseNotesTests(unittest.TestCase):
    """【原生发布测试】【摘要约束】覆盖原文件、删除历史、篡改和历史缺失"""

    def test_local_notes_require_the_exact_digest(self):
        """【原生发布测试】【本地说明】无传参，保留有效文件并拒绝摘要不匹配的文件，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            notes = root / "notes.md"
            notes.write_bytes(b"Local notes\r\n")
            digest = hashlib.sha256(notes.read_bytes()).hexdigest()
            output = root / "output/notes.md"
            self.assertEqual(resolve_notes(root, notes, digest, output), notes)
            self.assertFalse(output.exists())
            with self.assertRaisesRegex(ValueError, "digest mismatch"):
                resolve_notes(root, notes, "0" * 64, output)

    def test_deleted_notes_restore_the_matching_historical_bytes(self):
        """【原生发布测试】【删除恢复】无传参，跳过摘要不匹配的新版本并恢复旧版原始字节，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            git(root, "init", "--initial-branch=main")
            notes = root / "release/preview-notes.md"
            notes.parent.mkdir()
            original = b"Immutable preview notes\r\n"
            notes.write_bytes(original)
            git(root, "add", ".")
            git(root, "commit", "-m", "docs: add preview notes")
            notes.write_bytes(b"Later revision\n")
            git(root, "commit", "-am", "docs: revise preview notes")
            notes.unlink()
            git(root, "commit", "-am", "chore: stop tracking preview notes")
            digest = hashlib.sha256(original).hexdigest()
            output = root / "target/recovered-notes.md"
            self.assertEqual(
                resolve_notes(root, "release/preview-notes.md", digest, output), output
            )
            self.assertEqual(output.read_bytes(), original)
            self.assertFalse(notes.exists())
            with self.assertRaisesRegex(ValueError, "full Git history"):
                resolve_notes(root, notes, "0" * 64, output)

            # 1. 【原生发布测试】【浅历史】模拟默认检出深度，不允许绕过摘要恢复说明
            shallow = root / "shallow"
            git(root, "clone", "--depth=1", root.as_uri(), str(shallow))
            with self.assertRaisesRegex(ValueError, "full Git history"):
                resolve_notes(shallow, "release/preview-notes.md", digest, output)

    def test_missing_notes_outside_the_repository_are_rejected(self):
        """【原生发布测试】【路径边界】无传参，拒绝查询仓库外不存在的说明路径，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with self.assertRaises(ValueError):
                resolve_notes(
                    root, root.parent / "outside-notes.md", "0" * 64, root / "output"
                )


if __name__ == "__main__":
    unittest.main()
