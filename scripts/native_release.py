#!/usr/bin/env python3
"""【原生发布】【命令入口】校验固定清单并通过工作流令牌发布原生预览版"""

import argparse
import json
import os
from pathlib import Path

from native_release_manifest import ReleaseManifest
from native_release_notes import resolve_notes
from native_release_publish import GitHub, publish


def main():
    """【原生发布】【执行入口】解析清单、说明与证据目录，校验或发布后输出结果，失败以非零状态退出"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, default=Path("release/native-preview.json"))
    parser.add_argument("--notes", type=Path, default=Path("release/native-preview-notes.md"))
    parser.add_argument("--verification", type=Path, default=Path("release/native-preview-verification.json"))
    parser.add_argument("--directory", type=Path, default=Path("target/native-publication"))
    parser.add_argument("--validate-only", action="store_true")
    options = parser.parse_args()
    manifest = ReleaseManifest.load(options.manifest)
    notes = resolve_notes(Path.cwd(), options.notes, manifest.notes_sha256, options.directory / "release-notes.md")
    manifest.verify_documents(options.verification, notes)
    if options.validate_only:
        print(json.dumps({"tag": manifest.tag, "validated": True}))
        return
    result = publish(GitHub(), manifest, notes, options.verification, options.directory, os.environ["GITHUB_SHA"])
    print(json.dumps(result))


if __name__ == "__main__":
    main()
