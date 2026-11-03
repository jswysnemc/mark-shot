"""【原生发布】【远程交付】复用成功构建的产物，原子发布预览版并校验重试结果"""

import json
import os
from pathlib import Path
import subprocess

from native_release_artifact import verify_archive
from native_release_manifest import ARCHIVE_NAME, ARTIFACT_NAME, BRANCH, REPOSITORY, require, sha256_file


class GitHub:
    """【原生发布】【远程接口】通过 gh 使用当前工作流令牌，不配置个人凭据"""

    def command(self, arguments):
        """【原生发布】【命令执行】执行 arguments 中的 gh 参数，返回标准输出，失败时保留错误原因"""
        process = subprocess.run(["gh", *arguments], capture_output=True, check=False)
        if process.returncode:
            raise RuntimeError(process.stderr.decode(errors="replace").strip())
        return process.stdout

    def api(self, endpoint, method="GET", payload=None, missing_ok=False):
        """【原生发布】【接口请求】请求 endpoint；method、payload 指定操作，missing_ok 仅容忍 404，返回 JSON"""
        arguments = ["gh", "api", endpoint, "--method", method]
        if payload is not None:
            arguments += ["--input", "-"]
        process = subprocess.run(arguments, input=json.dumps(payload).encode() if payload is not None else None, capture_output=True, check=False)
        if process.returncode:
            error = process.stderr.decode(errors="replace")
            if missing_ok and "(HTTP 404)" in error:
                return None
            raise RuntimeError(error.strip())
        return json.loads(process.stdout) if process.stdout.strip() else None


def verify_source(github, manifest, publication_commit):
    """【原生发布】【来源核验】校验 github 上的任务、产物和 publication_commit 祖先关系，返回产物信息"""
    base = f"repos/{REPOSITORY}"
    run = github.api(f"{base}/actions/runs/{manifest.run_id}")
    require(run.get("status") == "completed" and run.get("conclusion") == "success", "source build did not succeed")
    require(run.get("head_sha") == manifest.source_commit and run.get("head_branch") == BRANCH, "source build revision mismatch")
    require(run.get("repository", {}).get("full_name") == REPOSITORY, "source build repository mismatch")
    require(run.get("path") == ".github/workflows/native-build.yml" and run.get("event") == "push", "unexpected source build workflow")
    comparison = github.api(f"{base}/compare/{manifest.source_commit}...{publication_commit}")
    require(comparison.get("status") in ("ahead", "identical") and comparison.get("merge_base_commit", {}).get("sha") == manifest.source_commit, "publication commit does not descend from verified source")
    artifact = github.api(f"{base}/actions/artifacts/{manifest.artifact_id}")
    require(artifact.get("name") == ARTIFACT_NAME and artifact.get("expired") is False, "missing or expired native artifact")
    require(artifact.get("workflow_run", {}).get("id") == manifest.run_id and artifact.get("workflow_run", {}).get("head_sha") == manifest.source_commit, "artifact source mismatch")
    return artifact


def verify_tag(github, manifest):
    """【原生发布】【标签核验】检查 github 上已有标签是否指向 manifest 来源，返回是否存在"""
    base = f"repos/{REPOSITORY}/git"
    reference = github.api(f"{base}/ref/tags/{manifest.tag}", missing_ok=True)
    if reference is None:
        return False
    target = reference["object"]
    if target["type"] == "tag":
        target = github.api(f"{base}/tags/{target['sha']}")["object"]
    require(target.get("type") == "commit" and target.get("sha") == manifest.source_commit, "existing preview tag points to another commit")
    return True


def verify_published_assets(github, manifest, release, directory):
    """【原生发布】【下载复核】下载 release 的正式资产到 directory，核对 manifest 后返回摘要"""
    expected = {ARCHIVE_NAME, ARCHIVE_NAME + ".sha256", "native-preview-verification.json"}
    assets = release.get("assets", [])
    require(len(assets) == len(expected) and {item["name"] for item in assets} == expected, "published release assets mismatch")
    directory.mkdir(parents=True, exist_ok=False)
    github.command(["release", "download", manifest.tag, "--repo", REPOSITORY, "--dir", str(directory)])
    require(sha256_file(directory / "native-preview-verification.json") == manifest.verification_sha256, "published verification digest mismatch")
    return verify_archive(directory / ARCHIVE_NAME, manifest)


def publish(github, manifest, notes, verification, directory, publication_commit):
    """【原生发布】【完整发布】用 github 发布 manifest 对应产物，notes、verification 为固定说明，返回结果并支持草稿重试"""
    require(os.environ.get("GITHUB_ACTIONS") == "true" and os.environ.get("GITHUB_REPOSITORY") == REPOSITORY, "publishing is restricted to this repository's GitHub Actions")
    require(os.environ.get("GITHUB_REF") == f"refs/heads/{BRANCH}", "publishing is restricted to the native branch")
    base = f"repos/{REPOSITORY}"
    manifest.verify_documents(verification, notes)
    verify_source(github, manifest, publication_commit)
    tag_exists = verify_tag(github, manifest)
    notes_text = Path(notes).read_text()
    release = github.api(f"{base}/releases/tags/{manifest.tag}", missing_ok=True)
    if release is not None:
        require(release.get("prerelease") is True and release.get("body") == notes_text, "existing release metadata mismatch")

    # 1. 已公开的同一预览版只进行下载核验，禁止覆盖正式资产
    if release is not None and not release["draft"]:
        require(tag_exists, "published release tag is missing")
        verify_published_assets(github, manifest, release, directory / "published")
        return {"tag": manifest.tag, "url": release["html_url"], "already_published": True}

    # 2. 只下载已通过测试的原始产物；名称、来源、摘要和归档同时校验
    artifacts = directory / "artifact"
    artifacts.mkdir(parents=True, exist_ok=False)
    github.command(["run", "download", str(manifest.run_id), "--repo", REPOSITORY, "--name", ARTIFACT_NAME, "--dir", str(artifacts)])
    verify_archive(artifacts / ARCHIVE_NAME, manifest)
    if release is None:
        release = github.api(f"{base}/releases", method="POST", payload={"tag_name": manifest.tag, "target_commitish": manifest.source_commit, "name": f"Mark Shot {manifest.tag}", "body": notes_text, "draft": True, "prerelease": True, "make_latest": "false"})
    for asset in release.get("assets", []):
        github.api(f"{base}/releases/assets/{asset['id']}", method="DELETE")
    github.command(["release", "upload", manifest.tag, "--repo", REPOSITORY, str(artifacts / ARCHIVE_NAME), str(artifacts / (ARCHIVE_NAME + ".sha256")), str(verification)])
    release = github.api(f"{base}/releases/{release['id']}")
    verify_published_assets(github, manifest, release, directory / "draft-check")

    # 3. 草稿资产全部校验后公开，工作流令牌避免触发原版稳定渠道
    release = github.api(f"{base}/releases/{release['id']}", method="PATCH", payload={"draft": False, "prerelease": True, "make_latest": "false"})
    require(release.get("draft") is False and release.get("prerelease") is True, "release publication did not complete")
    require(verify_tag(github, manifest), "published tag is missing")
    return {"tag": manifest.tag, "url": release["html_url"], "already_published": False}
