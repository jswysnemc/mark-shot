"""【Nix打包】【迁移契约】验证原生构建入口、锁文件、媒体来源及双架构任务"""

import json
from pathlib import Path
import re
import sys
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/packaging"))

from fetch_media_sources import SOURCES


class NixPackage(unittest.TestCase):
    def test_flake_preserves_entries_with_native_toolchain_and_linux_platforms(self):
        """【Nix打包】【原生迁移】无传参，保留包和开发入口，并拒绝 Qt 构建、旧插件与外部截图工具，无返回值"""
        flake = (ROOT / "flake.nix").read_text()
        package = (ROOT / "packaging/nix/package.nix").read_text()
        files = [ROOT / "flake.nix", *sorted((ROOT / "packaging/nix").glob("*.nix"))]
        text = "\n".join(path.read_text() for path in files)
        for forbidden in (
            "qt6.",
            "wrapQtAppsHook",
            "qtModules",
            "layer-shell-qt",
            "cmakeFlags",
            "runtimeBinaries",
            "grim",
            "wl-clipboard",
        ):
            self.assertNotIn(forbidden, text)
        for expected in (
            '"x86_64-linux"',
            '"aarch64-linux"',
            "default = mark-shot",
            "devShells.default",
            "fromRustupToolchainFile ./rust-toolchain.toml",
        ):
            self.assertIn(expected, flake)
        self.assertIn("rustPlatform.buildRustPackage", package)
        self.assertIn("cargoLock.lockFile = ../../Cargo.lock", package)
        self.assertIn("doCheck = true", package)
        self.assertIn("cargo-about generate --frozen", package)

    def test_fixed_media_sources_match_shared_native_build(self):
        """【Nix媒体】【来源一致】无传参，Nix 的三个固定归档必须与共用构建脚本及摘要完全一致，无返回值"""
        text = (ROOT / "packaging/nix/media-sources.nix").read_text()
        sources = re.findall(r'url = "([^"]+)";\s+sha256 = "([0-9a-f]+)";', text)
        self.assertEqual(set(sources), {(url, digest) for _, url, digest in SOURCES})
        self.assertEqual(len(sources), len(SOURCES))
        media = (ROOT / "packaging/nix/media-library.nix").read_text()
        self.assertIn("fetch_media_sources.py", media)
        self.assertIn("MARK_SHOT_MEDIA_SOURCES", media)
        self.assertIn("bash scripts/media/build-libraries.sh linux", media)
        self.assertIn("media_checksums.py", media)

    def test_lock_pins_toolchain_overlay_and_follows_shared_nixpkgs(self):
        """【Nix打包】【版本锁定】无传参，输入必须包含完整提交和摘要，工具链覆盖复用同一 nixpkgs，无返回值"""
        nodes = json.loads((ROOT / "flake.lock").read_text())["nodes"]
        for name in ("nixpkgs", "rust-overlay", "flake-utils"):
            self.assertRegex(nodes[name]["locked"]["rev"], r"^[0-9a-f]{40}$")
            self.assertTrue(nodes[name]["locked"]["narHash"].startswith("sha256-"))
        self.assertEqual(nodes["rust-overlay"]["inputs"]["nixpkgs"], ["nixpkgs"])

    def test_source_filter_excludes_local_caches_and_keeps_runtime_assets(self):
        """【Nix打包】【源码范围】无传参，源集必须保留原生资源和测试，且不得纳入本地缓存与版本目录，无返回值"""
        text = (ROOT / "packaging/nix/source.nix").read_text()
        for name in ("target", ".git", ".docs", ".ruff_cache", "benchmarks"):
            self.assertNotIn(f'"{name}"', text)
        for name in (
            "src",
            "crates",
            "assets",
            "tests",
            "Cargo.lock",
            "scripts",
            "packaging",
        ):
            self.assertIn(f'"{name}"', text)

    def test_workflow_builds_both_native_architectures_without_rewriting_lock(self):
        """【Nix构建】【任务覆盖】无传参，两个原生架构均执行固定锁文件验证，禁止使用 Qt 构建入口，无返回值"""
        data = yaml.load(
            (ROOT / ".github/workflows/nix-build.yml").read_text(),
            Loader=yaml.BaseLoader,
        )
        job = data["jobs"]["linux"]
        rows = job["strategy"]["matrix"]["include"]
        self.assertEqual(
            {(row["system"], row["runner"]) for row in rows},
            {("x86_64-linux", "ubuntu-24.04"), ("aarch64-linux", "ubuntu-24.04-arm")},
        )
        command = "\n".join(step.get("run", "") for step in job["steps"])
        self.assertIn("nix flake check --no-write-lock-file", command)
        self.assertIn("nix build .#mark-shot --no-write-lock-file", command)


if __name__ == "__main__":
    unittest.main()
