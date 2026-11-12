"""【原生发布测试】【归档回归】验证二进制身份、许可、来源和安全归档边界"""

from dataclasses import replace
from pathlib import Path
import tarfile
import tempfile
import unittest

from native_release_artifact import ROOT, verify_archive
from native_release_manifest import sha256_file
from native_release_test_support import fixture, write_package


class ArtifactTests(unittest.TestCase):
    """【原生发布测试】【产物场景】覆盖有效发行包和非法成员"""

    def test_valid_archive(self):
        """【原生发布测试】【有效归档】完整匹配的发行包通过校验，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            *_, manifest = fixture(directory)
            archive = Path(directory) / "mark-shot-native-linux-x86_64.tar.gz"
            self.assertEqual(verify_archive(archive, manifest)["binary_sha256"], manifest.binary_sha256)

    def test_invalid_contents(self):
        """【原生发布测试】【内容拒绝】拒绝缺失许可、错误来源、不同二进制和禁止依赖，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            *_, manifest = fixture(directory)
            for changes in ({"licenses/DEPENDENCIES.txt": None}, {"licenses/LICENSE-ONNXRUNTIME.txt": b""}, {"SOURCE-COMMIT": b"local-worktree\n"}, {"mark-shot": b"\x7fELFdifferent-binary"}, {"mark-shot": b"not-ELF-binary!"}, {"native-linkage.txt": b"libQt6Core.so"}, {"native-linkage.txt": b"libfontconfig => not found"}, {"native-help.txt": b"unexpected output"}):
                with self.subTest(changes=changes):
                    archive = write_package(directory, changes)
                    current = replace(manifest, archive_sha256=sha256_file(archive))
                    with self.assertRaises(ValueError):
                        verify_archive(archive, current)

    def test_unsafe_members(self):
        """【原生发布测试】【归档路径】拒绝目录越界、绝对路径、链接、设备和重复成员，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            *_, manifest = fixture(directory)
            for name, kind in (("../escape", tarfile.REGTYPE), ("/absolute", tarfile.REGTYPE), (f"{ROOT}/../escape", tarfile.REGTYPE), (f"{ROOT}/link", tarfile.SYMTYPE), (f"{ROOT}/hardlink", tarfile.LNKTYPE), (f"{ROOT}/device", tarfile.CHRTYPE), (f"{ROOT}/README.md", tarfile.REGTYPE)):
                with self.subTest(name=name, kind=kind):
                    member = tarfile.TarInfo(name)
                    member.type = kind
                    member.linkname = "README.md"
                    archive = write_package(directory, extra=((member, None),))
                    current = replace(manifest, archive_sha256=sha256_file(archive))
                    with self.assertRaises(ValueError):
                        verify_archive(archive, current)

    def test_digest_and_checksum(self):
        """【原生发布测试】【摘要拒绝】归档或旁路摘要文件发生变化时拒绝，无参数、无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            *_, manifest = fixture(directory)
            archive = Path(directory) / "mark-shot-native-linux-x86_64.tar.gz"
            with self.assertRaisesRegex(ValueError, "archive digest"):
                verify_archive(archive, replace(manifest, archive_sha256="0" * 64))
            archive.with_name(archive.name + ".sha256").write_text("another digest\n")
            with self.assertRaisesRegex(ValueError, "checksum file"):
                verify_archive(archive, manifest)

    def test_native_media_is_complete_and_matches_its_sources(self):
        """【原生发布测试】【媒体交付】拒绝缺失库、替换库、缺失源码、错误摘要及外部录制程序，无参数无返回值"""
        with tempfile.TemporaryDirectory() as directory:
            *_, manifest = fixture(directory)
            for changes in ({"media/libmarkshot-media.so": None}, {"media/libmarkshot-media.so": b"\x7fELFchanged"}, {"media/MEDIA-SOURCES.tar.xz": b"changed"}, {"media/SHA256SUMS": b"invalid\n"}, {"recording/wf-recorder": b"executable"}):
                with self.subTest(changes=changes):
                    archive = write_package(directory, changes)
                    current = replace(manifest, archive_sha256=sha256_file(archive))
                    with self.assertRaises(ValueError):
                        verify_archive(archive, current)


if __name__ == "__main__":
    unittest.main()
