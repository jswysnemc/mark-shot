#!/usr/bin/env bash
# 【原生发布】【AUR上传】仅由正式标签工作流调用，失败时保留失败状态，不误报维护停机为发布成功
set -euo pipefail
name="${1:?package name required}"
prepared="$(realpath "${2:?prepared directory required}")"
case "$name" in mark-shot|mark-shot-bin) ;; *) exit 1 ;; esac
test -s "$prepared/PKGBUILD"
test -s "$prepared/.SRCINFO"
test -n "${AUR_SSH_PRIVATE_KEY:?AUR_SSH_PRIVATE_KEY is required}"
test -n "${AUR_SSH_KNOWN_HOSTS:?AUR_SSH_KNOWN_HOSTS must contain the independently verified AUR host key}"
temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT
chmod 700 "$temporary"
printf '%s\n' "$AUR_SSH_PRIVATE_KEY" > "$temporary/key"
printf '%s\n' "$AUR_SSH_KNOWN_HOSTS" > "$temporary/known_hosts"
chmod 600 "$temporary/key" "$temporary/known_hosts"
export GIT_SSH_COMMAND="ssh -i $temporary/key -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile=$temporary/known_hosts"
git clone "ssh://aur@aur.archlinux.org/$name.git" "$temporary/repository"
cp "$prepared/PKGBUILD" "$prepared/.SRCINFO" "$temporary/repository/"
git -C "$temporary/repository" config user.name 'Mark Shot maintainers'
git -C "$temporary/repository" config user.email 'snemc@qq.com'
git -C "$temporary/repository" add PKGBUILD .SRCINFO
if git -C "$temporary/repository" diff --cached --quiet; then
  printf '【原生发布】【AUR上传】软件包已是当前版本\n'
  exit 0
fi
git -C "$temporary/repository" commit -m 'chore: update native package'
git -C "$temporary/repository" push origin HEAD:master
