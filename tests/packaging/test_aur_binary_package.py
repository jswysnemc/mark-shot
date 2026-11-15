"""【原生发布】【AUR打包测试】验证预编译资产在默认目录中可以重打包并保留安装文件"""

import io
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/packaging"))
from artifact_metadata import finalize
from prepare_aur import prepare
from release_context import Context


@unittest.skipUnless(
    getattr(os, "geteuid", lambda: 0)() != 0
    and all(shutil.which(tool) for tool in ("makepkg", "fakeroot", "bsdtar", "zstd")),
    "requires an unprivileged Arch packaging environment",
)
class AurBinaryPackage(unittest.TestCase):
    def test_default_directories_repackage_both_architectures_without_collision(self):
        """【原生发布】【同名回归】无传参，在默认下载及输出目录打包两种架构并核对文件字节，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            assets = root / "assets"
            assets.mkdir()
            context = Context("v0.2.0", "0.2.0", "a" * 40, True)
            content = b"prebuilt application bytes\n"
            archive = root / "package.tar"
            with tarfile.open(archive, "w") as output:
                for name, data in (
                    ("usr/lib/mark-shot/mark-shot", content),
                    (".PKGINFO", b"pkgname = upstream-asset\n"),
                ):
                    entry = tarfile.TarInfo(name)
                    entry.size = len(data)
                    entry.mode = 0o755 if name.startswith("usr/") else 0o644
                    output.addfile(entry, io.BytesIO(data))
            for arch in ("x86_64", "aarch64"):
                asset = assets / f"mark-shot-bin-0.2.0-1-{arch}.pkg.tar.zst"
                subprocess.run(
                    ["zstd", "-q", str(archive), "-o", str(asset)], check=True
                )
                finalize(asset, context.version, arch, context.commit, "pacman")

            for arch in ("x86_64", "aarch64"):
                with self.subTest(arch=arch):
                    work = root / arch
                    recipe = prepare(ROOT, context, work, True, assets)
                    source = subprocess.check_output(
                        [
                            "bash",
                            "-c",
                            'source "$1"; declare -n files="source_$2"; printf "%s\\n" "${files[@]}"',
                            "bash",
                            str(recipe),
                            arch,
                        ],
                        text=True,
                    ).strip()
                    # 1. 【原生发布】【离线样本】按实际下载名称缓存资产，保留 makepkg 默认输出目录
                    local_name = source.split("::", 1)[0].rsplit("/", 1)[-1]
                    shutil.copyfile(
                        assets / f"mark-shot-bin-0.2.0-1-{arch}.pkg.tar.zst",
                        work / local_name,
                    )
                    config = work / "makepkg.conf"
                    config.write_text(
                        "source /etc/makepkg.conf\n"
                        f"CARCH={arch}\nPKGDEST=\nSRCDEST=\nBUILDDIR=\n"
                        "PKGEXT=.pkg.tar.zst\n"
                    )
                    result = subprocess.run(
                        ["makepkg", "--nodeps", "--nocheck", "--config", str(config)],
                        cwd=work,
                        capture_output=True,
                        text=True,
                    )
                    self.assertEqual(
                        result.returncode, 0, result.stdout + result.stderr
                    )
                    # 2. 【原生发布】【安装核验】检查预编译文件不变，输出元数据采用 AUR 包名与版本
                    package = work / f"mark-shot-bin-0.2.0-1-{arch}.pkg.tar.zst"
                    self.assertEqual(
                        subprocess.check_output(
                            [
                                "bsdtar",
                                "-xOf",
                                str(package),
                                "usr/lib/mark-shot/mark-shot",
                            ]
                        ),
                        content,
                    )
                    metadata = subprocess.check_output(
                        ["bsdtar", "-xOf", str(package), ".PKGINFO"], text=True
                    )
                    self.assertIn("pkgname = mark-shot-bin\n", metadata)
                    self.assertIn("pkgver = 0.2.0-1\n", metadata)
                    self.assertIn(f"arch = {arch}\n", metadata)


if __name__ == "__main__":
    unittest.main()
