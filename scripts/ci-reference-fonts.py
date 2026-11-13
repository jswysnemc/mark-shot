#!/usr/bin/env python3
"""Install byte-pinned Qt reference fonts into a Linux CI-only fontconfig tree.

The Qt fixtures (qt-text-storage/probe.h, qt-annotation-collection/probe.h,
qt-selection-size/field-mouse.json) use Noto Sans, including DemiBold. The
reference machine has Noto Sans 2.015, hinted static TTFs (fontversion 132055),
not Ubuntu's fonts-noto-core revision. Never substitute variable/unhinted TTFs.
This does not install fonts globally or change/bundle application fonts.

Usage: python3 scripts/ci-reference-fonts.py [--root target/ci-private/fonts]
Export FONTCONFIG_FILE=<root>/fonts.conf for tests (or pass --github-env).
Use --archive for an offline source archive; --download-only prepares source inputs.
Only the Python standard library, curl and fontconfig commands are required.
"""

import argparse
import hashlib
import os
from pathlib import Path
import subprocess
import sys
import zipfile
from xml.sax.saxutils import escape


URL = (
    "https://github.com/notofonts/latin-greek-cyrillic/releases/download/"
    "NotoSans-v2.015/NotoSans-v2.015.zip"
)
ARCHIVE_SHA256 = "0c34df072a3fa7efbb7cbf34950e1f971a4447cffe365d3a359e2d4089b958f5"
# These hashes also match /usr/share/fonts/noto on the fixture reference host.
FONTS = {
    "Regular": "478c558ea716033cd60c03438f628dfa75694dcf6b5f6d505a2f05fd2b4f3823",
    "Medium": "635d93d1131d791f2576de90b3bb0f7cdf61929906e8420a61b5f7f8e76420bb",
    "SemiBold": "a4e91fd530ac2b4ef5367240144ff37d7d65d66cf76f2e9a2187b93c676f92d0",
    "Bold": "1df075a380fc7cb898acf64c1f7b3b4dd780de3caa860178bf929de35817a913",
    "Italic": "467e3f89eeca4108bb8710a2b9e0cf2281ac56d5b0609211a83776d0505eecb5",
    "MediumItalic": "2c103ff3735d1adffe0cc07f5121492a1ccbb271f46e54bb35d753445b5b3693",
    "SemiBoldItalic": "62b9a66fb79be097ace62febeb884eeaec0e7440cb9e92f5c31db4eeae263e64",
    "BoldItalic": "1b602a9d6353be42c91df097a4857b69fa2696f26703d7a33b54a15d87c2622c",
}
LICENSE_SHA256 = "cee9892f9f0cc8fe882c9e9537ee6a89621d86ee7ceaf70b02e2b2b1c25c061a"


def checked(data, expected, name):
    """【CI字体】【摘要校验】按expected检查data，失败报告name，成功返回原始字节"""
    actual = hashlib.sha256(data).hexdigest()
    if actual != expected:
        raise RuntimeError(f"SHA-256 mismatch for {name}: {actual} != {expected}")
    return data


def source_archive(root, archive=None):
    """【CI字体】【归档准备】在 root 校验或下载固定归档，archive 指定离线来源且禁止联网替代，返回归档路径"""
    root.mkdir(parents=True, exist_ok=True)
    destination = root / "NotoSans-v2.015.zip"
    # 1. 【CI字体】【离线来源】指定源码组件后只校验该文件，缺失或摘要不符直接失败
    if archive is not None:
        archive = Path(archive).resolve()
        data = checked(archive.read_bytes(), ARCHIVE_SHA256, str(archive))
        if archive != destination.resolve():
            destination.write_bytes(data)
    elif not destination.exists():
        temporary = root / "download.zip.part"
        subprocess.run([sys.executable, str(Path(__file__).with_name("ci-download.py")),
                        URL, str(temporary), ARCHIVE_SHA256], check=True)
        checked(temporary.read_bytes(), ARCHIVE_SHA256, URL)
        temporary.replace(destination)
    checked(destination.read_bytes(), ARCHIVE_SHA256, str(destination))
    with zipfile.ZipFile(destination) as source:
        (root / "OFL.txt").write_bytes(
            checked(source.read("OFL.txt"), LICENSE_SHA256, "OFL.txt")
        )
    return destination


def prepare(root, archive=None):
    """【CI字体】【私有安装】在 root 准备固定字体及隔离配置，archive 可指定离线来源，返回配置路径，不修改系统字体"""
    archive = source_archive(root, archive)
    font_dir = root / "ttf"
    font_dir.mkdir(exist_ok=True)
    with zipfile.ZipFile(archive) as source:
        # Extract only known members; retain the upstream OFL notice alongside them.
        for style, digest in FONTS.items():
            name = f"NotoSans-{style}.ttf"
            data = source.read(f"NotoSans/hinted/ttf/{name}")
            (font_dir / name).write_bytes(checked(data, digest, name))
        (root / "OFL.txt").write_bytes(
            checked(source.read("OFL.txt"), LICENSE_SHA256, "OFL.txt")
        )
    config = root / "fonts.conf"
    accepted = "\n".join(
        '<pattern><patelt name="file"><string>'
        + escape(str(font_dir / f"NotoSans-{style}.ttf"))
        + '</string></patelt></pattern>' for style in FONTS
    )
    # Preserve distro fallback/metric aliases; CI isolates XDG_CONFIG_HOME.
    # acceptfont overrides rejectfont for our pinned faces. Reject every other
    # Noto Sans face (including differently named variable files), not CJK families.
    config.write_text(f'''<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
<fontconfig>
  <include>/etc/fonts/fonts.conf</include>
  <dir>{escape(str(font_dir))}</dir>
  <cachedir>{escape(str(root / "cache"))}</cachedir>
  <selectfont>
    <acceptfont>{accepted}</acceptfont>
    <rejectfont><pattern>
      <patelt name="family"><string>Noto Sans</string></patelt>
    </pattern></rejectfont>
  </selectfont>
  <alias binding="strong">
    <family>sans</family><prefer><family>Noto Sans</family><family>Noto Sans CJK SC</family></prefer>
  </alias>
  <alias binding="strong">
    <family>sans-serif</family><prefer><family>Noto Sans</family><family>Noto Sans CJK SC</family></prefer>
  </alias>
  <alias binding="strong">
    <family>Sans Serif</family><prefer><family>Noto Sans</family><family>Noto Sans CJK SC</family></prefer>
  </alias>
</fontconfig>
''', encoding="utf-8")
    return config


def verify(config):
    """【CI字体】【匹配验证】使用config验证全部样式与中文回退，无返回值，匹配错误立即终止"""
    env = dict(os.environ, FONTCONFIG_FILE=str(config))
    subprocess.run(["fc-cache", "--force"], env=env, check=True)
    for family in ("Noto Sans", "sans-serif", "Sans Serif"):
        for style in FONTS:
            weight = next((name.lower() for name in ("Medium", "SemiBold", "Bold")
                           if style.startswith(name)), "regular")
            slant = "italic" if "Italic" in style else "roman"
            pattern = f"{family}:weight={weight}:slant={slant}"
            actual = subprocess.check_output(
                ["fc-match", "-f", "%{file}", pattern], env=env, text=True,
            )
            expected = config.parent / "ttf" / f"NotoSans-{style}.ttf"
            if Path(actual) != expected:
                raise RuntimeError(f"{pattern}: expected {expected}, got {actual}")
            print(f"{pattern} -> {actual}")
    cjk = subprocess.check_output(
        ["fc-match", "-f", "%{family}|%{file}", "sans-serif:lang=zh-cn:charset=4e2d"],
        env=env, text=True,
    )
    if "Noto Sans CJK" not in cjk:
        raise RuntimeError(f"System Noto CJK fallback unavailable: {cjk}")
    print(f"CJK fallback -> {cjk}")


def main():
    """【CI字体】【命令入口】读取参数并准备测试字体，按需写入工作流环境，无参数、无返回值"""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("target/ci-private/fonts"))
    parser.add_argument("--github-env", type=Path, help="append FONTCONFIG_FILE to this env file")
    parser.add_argument("--archive", type=Path, help="use only this verified offline source archive")
    parser.add_argument("--download-only", action="store_true", help="prepare the archive and its license without installing fonts")
    args = parser.parse_args()
    if args.download_only:
        archive = source_archive(args.root.resolve(), args.archive)
        print(f"archive={archive}")
        return
    config = prepare(args.root.resolve(), args.archive)
    verify(config)
    if args.github_env:
        with args.github_env.open("a", encoding="utf-8") as output:
            output.write(f"FONTCONFIG_FILE={config}\n")
    print(f"export FONTCONFIG_FILE={config}")


if __name__ == "__main__":
    main()
