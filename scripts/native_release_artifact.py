"""【原生发布】【产物校验】检查下载归档的内容、来源、许可与二进制摘要"""

import hashlib
from pathlib import PurePosixPath
import re
import tarfile

from native_release_manifest import ARCHIVE_NAME, require, sha256_file


ROOT = "mark-shot-native"
REQUIRED = {
    "mark-shot", "SOURCE-COMMIT", "LICENSE-MIT", "LICENSE-APACHE", "README.md",
    "native-help.txt", "native-linkage.txt", "licenses/DEPENDENCIES.txt",
    "licenses/LICENSE-OFL.txt", "licenses/LICENSE-NUMERALS.txt", "licenses/LICENSE-ICONS.txt",
    "licenses/LICENSE-ONNXRUNTIME.txt", "licenses/NOTICES-ONNXRUNTIME.txt",
    "docs/rewrite-status.md", "media/libmarkshot-media.so", "media/NOTICES.txt",
    "media/MEDIA-SOURCES.tar.xz", "media/SHA256SUMS", "media/FFMPEG-CONFIG.h",
    "media/licenses/FFMPEG.txt", "media/licenses/OPENH264.txt",
}


def verify_archive(archive, manifest):
    """【原生发布】【归档检查】核验 archive 是否对应 manifest，返回归档与二进制摘要，不解包执行"""
    require(archive.name == ARCHIVE_NAME, "unexpected archive filename")
    require(sha256_file(archive) == manifest.archive_sha256, "archive digest mismatch")
    checksum = archive.with_name(ARCHIVE_NAME + ".sha256")
    require(checksum.read_text() == f"{manifest.archive_sha256}  {ARCHIVE_NAME}\n", "checksum file mismatch")
    with tarfile.open(archive, "r:gz") as package:
        members = {}
        total = 0
        for member in package:
            path = PurePosixPath(member.name)
            require(bool(path.parts) and not path.is_absolute() and ".." not in path.parts and path.parts[0] == ROOT, "unsafe archive path")
            require(member.name not in members, "duplicate archive path")
            require(member.isfile() or member.isdir(), "archive links and special files are forbidden")
            require(member.size >= 0 and not member.mode & 0o7000, "unsafe archive size or permission mode")
            total += member.size
            require(total <= 256 * 1024 * 1024, "archive content exceeds release limit")
            members[member.name] = member
        require(all(f"{ROOT}/{name}" in members and members[f"{ROOT}/{name}"].isfile() for name in REQUIRED), "missing required release files")
        for name in REQUIRED:
            require(members[f"{ROOT}/{name}"].size > 0, f"empty release file: {name}")

        # 1. 二进制必须为经过界面验证的同一 ELF 文件，不运行下载内容
        binary = members[f"{ROOT}/mark-shot"]
        require(binary.size == manifest.binary_bytes and binary.mode & 0o111, "binary size or executable mode mismatch")
        with package.extractfile(binary) as source:
            require(source.read(4) == b"\x7fELF", "release binary is not ELF")
            source.seek(0)
            digest = hashlib.file_digest(source, "sha256").hexdigest()
        require(digest == manifest.binary_sha256, "binary digest mismatch")

        # 2. 来源标记和编译时依赖检查必须对应成功的远程构建
        require(package.extractfile(f"{ROOT}/SOURCE-COMMIT").read().decode() == manifest.source_commit + "\n", "packaged source commit mismatch")
        linkage = package.extractfile(f"{ROOT}/native-linkage.txt").read().decode()
        require(not re.search(r"not found|libQt|libgtk|webkit|webview|webgpu_dawn", linkage, re.IGNORECASE), "forbidden or missing native dependency")
        require(b"Usage:" in package.extractfile(f"{ROOT}/native-help.txt").read(), "missing executable smoke test output")
        # 3. 【原生发布】【媒体库完整性】库和对应源码必须与随包清单一致，禁止重新混入外部录制程序
        media_checksums = package.extractfile(f"{ROOT}/media/SHA256SUMS").read().decode().splitlines()
        expected_media = {"libmarkshot-media.so", "MEDIA-SOURCES.tar.xz"}
        found = set()
        for line in media_checksums:
            match = re.fullmatch(r"([0-9a-f]{64})  (libmarkshot-media\.so|MEDIA-SOURCES\.tar\.xz)", line)
            require(match is not None, "invalid media checksum entry")
            digest_value, name = match.groups()
            require(name not in found, "duplicate media checksum entry")
            found.add(name)
            with package.extractfile(f"{ROOT}/media/{name}") as source:
                require(hashlib.file_digest(source, "sha256").hexdigest() == digest_value, "media checksum mismatch")
        require(found == expected_media, "missing media checksums")
        require(package.extractfile(f"{ROOT}/media/libmarkshot-media.so").read(4) == b"\x7fELF", "media library is not ELF")
        forbidden = {"ffmpeg", "ffprobe", "wf-recorder", "gpu-screen-recorder"}
        require(not any(PurePosixPath(name).name in forbidden for name in members), "external recording executable in package")
    return {"archive_sha256": manifest.archive_sha256, "binary_sha256": digest}
