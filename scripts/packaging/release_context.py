"""【原生发布】【源码边界】只允许原生源码与原生标签进入新的发布工作流"""

import argparse
from dataclasses import dataclass
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib

from artifact_metadata import output, source_sha, version


@dataclass(frozen=True)
class Context:
    tag: str
    version: str
    commit: str
    publish: bool


def git(root, *arguments):
    """【原生发布】【仓库读取】在 root 执行 arguments 指定的 Git 查询，返回完整文本或失败原因"""
    return subprocess.check_output(
        ["git", "-C", str(root), *arguments], text=True
    ).strip()


def resolve(root, tag, reference, publish=False):
    """【原生发布】【上下文校验】按 root、tag、reference 和 publish 返回确认过的原生发布上下文"""
    if not tag.startswith("native-v"):
        raise ValueError(
            "native releases require a native-v tag; Qt v* tags are not accepted"
        )
    package_version = version(tag.removeprefix("native-v"))
    if (
        not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_./+-]*", reference)
        or ".." in reference
    ):
        raise ValueError("invalid source reference")
    commit = None
    for candidate in (reference, f"origin/{reference}"):
        try:
            commit = git(root, "rev-parse", "--verify", f"{candidate}^{{commit}}")
            break
        except subprocess.CalledProcessError:
            continue
    if commit is None:
        subprocess.run(
            ["git", "-C", str(root), "fetch", "--no-tags", "origin", reference],
            check=True,
        )
        commit = git(root, "rev-parse", "--verify", "FETCH_HEAD^{commit}")
    commit = source_sha(commit)
    manifest = tomllib.loads(git(root, "show", f"{commit}:Cargo.toml"))
    if "ocr-dynamic" not in manifest.get(
        "features", {}
    ) or "shot-media" not in manifest.get("dependencies", {}):
        raise ValueError("the requested source is not the native Rust application")
    if (
        package_version.split("-", 1)[0].split("+", 1)[0]
        != manifest["package"]["version"]
    ):
        raise ValueError("release version does not match native Cargo.toml")
    if publish:
        canonical = source_sha(
            git(root, "rev-parse", "--verify", f"refs/tags/{tag}^{{commit}}")
        )
        if canonical != commit:
            raise ValueError("published assets must be built from the exact native tag")
    return Context(tag, package_version, commit, bool(publish))


def main():
    """【原生发布】【命令入口】读取发布参数，校验源码并输出构建版本、提交和发布开关，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--tag", default=os.environ.get("NATIVE_TAG"))
    parser.add_argument("--ref", default=os.environ.get("NATIVE_SOURCE_REF", "HEAD"))
    parser.add_argument("--publish", action="store_true")
    parser.add_argument("--json", type=Path)
    options = parser.parse_args()
    if not options.tag:
        parser.error("a native tag or test version is required")
    context = resolve(options.root, options.tag, options.ref, options.publish)
    values = {
        "tag": context.tag,
        "version": context.version,
        "source_sha": context.commit,
        "checkout_ref": context.commit,
        "publish": context.publish,
    }
    if options.json:
        options.json.parent.mkdir(parents=True, exist_ok=True)
        options.json.write_text(json.dumps(values, indent=2) + "\n")
    output(values)


if __name__ == "__main__":
    main()
