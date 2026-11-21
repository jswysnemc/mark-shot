#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
extension_id=mark-shot-pin@jswysnemc
extension_source="$script_dir/../assets/gnome-shell/$extension_id"
extension_target="${XDG_DATA_HOME:-$HOME/.local/share}/gnome-shell/extensions/$extension_id"

install -d -- "$extension_target"
install -m 644 -- "$extension_source/metadata.json" "$extension_source/extension.js" "$extension_target/"
printf 'Installed %s\nLog out and log back in, then run: gnome-extensions enable %s\n' "$extension_target" "$extension_id"
