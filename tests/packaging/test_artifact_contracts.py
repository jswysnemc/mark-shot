"""【原生打包】【契约测试】验证版本、架构、摘要及安装树不接受不完整或错误的产物"""

import io
import json
from pathlib import Path
import struct
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/packaging"))
from artifact_metadata import (
    architecture,
    checksum,
    finalize,
    source_sha,
    verify_elf,
    version,
)
from install_linux import install
from media_checksums import verify_media
from package_pacman import package as pacman
from package_portable import package as portable
from package_rpm import package as rpm
from publish_assets import assets
from release_context import Context
from source_tree import archive


COMMIT = "a" * 40


def elf(path, machine=62):
    """【原生打包】【ELF样本】在 path 写入 machine 指定的最小文件头，返回文件路径"""
    header = bytearray(64)
    header[:6] = b"\x7fELF\x02\x01"
    struct.pack_into("<H", header, 18, machine)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(header)
    return path


def fixture(directory):
    """【原生打包】【安装样本】在 directory 创建完整版权、媒体来源和 ELF，返回三个输入目录"""
    directory = Path(directory)
    binary = elf(directory / "mark-shot")
    media = directory / "media"
    elf(media / "libmarkshot-media.so")
    with tarfile.open(media / "MEDIA-SOURCES.tar.xz", "w:xz") as stream:
        entry = tarfile.TarInfo("SOURCE.txt")
        entry.size = 6
        stream.addfile(entry, io.BytesIO(b"source"))
    (media / "SHA256SUMS").write_text(
        "".join(
            f"{checksum(media / name)}  {name}\n"
            for name in ("libmarkshot-media.so", "MEDIA-SOURCES.tar.xz")
        )
    )
    licenses = directory / "licenses"
    licenses.mkdir()
    (licenses / "DEPENDENCIES.txt").write_text("Test dependency notices\n")
    return binary, media, licenses


class ArtifactContracts(unittest.TestCase):
    def test_media_refresh_keeps_source_archive_immutable(self):
        """【原生打包】【摘要刷新】无传参，允许受控库路径修正，但拒绝任何源码变更，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            binary, media, licenses = fixture(Path(temporary))
            original = verify_media(media, arch="x86_64")
            with (media / "libmarkshot-media.so").open("ab") as stream:
                stream.write(b"changed RPATH")
            with self.assertRaises(ValueError):
                verify_media(media, arch="x86_64")
            current = verify_media(media, refresh=True, arch="x86_64")
            self.assertNotEqual(
                current["libmarkshot-media.so"], original["libmarkshot-media.so"]
            )
            self.assertEqual(
                current["MEDIA-SOURCES.tar.xz"], original["MEDIA-SOURCES.tar.xz"]
            )
            verify_media(media, arch="x86_64")
            (media / "MEDIA-SOURCES.tar.xz").write_bytes(b"changed sources")
            with self.assertRaises(ValueError):
                verify_media(media, refresh=True, arch="x86_64")

    def test_source_rpm_assets_do_not_collide_between_architectures(self):
        """【原生打包】【RPM矩阵】无传参，复现两种架构合并产物时的源码包名称冲突，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            source = directory / "upstream.tar.xz"
            source.write_bytes(b"source")
            names = []
            for arch in ("x86_64", "aarch64"):
                destination = directory / arch

                def build(command, **kwargs):
                    """【原生打包】【RPM样本】按 command 模拟本轮架构构建，kwargs 保留调用契约，返回成功结果"""
                    definition = next(
                        value for value in command if value.startswith("_topdir ")
                    )
                    top = Path(definition.removeprefix("_topdir "))
                    binaries = top / "RPMS" / arch
                    binaries.mkdir()
                    (binaries / f"mark-shot-0.2.0-1.fc45.{arch}.rpm").write_bytes(
                        b"binary"
                    )
                    (top / "SRPMS/mark-shot-0.2.0-1.fc45.src.rpm").write_bytes(
                        arch.encode()
                    )
                    return subprocess.CompletedProcess(command, 0)

                with (
                    patch(
                        "package_rpm.architecture",
                        side_effect=lambda value=None: value or arch,
                    ),
                    patch(
                        "package_rpm.prepare", return_value={"source_archive": source}
                    ),
                    patch("package_rpm.subprocess.run", side_effect=build),
                ):
                    rpm(ROOT, "0.2.0", arch, COMMIT, destination)
                names.append(next(destination.glob("*.src.rpm")).name)
            self.assertNotEqual(names[0], names[1])

    def test_input_validation(self):
        """【原生打包】【输入测试】无传参，验证合法架构、版本和完整提交，拒绝路径片段，无返回值"""
        self.assertEqual(architecture("arm64"), "aarch64")
        self.assertEqual(version("0.2.0-rc.1"), "0.2.0-rc.1")
        self.assertEqual(source_sha(COMMIT), COMMIT)
        for value in ("../0.2.0", "0.2.0\n", "0.2.0;ls", "v0.2.0"):
            with self.assertRaises(ValueError):
                version(value)
        with self.assertRaises(ValueError):
            architecture("riscv64")
        with self.assertRaises(ValueError):
            source_sha("abcdef0")

    def test_elf_architecture(self):
        """【原生打包】【架构测试】无传参，拒绝架构错误或非 ELF 程序，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            path = elf(Path(temporary) / "binary")
            verify_elf(path, "x86_64")
            with self.assertRaises(ValueError):
                verify_elf(path, "aarch64")
            path.write_text("not an ELF")
            with self.assertRaises(ValueError):
                verify_elf(path, "x86_64")

    def test_install_is_complete_and_relative(self):
        """【原生打包】【布局测试】无传参，检查相对入口、桌面图标、来源和版权文件，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            binary, media, licenses = fixture(directory / "inputs")
            installed = install(
                ROOT,
                binary,
                media,
                directory / "stage",
                "/usr",
                "x86_64",
                COMMIT,
                licenses,
            )
            self.assertEqual(
                (installed / "bin/mark-shot").readlink().as_posix(),
                "../lib/mark-shot/mark-shot",
            )
            for path in (
                "lib/mark-shot/media/MEDIA-SOURCES.tar.xz",
                "share/licenses/mark-shot/DEPENDENCIES.txt",
                "share/applications/io.github.jswysnemc.MarkShot.desktop",
                "share/metainfo/io.github.jswysnemc.MarkShot.metainfo.xml",
            ):
                self.assertTrue((installed / path).is_file())
            self.assertEqual(
                (installed / "lib/mark-shot/SOURCE-COMMIT").read_text().strip(), COMMIT
            )
            with self.assertRaises(ValueError):
                install(
                    ROOT, binary, media, directory / "bad", "/usr", "x86_64", COMMIT
                )
            with self.assertRaises(ValueError):
                install(
                    ROOT,
                    binary,
                    media,
                    directory / "bad",
                    "/usr/../etc",
                    "x86_64",
                    COMMIT,
                    licenses,
                )
            (media / "libmarkshot-media.so").write_bytes(
                b"\x7fELF\x02\x01" + b"\0" * 58
            )
            with self.assertRaises(ValueError):
                install(
                    ROOT,
                    binary,
                    media,
                    directory / "bad",
                    "/usr",
                    "x86_64",
                    COMMIT,
                    licenses,
                )

    def test_archives_are_reproducible(self):
        """【原生打包】【重现测试】无传参，检查 gzip 文件头及 tar 元数据不依赖生成时间，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            source = directory / "source"
            source.mkdir()
            (source / "test").write_text("content")
            (source / "link").symlink_to("test")
            for suffix in ("tar.gz", "tar.xz"):
                first = archive(
                    source, directory / f"one.{suffix}", "upstream", 1700000000
                )
                second = archive(
                    source, directory / f"two.{suffix}", "upstream", 1700000000
                )
                self.assertEqual(checksum(first), checksum(second))
                with tarfile.open(first) as stream:
                    self.assertTrue(
                        all(
                            item.uid == 0 and item.mtime == 1700000000
                            for item in stream
                        )
                    )

    def test_release_assets_reject_tampering(self):
        """【原生发布】【摘要测试】无传参，拒绝文件篡改、错误版本及路径穿越，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            asset = directory / "package.tar.gz"
            asset.write_bytes(b"test")
            finalize(asset, "0.2.0", "x86_64", COMMIT, "portable")
            context = Context("native-v0.2.0", "0.2.0", COMMIT, False)
            self.assertEqual(len(assets(directory, context)), 3)
            with self.assertRaises(ValueError):
                assets(directory, Context("native-v0.2.1", "0.2.1", COMMIT, False))
            asset.write_bytes(b"tampered")
            with self.assertRaises(ValueError):
                assets(directory, context)
            record = Path(str(asset) + ".json")
            data = json.loads(record.read_text())
            data["file"] = "../outside"
            record.write_text(json.dumps(data))
            with self.assertRaises(ValueError):
                assets(directory, context)

    def test_generated_pacman_and_portable_packages(self):
        """【原生打包】【归档测试】无传参，检查 pacman 元数据和便携包保留完整安装树，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            binary, media, licenses = fixture(directory / "inputs")
            stage = directory / "stage"
            install(ROOT, binary, media, stage, "/usr", "x86_64", COMMIT, licenses)
            record = pacman(stage, "0.2.0", "x86_64", COMMIT, directory / "packages")
            data = subprocess.check_output(
                ["zstd", "-q", "-d", "-c", str(directory / "packages" / record["file"])]
            )
            with tarfile.open(fileobj=io.BytesIO(data)) as stream:
                metadata = stream.extractfile(".PKGINFO").read().decode()
                self.assertIn("pkgname = mark-shot-bin\n", metadata)
                self.assertIn("provides = mark-shot=0.2.0\n", metadata)
                self.assertEqual(
                    stream.getmember("usr/bin/mark-shot").linkname,
                    "../lib/mark-shot/mark-shot",
                )
            record = portable(stage, "0.2.0", "x86_64", COMMIT, directory / "portable")
            with tarfile.open(directory / "portable" / record["file"]) as stream:
                self.assertTrue(
                    any(
                        item.name.endswith("/mark-shot") and item.mode == 0o755
                        for item in stream
                    )
                )


if __name__ == "__main__":
    unittest.main()
