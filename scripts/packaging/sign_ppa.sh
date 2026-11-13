#!/usr/bin/env bash
# 【原生发布】【PPA签名】按 changes 参数签名并验证全部来源文件，私钥仅保留在临时目录
set -euo pipefail
changes="$(realpath "${1:?source changes file required}")"
test -n "${LAUNCHPAD_GPG_PRIVATE_KEY:?Launchpad signing key is required}"
temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT
export GNUPGHOME="$temporary/gnupg"
mkdir -m 700 "$GNUPGHOME"
printf '%s' "$LAUNCHPAD_GPG_PRIVATE_KEY" | gpg --batch --import
fingerprint="$(gpg --batch --with-colons --list-secret-keys | awk -F: '$1 == "fpr" {print $10; exit}')"
test -n "$fingerprint"
export GPG_PASSPHRASE_FILE="$temporary/passphrase"
printf '%s' "${LAUNCHPAD_GPG_PASSPHRASE:-}" > "$GPG_PASSPHRASE_FILE"
chmod 600 "$GPG_PASSPHRASE_FILE"
cat > "$temporary/gpg-wrapper" <<'SH'
#!/bin/sh
exec /usr/bin/gpg --batch --no-tty --pinentry-mode loopback --passphrase-file "$GPG_PASSPHRASE_FILE" "$@"
SH
chmod 700 "$temporary/gpg-wrapper"
debsign -k"$fingerprint" -p"$temporary/gpg-wrapper" "$changes"
gpg --batch --export "$fingerprint" > "$temporary/keyring.gpg"
dscverify --no-conf --no-default-keyrings --keyring "$temporary/keyring.gpg" "$changes"
