"""【原生发布测试】【固定样本】生成隔离清单、界面验证记录和最小发行归档"""

from dataclasses import asdict
import io
import json
from pathlib import Path
import tarfile

from native_release_artifact import REQUIRED, ROOT
from native_release_manifest import ARCHIVE_NAME, BRANCH, GUI_GROUPS, REPOSITORY, ReleaseManifest, sha256_file


def write_package(directory, replacements=None, extra=()):
    """【原生发布测试】【归档生成】在 directory 写入归档，replacements 替换成员，extra 追加成员，返回文件路径"""
    files = {name: b"license or documentation\n" for name in REQUIRED}
    files.update({"mark-shot": b"\x7fELFtest-binary", "SOURCE-COMMIT": b"a" * 40 + b"\n", "native-help.txt": b"Usage: mark-shot [options]\n", "native-linkage.txt": b"libc.so.6 => /lib/libc.so.6\n"})
    files.update(replacements or {})
    archive = Path(directory) / ARCHIVE_NAME
    with tarfile.open(archive, "w:gz") as package:
        root = tarfile.TarInfo(ROOT)
        root.type = tarfile.DIRTYPE
        package.addfile(root)
        for name, content in sorted(files.items()):
            if content is None:
                continue
            member = tarfile.TarInfo(f"{ROOT}/{name}")
            member.size = len(content)
            member.mode = 0o755 if name == "mark-shot" else 0o644
            package.addfile(member, io.BytesIO(content))
        for member, content in extra:
            package.addfile(member, io.BytesIO(content) if content is not None else None)
    archive.with_name(ARCHIVE_NAME + ".sha256").write_text(f"{sha256_file(archive)}  {ARCHIVE_NAME}\n")
    return archive


def fixture(directory):
    """【原生发布测试】【样本组装】在 directory 生成彼此匹配的清单、报告和说明，返回三者路径及清单对象"""
    directory = Path(directory)
    archive = write_package(directory)
    binary = directory / "binary"
    binary.write_bytes(b"\x7fELFtest-binary")
    digest = sha256_file(binary)
    verification = directory / "native-preview-verification.json"
    verification.write_text(json.dumps({"source_commit": "a" * 40, "run_id": 12, "binary": {"sha256": digest, "bytes": binary.stat().st_size}, "original_goal_complete": False, "user_configuration_modified": False, "original_qt_sources_modified": False, "gui_groups": [{"name": name, "passed": True, "binary_sha256": digest, "scenario_count": count} for name, count in GUI_GROUPS.items()]}))
    notes = directory / "notes.md"
    notes.write_text("Native preview; full rewrite remains incomplete.\n")
    manifest = ReleaseManifest("native-v0.2.0-preview.1", "a" * 40, 12, 34, sha256_file(archive), digest, binary.stat().st_size, sha256_file(verification), sha256_file(notes))
    path = directory / "manifest.json"
    path.write_text(json.dumps({"schema": 1, "repository": REPOSITORY, "branch": BRANCH, **asdict(manifest)}))
    return path, verification, notes, manifest
