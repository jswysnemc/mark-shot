"""【原生发布】【复用测试】防止未验证源码、缺失平台或失败构建进入发布重试"""

import copy
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/packaging"))
from release_build import PUBLICATION_JOB, REQUIRED_GROUPS, validate_run
from release_context import Context


class ReleaseBuild(unittest.TestCase):
    def setUp(self):
        """【原生发布】【构建样本】无传参，创建同一原生标签的完整成功任务和失败发布任务，无返回值"""
        self.context = Context(
            "native-v0.2.0-preview.7", "0.2.0-preview.7", "a" * 40, True
        )
        self.run = {
            "repository": {"full_name": "owner/project"},
            "head_sha": self.context.commit,
            "path": ".github/workflows/native-release.yml",
            "event": "push",
            "status": "completed",
            "conclusion": "failure",
        }
        self.jobs = []
        for prefix, count in REQUIRED_GROUPS.items():
            for index in range(count):
                self.jobs.append(
                    {
                        "id": len(self.jobs) + 1,
                        "name": f"{prefix}fixture-{index}",
                        "status": "completed",
                        "conclusion": "success",
                    }
                )
        self.jobs.append(
            {
                "id": len(self.jobs) + 1,
                "name": PUBLICATION_JOB,
                "status": "completed",
                "conclusion": "failure",
            }
        )

    def test_only_failed_publication_can_be_retried_after_complete_build(self):
        """【原生发布】【复用成功】无传参，允许发布失败但所有构建验证成功的任务进入重试，无返回值"""
        validate_run(self.run, self.jobs, self.context, "owner/project")

    def test_wrong_source_workflow_repository_and_unfinished_run_are_rejected(self):
        """【原生发布】【来源拒绝】无传参，拒绝错误源码、工作流、仓库及未完成任务，无返回值"""
        for field, value in (
            ("head_sha", "b" * 40),
            ("path", ".github/workflows/build.yml"),
            ("repository", {"full_name": "attacker/project"}),
            ("event", "pull_request"),
            ("status", "in_progress"),
            ("conclusion", "cancelled"),
        ):
            with self.subTest(field=field), self.assertRaises(ValueError):
                validate_run(
                    {**self.run, field: value}, self.jobs, self.context, "owner/project"
                )

    def test_every_required_build_group_must_be_complete(self):
        """【原生发布】【覆盖门槛】无传参，逐项拒绝任何缺失或跳过的必需平台和验证，无返回值"""
        for prefix in REQUIRED_GROUPS:
            jobs = copy.deepcopy(self.jobs)
            next(job for job in jobs if job["name"].startswith(prefix))[
                "conclusion"
            ] = "skipped"
            with (
                self.subTest(prefix=prefix),
                self.assertRaisesRegex(ValueError, "incomplete"),
            ):
                validate_run(self.run, jobs, self.context, "owner/project")

    def test_failed_build_is_not_confused_with_failed_publication(self):
        """【原生发布】【失败隔离】无传参，拒绝发布之外的任何失败任务，无返回值"""
        jobs = copy.deepcopy(self.jobs)
        jobs[0]["conclusion"] = "failure"
        with self.assertRaisesRegex(ValueError, "did not succeed"):
            validate_run(self.run, jobs, self.context, "owner/project")

    def test_successful_retry_uses_latest_result_for_the_same_job(self):
        """【原生发布】【重试结果】无传参，同名任务成功重试后只使用最新结果，无返回值"""
        previous = {**self.jobs[0], "id": 0, "conclusion": "failure"}
        validate_run(self.run, [previous, *self.jobs], self.context, "owner/project")


if __name__ == "__main__":
    unittest.main()
