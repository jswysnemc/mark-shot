#!/usr/bin/env bash
# 【原生构建】【工作流校验】检查 YAML 表达式、内联脚本及打包入口，失败时阻止构建继续
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
export GOBIN="${RUNNER_TEMP:-$root/target/packaging-tools}/workflow-tools"
mkdir -p "$GOBIN"
go install github.com/rhysd/actionlint/cmd/actionlint@v1.7.11
"$GOBIN/actionlint"
shellcheck scripts/packaging/*.sh scripts/media/build-libraries.sh packaging/appimage/AppRun debian/tests/smoke
