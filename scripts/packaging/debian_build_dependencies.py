"""【原生打包】【源码依赖】分别校验实际 Rust 工具链与 dpkg 管理的系统构建依赖"""

import argparse
from pathlib import Path
import re
import subprocess
import tempfile
import tomllib


def system_dependencies(root):
    """【原生打包】【依赖解析】按 root 读取 Debian 构建依赖，仅分离 cargo 与 rustc，返回系统依赖表达式"""
    text = (Path(root) / "debian/control").read_text()
    match = re.search(r"^Build-Depends:\s*\n((?:[ \t].*\n)+)", text, re.MULTILINE)
    if not match:
        raise ValueError("expected an explicit Debian Build-Depends block")
    dependencies = [item.strip() for item in match[1].split(",") if item.strip()]
    system = [
        item
        for item in dependencies
        if not re.fullmatch(r"(?:rustc|cargo)\s*\(>= [0-9.]+\)", item)
    ]
    if len(dependencies) - len(system) != 2:
        raise ValueError("Rust package requirements must remain explicitly declared")
    return ", ".join(system)


def verify(root):
    """【原生打包】【依赖验证】校验 root 的实际 Rust 最低版本和全部系统依赖，无返回值"""
    root = Path(root)
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    required = tuple(map(int, manifest["package"]["rust-version"].split(".")))
    for tool in ("cargo", "rustc"):
        result = subprocess.check_output([tool, "--version"], cwd=root, text=True)
        actual = re.match(rf"{tool} ([0-9]+)\.([0-9]+)\.([0-9]+)", result)
        if not actual or tuple(map(int, actual.groups()))[: len(required)] < required:
            raise ValueError(
                f"{tool} does not meet the native minimum version: {result.strip()}"
            )
    subprocess.run(
        ["dpkg-checkbuilddeps", "-d", system_dependencies(root)], cwd=root, check=True
    )


def install(root):
    """【原生打包】【系统依赖】按 root 安装非 Rust 构建依赖，保留原始 control 中的正式版本约束，无返回值"""
    root = Path(root)
    with tempfile.TemporaryDirectory(prefix="mark-shot-build-deps-") as temporary:
        control = Path(temporary) / "control"
        control.write_text(
            "Source: mark-shot-ci\nBuild-Depends: "
            + system_dependencies(root)
            + "\n\nPackage: mark-shot-ci\nArchitecture: any\nDescription: native CI build dependencies\n"
        )
        subprocess.run(
            [
                "mk-build-deps",
                "--install",
                "--remove",
                "--tool",
                "apt-get -o Debug::pkgProblemResolver=yes --no-install-recommends -y",
                str(control),
            ],
            cwd=temporary,
            check=True,
        )


def main():
    """【原生打包】【依赖入口】按参数安装系统依赖或验证真实工具链，成功时才允许源码构建，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--install", action="store_true")
    options = parser.parse_args()
    if options.install:
        install(options.root)
    else:
        verify(options.root)


if __name__ == "__main__":
    main()
