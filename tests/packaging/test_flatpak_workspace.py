"""【原生打包】【Flatpak工作目录】验证缓存恢复或重复执行不能复用旧源码和沙箱仓库"""

from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/packaging"))
from package_flatpak import package


class FlatpakWorkspace(unittest.TestCase):
    def test_failed_sandbox_validation_does_not_publish_a_bundle(self):
        """【原生打包】【沙箱失败清理】无传参，模拟最终沙箱校验失败，确认不交付软件包且保留其他文件，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source.tar.xz"
            source.write_bytes(b"source")
            output = root / "packages"
            output.mkdir()
            marker = output / "keep.txt"
            marker.write_text("unrelated output\n")

            def build(command, **kwargs):
                """【原生打包】【失败样本】按 command 生成待验证包并模拟媒体校验失败，kwargs 保留接口，返回成功或抛出错误"""
                if command[1] == "build-bundle":
                    Path(command[3]).write_bytes(b"unverified bundle")
                if "--command=sh" in command:
                    raise subprocess.CalledProcessError(1, command)
                return subprocess.CompletedProcess(command, 0)

            with (
                patch(
                    "package_flatpak.prepare_flatpak",
                    return_value={"source_archive": source},
                ),
                patch("package_flatpak.subprocess.run", side_effect=build),
                self.assertRaises(subprocess.CalledProcessError),
            ):
                package(ROOT, "0.2.0", "a" * 40, output)
            self.assertEqual(list(output.iterdir()), [marker])
            self.assertEqual(marker.read_text(), "unrelated output\n")

    def test_cached_source_and_repeated_builds_are_isolated(self):
        """【原生打包】【沙箱缓存回归】无传参，保留旧源码和仓库并连续构建不同提交，最终产物必须来自本轮，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "packages"
            for directory in (
                "source/mark-shot-0.2.0",
                "flatpak-build",
                "flatpak-repo",
            ):
                (output / directory).mkdir(parents=True)
                (output / directory / "keep.txt").write_text("cached data\n")
            workspaces = []
            for commit in ("a" * 40, "b" * 40):

                def prepare(repository, source_sha, version, destination):
                    """【原生打包】【沙箱源码样本】按源码身份和版本写 destination，repository 保留调用接口，返回归档记录"""
                    root = Path(destination) / f"mark-shot-{version}"
                    root.mkdir(parents=True, exist_ok=False)
                    archive = root.parent / "source.tar.xz"
                    archive.write_bytes(source_sha.encode())
                    workspaces.append(root.parent.parent)
                    return {"source_archive": archive}

                def build(command, **kwargs):
                    """【原生打包】【沙箱构建样本】按真实 command 创建仓库或包，kwargs 保留接口，返回成功结果"""
                    if command[0] == "flatpak-builder":
                        repo = Path(
                            next(
                                value[7:]
                                for value in command
                                if value.startswith("--repo=")
                            )
                        )
                        repo.mkdir(exist_ok=False)
                        (repo / "commit").write_text(commit)
                    elif command[1] == "build-bundle":
                        Path(command[3]).write_bytes(
                            (Path(command[2]) / "commit").read_bytes()
                        )
                    return subprocess.CompletedProcess(command, 0)

                with (
                    patch("package_flatpak.prepare_flatpak", side_effect=prepare),
                    patch("package_flatpak.subprocess.run", side_effect=build),
                ):
                    record = package(ROOT, "0.2.0", commit, output)
                self.assertEqual(
                    (output / record["file"]).read_bytes(), commit.encode()
                )
            self.assertNotEqual(workspaces[0], workspaces[1])
            self.assertTrue(all(not path.exists() for path in workspaces))
            self.assertTrue(
                all(
                    path.read_text() == "cached data\n"
                    for path in output.rglob("keep.txt")
                )
            )


if __name__ == "__main__":
    unittest.main()
