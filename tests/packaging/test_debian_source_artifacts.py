"""【原生源码】【完整交付测试】校验源码归档、描述文件和构建记录不能因路径或通配符错误而遗漏"""

from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/packaging"))
from artifact_metadata import checksum
from collect_debian_source import collect, referenced_files


def manifest(path, files):
    """【原生源码】【清单样本】为 files 写入 path 对应的 SHA-256 和大小，返回描述文件路径"""
    path.write_text(
        "Format: 3.0 (quilt)\nChecksums-Sha256:\n"
        + "".join(
            f" {checksum(item)} {item.stat().st_size} {item.name}\n" for item in files
        )
    )
    return path


class DebianSourceArtifacts(unittest.TestCase):
    def test_all_four_source_components_and_build_records_are_collected(self):
        """【原生源码】【完整归档】无传参，收集四个组件及 Debian 差异、dsc、changes、buildinfo，缺失或篡改时失败，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "container-temp/source"
            source.mkdir(parents=True)
            archives = []
            for suffix in (
                "orig.tar.gz",
                "orig-vendor.tar.xz",
                "orig-media.tar.xz",
                "orig-reference-fonts.tar.xz",
                "debian.tar.xz",
            ):
                path = source / f"mark-shot_0.2.0.{suffix}"
                path.write_bytes(suffix.encode())
                archives.append(path)
            dsc = manifest(source / "mark-shot_0.2.0-1.dsc", archives)
            info = source / "mark-shot_0.2.0-1_source.buildinfo"
            info.write_bytes(b"build record")
            changes = manifest(
                source / "mark-shot_0.2.0-1_source.changes", [dsc, info, *archives]
            )
            destination = root / "workspace/target/ppa-source-artifacts"
            copied = collect(source, destination)
            self.assertEqual(
                {path.name for path in copied},
                {path.name for path in [dsc, changes, info, *archives]},
            )
            archives[0].unlink()
            with self.assertRaisesRegex(ValueError, "missing"):
                collect(source, root / "missing")
            self.assertFalse((root / "missing").exists())
            archives[0].write_bytes(b"changed upstream")
            with self.assertRaisesRegex(ValueError, "mismatch"):
                collect(source, root / "tampered")
            self.assertFalse((root / "tampered").exists())

    def test_clearsigned_inventory_and_unsafe_names(self):
        """【原生源码】【签名清单】无传参，读取已签名清单中的原始字段，并拒绝路径穿越与无摘要清单，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            asset = root / "mark-shot_0.2.0.orig.tar.gz"
            asset.write_bytes(b"source")
            dsc = manifest(root / "mark-shot_0.2.0-1.dsc", [asset])
            dsc.write_text(
                "-----BEGIN PGP SIGNED MESSAGE-----\nHash: SHA512\n\n"
                + dsc.read_text()
                + "\n-----BEGIN PGP SIGNATURE-----\nsignature\n"
            )
            self.assertEqual(referenced_files(dsc), [asset])
            dsc.write_text(dsc.read_text().replace(asset.name, "../" + asset.name))
            with self.assertRaisesRegex(ValueError, "filename"):
                referenced_files(dsc)
            dsc.write_text("Format: 3.0 (quilt)\n")
            with self.assertRaisesRegex(ValueError, "inventory"):
                referenced_files(dsc)


if __name__ == "__main__":
    unittest.main()
