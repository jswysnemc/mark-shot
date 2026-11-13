"""【原生打包】【RPM工作目录】复现缓存恢复和重复构建时的目录冲突及陈旧产物混入"""

from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/packaging"))
from package_rpm import package


class RpmWorkspace(unittest.TestCase):
    def test_failed_build_removes_only_its_temporary_workspace(self):
        """【原生打包】【失败清理】无传参，模拟 rpmbuild 失败，确认临时目录清理且其他输出保留，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source.tar.xz"
            source.write_bytes(b"source")
            output = root / "packages"
            output.mkdir()
            marker = output / "keep.txt"
            marker.write_text("unrelated output\n")
            with (
                patch(
                    "package_rpm.architecture",
                    side_effect=lambda value=None: value or "aarch64",
                ),
                patch("package_rpm.prepare", return_value={"source_archive": source}),
                patch(
                    "package_rpm.subprocess.run",
                    side_effect=subprocess.CalledProcessError(1, "rpmbuild"),
                ),
                self.assertRaises(subprocess.CalledProcessError),
            ):
                package(ROOT, "0.2.0", "aarch64", "a" * 40, output)
            self.assertEqual(list(output.iterdir()), [marker])
            self.assertEqual(marker.read_text(), "unrelated output\n")

    def test_cached_directories_and_repeated_builds_are_isolated(self):
        """【原生打包】【缓存回归】无传参，保留旧目录并连续构建两种源码，拒绝复用陈旧 RPM，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "packages"
            cached = output / "rpmbuild"
            for name in ("BUILD", "BUILDROOT", "RPMS", "SOURCES", "SPECS", "SRPMS"):
                (cached / name).mkdir(parents=True)
            stale = cached / "RPMS/mark-shot-old.aarch64.rpm"
            stale.write_bytes(b"cached binary")
            old_source = output / "source/mark-shot-0.2.0"
            old_source.mkdir(parents=True)
            marker = old_source / "SOURCE-COMMIT"
            marker.write_text("old source\n")
            unrelated = output / "other-package.txt"
            unrelated.write_text("keep this file\n")
            workspaces = []
            for commit in ("a" * 40, "b" * 40):

                def prepare(repository, source_sha, version, destination, kind):
                    """【原生打包】【源码样本】按提交、版本和格式写入 destination，返回源码路径，repository 保留接口"""
                    root = Path(destination) / f"mark-shot-{version}"
                    root.mkdir(parents=True, exist_ok=False)
                    archive = root.parent / "source.tar.xz"
                    archive.write_bytes(source_sha.encode())
                    return {"source_archive": archive}

                def build(command, **kwargs):
                    """【原生打包】【构建样本】按 command 的真实工作目录写本轮 RPM，kwargs 保留接口，返回成功结果"""
                    definition = next(
                        value for value in command if value.startswith("_topdir ")
                    )
                    top = Path(definition.removeprefix("_topdir "))
                    workspaces.append(top.parent)
                    binaries = top / "RPMS/aarch64"
                    binaries.mkdir()
                    (binaries / "mark-shot-0.2.0-1.fc45.aarch64.rpm").write_bytes(
                        commit.encode()
                    )
                    (top / "SRPMS/mark-shot-0.2.0-1.fc45.src.rpm").write_bytes(
                        commit.encode()
                    )
                    return subprocess.CompletedProcess(command, 0)

                with (
                    patch(
                        "package_rpm.architecture",
                        side_effect=lambda value=None: value or "aarch64",
                    ),
                    patch("package_rpm.prepare", side_effect=prepare),
                    patch("package_rpm.subprocess.run", side_effect=build),
                ):
                    asset = package(ROOT, "0.2.0", "aarch64", commit, output)
                self.assertEqual(asset.read_bytes(), commit.encode())
                self.assertEqual(
                    (output / "mark-shot_0.2.0_fedora_aarch64.src.rpm").read_bytes(),
                    commit.encode(),
                )
            self.assertNotEqual(workspaces[0], workspaces[1])
            self.assertTrue(all(not path.exists() for path in workspaces))
            self.assertEqual(stale.read_bytes(), b"cached binary")
            self.assertEqual(marker.read_text(), "old source\n")
            self.assertEqual(unrelated.read_text(), "keep this file\n")


if __name__ == "__main__":
    unittest.main()
