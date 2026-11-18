"""【AUR 回归】【媒体摘要】验证启用 strip 和 debug 时最终包保留已校验共享库"""

import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


@unittest.skipUnless(
    getattr(os, "geteuid", lambda: 0)() != 0
    and all(
        shutil.which(tool) for tool in ("makepkg", "fakeroot", "bsdtar", "gcc", "strip")
    ),
    "requires an unprivileged Arch packaging environment",
)
class AurMediaIntegrity(unittest.TestCase):
    """【AUR 回归】【最终包】使用真实 ELF 和 makepkg 调试处理验证安装包内容"""

    def test_debug_processing_does_not_change_media_checksum(self):
        """【AUR 回归】【调试符号】无传参，复现 debuglink 摘要变化并验证源码包选项保留原库，无返回值"""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "media.c"
            source.write_text("int media_version(void) { return 1; }\n")
            library = root / "libmarkshot-media.so"
            subprocess.run(
                ["gcc", "-g", "-shared", "-fPIC", str(source), "-o", str(library)],
                check=True,
            )
            subprocess.run(["strip", "--strip-unneeded", str(library)], check=True)
            original = library.read_bytes()
            options = (
                subprocess.check_output(
                    [
                        "bash",
                        "-c",
                        'source "$1"; declare -p options',
                        "bash",
                        str(ROOT / "packaging/aur/PKGBUILD.in"),
                    ],
                    text=True,
                )
                .strip()
                .removeprefix("declare -a ")
            )
            # 1. 【AUR 回归】【同源对照】同一份媒体库分别继承 debug 和使用实际源码包选项
            for label, declaration in [("control", "options=()"), ("fixed", options)]:
                work = root / label
                work.mkdir()
                shutil.copyfile(library, work / library.name)
                digest = hashlib.sha256(original).hexdigest()
                (work / "PKGBUILD").write_text(
                    "pkgname=mark-shot-media-check\npkgver=1\npkgrel=1\n"
                    "pkgdesc='Media checksum regression fixture'\narch=('x86_64')\nlicense=('MIT')\n"
                    + declaration
                    + "\n"
                    + 'package() {\n mkdir -p "$pkgdir/usr/lib/mark-shot/media"\n'
                    + ' cp "$startdir/libmarkshot-media.so" "$pkgdir/usr/lib/mark-shot/media/"\n'
                    + f" printf '%s\\n' '{digest}  libmarkshot-media.so' > \"$pkgdir/usr/lib/mark-shot/media/SHA256SUMS\"\n}}\n"
                )
                config = work / "makepkg.conf"
                config.write_text(
                    "source /etc/makepkg.conf\nCARCH=x86_64\n"
                    "OPTIONS=(strip debug)\nPKGDEST=\nSRCDEST=\nBUILDDIR=\nPKGEXT=.pkg.tar.zst\n"
                )
                result = subprocess.run(
                    ["makepkg", "--nodeps", "--nocheck", "--config", str(config)],
                    cwd=work,
                    capture_output=True,
                    text=True,
                )
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                package = work / "mark-shot-media-check-1-1-x86_64.pkg.tar.zst"
                packed = subprocess.check_output(
                    [
                        "bsdtar",
                        "-xOf",
                        str(package),
                        "usr/lib/mark-shot/media/libmarkshot-media.so",
                    ]
                )
                # 2. 【AUR 回归】【字节核验】控制组必须复现变化，修复组必须保留原始摘要与全部文件字节
                if label == "control":
                    self.assertNotEqual(packed, original)
                    self.assertIn(b".gnu_debuglink", packed)
                else:
                    self.assertEqual(packed, original)
                    self.assertEqual(hashlib.sha256(packed).hexdigest(), digest)


if __name__ == "__main__":
    unittest.main()
