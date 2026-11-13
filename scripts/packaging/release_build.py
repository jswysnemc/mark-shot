"""【原生发布】【构建复用】仅允许复用同一标签下全部验证成功的原生构建产物"""

import argparse
import os
from pathlib import Path

from artifact_metadata import output
from release_api import request
from release_context import resolve


REQUIRED_GROUPS = {
    "verify / Linux Wayland ": 2,
    "binaries / packages ": 2,
    "binaries / windows / Windows x86_64 MSVC ": 1,
    "binaries / collect-windows": 1,
    "deb / packages ": 8,
    "rpm / packages ": 2,
    "arch / packages ": 2,
    "appimage / packages ": 1,
    "appimage / verify-ubuntu2404": 1,
    "flatpak / packages": 1,
}
PUBLICATION_JOB = "Publish complete verified release"


def validate_run(run, jobs, context, repository):
    """【原生发布】【来源门槛】按 run、jobs、context 和 repository 验证源码、工作流及完整构建；成功无返回值"""
    if (
        run["repository"]["full_name"] != repository
        or run["head_sha"] != context.commit
        or run["path"] != ".github/workflows/native-release.yml"
        or run["event"] not in ("push", "workflow_dispatch")
        or run["status"] != "completed"
        or run["conclusion"] not in ("success", "failure")
    ):
        raise ValueError("build run does not match the canonical native release source")
    # 1. 【原生发布】【重试归并】同名任务只接受最后一次执行结果，不把早期失败当作成功证据
    latest = {}
    for job in sorted(jobs, key=lambda item: item["id"]):
        latest[job["name"]] = job
    for job in latest.values():
        if job["name"] == PUBLICATION_JOB:
            continue
        if job["status"] != "completed" or job["conclusion"] not in (
            "success",
            "skipped",
        ):
            raise ValueError(f"build or verification did not succeed: {job['name']}")
    # 2. 【原生发布】【平台覆盖】允许跳过稳定渠道，但不得缺少任何必需验证、架构或安装包任务
    for prefix, minimum in REQUIRED_GROUPS.items():
        completed = [
            job
            for name, job in latest.items()
            if name.startswith(prefix) and job["conclusion"] == "success"
        ]
        if len(completed) < minimum:
            raise ValueError(f"required build group is incomplete: {prefix}")


def main():
    """【原生发布】【复用入口】读取标签与任务编号，校验 GitHub 构建结果并输出固定源码及任务编号，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag", required=True)
    parser.add_argument("--run-id", type=int, required=True)
    options = parser.parse_args()
    if options.run_id <= 0:
        parser.error("run ID must be positive")
    context = resolve(Path.cwd(), options.tag, options.tag, publish=True)
    repository = os.environ["GITHUB_REPOSITORY"]
    base = f"repos/{repository}/actions/runs/{options.run_id}"
    run = request(base)
    jobs = []
    page = 1
    while True:
        batch = request(f"{base}/jobs?filter=all&per_page=100&page={page}")["jobs"]
        jobs.extend(batch)
        if len(batch) < 100:
            break
        page += 1
    validate_run(run, jobs, context, repository)
    output({"tag": context.tag, "source_sha": context.commit, "run_id": options.run_id})


if __name__ == "__main__":
    main()
