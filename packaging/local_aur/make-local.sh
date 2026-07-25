#!/usr/bin/env bash
# 从仓库根目录生成源码包并在本目录执行 makepkg
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/../.." && pwd)"
pkgver="$(grep -E '^pkgver=' "${script_dir}/PKGBUILD" | cut -d= -f2)"
pkgname="$(grep -E '^pkgname=' "${script_dir}/PKGBUILD" | cut -d= -f2)"
archive="${script_dir}/${pkgname}-${pkgver}.tar.gz"
tag="v${pkgver}"

usage() {
    cat <<EOF
用法: $(basename "$0") [选项]

  默认: 生成 ${pkgname}-${pkgver}.tar.gz 后执行 makepkg -si

选项:
  --tar-only       只生成源码压缩包
  --no-install     执行 makepkg -s（构建包文件，不 pacman -U）
  --working-tree   打包当前工作区（含未提交改动），而不是 git archive 标签/HEAD
  -h, --help       显示本说明
EOF
}

tar_only=0
working_tree=0
makepkg_args=(-si)

while [[ $# -gt 0 ]]; do
    case "$1" in
        --tar-only) tar_only=1; shift ;;
        --no-install) makepkg_args=(-s); shift ;;
        --working-tree) working_tree=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "未知参数: $1" >&2; usage >&2; exit 1 ;;
    esac
done

cd "${repo_root}"
if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "错误: 未在 git 仓库内: ${repo_root}" >&2
    exit 1
fi

rm -f "${archive}"

if [[ "${working_tree}" -eq 1 ]]; then
    # 打包当前工作区：跟踪文件 + 未跟踪源码，排除构建产物与嵌套打包目录
    stage="$(mktemp -d "${TMPDIR:-/tmp}/mark-shot-local-aur.XXXXXX")"
    cleanup() { rm -rf "${stage}"; }
    trap cleanup EXIT

    prefix="${stage}/${pkgname}-${pkgver}"
    mkdir -p "${prefix}"

    # 1. 已跟踪文件（含本地修改）
    git ls-files -z | rsync -a --from0 --files-from=- ./ "${prefix}/"

    # 2. 未跟踪但未被 ignore 的源码（新文件）
    while IFS= read -r -d '' path; do
        case "${path}" in
            packaging/aur/*|packaging/local_aur/*|packaging/aur_bin/*|build/*|build-*/*|.git/*)
                continue
                ;;
        esac
        mkdir -p "${prefix}/$(dirname "${path}")"
        cp -a "${path}" "${prefix}/${path}"
    done < <(git ls-files -z --others --exclude-standard)

    tar -C "${stage}" -czf "${archive}" "${pkgname}-${pkgver}"
    echo "已生成(工作区): ${archive}"
else
    if git rev-parse "${tag}" >/dev/null 2>&1; then
        ref="${tag}"
    else
        echo "提示: 未找到标签 ${tag}，使用当前 HEAD 打源码包" >&2
        ref=HEAD
    fi
    git archive --format=tar.gz --prefix="${pkgname}-${pkgver}/" "${ref}" -o "${archive}"
    echo "已生成: ${archive} (ref=${ref})"
fi

if [[ "${tar_only}" -eq 1 ]]; then
    exit 0
fi

cd "${script_dir}"
# 清理旧构建残留，避免混入上次产物
rm -rf src pkg
# 不清理历史 .pkg.tar.zst，便于对照；同名新包会覆盖
makepkg "${makepkg_args[@]}"
