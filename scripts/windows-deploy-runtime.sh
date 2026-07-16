#!/usr/bin/env bash
set -euo pipefail

app_bin="${1:-$PWD/app/bin}"
exe="${2:-$app_bin/mark-shot.exe}"
app_root="$(dirname "$app_bin")"

if [ ! -x "$exe" ]; then
  echo "Expected executable not found: $exe" >&2
  exit 1
fi

find_windeployqt() {
  local deploy_path

  deploy_path="$(command -v windeployqt || true)"
  if [ -n "$deploy_path" ]; then
    printf '%s\n' "$deploy_path"
    return 0
  fi

  deploy_path="$(command -v windeployqt6 || true)"
  if [ -n "$deploy_path" ]; then
    printf '%s\n' "$deploy_path"
    return 0
  fi

  find /ucrt64 -type f -name windeployqt.exe -print -quit
}

copy_dependency_closure() {
  local current current_key dep_name dep_path dest
  declare -A seen
  mapfile -t queue < <(find "$app_root" -type f \( -name "*.exe" -o -name "*.dll" \))

  while ((${#queue[@]})); do
    current="${queue[0]}"
    queue=("${queue[@]:1}")
    current_key="$(cygpath -am "$current" 2>/dev/null || printf '%s' "$current")"
    if [[ -n "${seen[$current_key]:-}" ]]; then
      continue
    fi
    seen["$current_key"]=1

    while IFS= read -r dep_name; do
      dep_path="/ucrt64/bin/$dep_name"
      if [ ! -f "$dep_path" ]; then
        continue
      fi

      dest="$app_bin/$(basename "$dep_path")"
      if [ ! -f "$dest" ]; then
        cp "$dep_path" "$dest"
      fi
      queue+=("$dest")
    done < <(objdump -p "$current" | awk '/DLL Name:/ { print $3 }')
  done
}

windeployqt_path="$(find_windeployqt)"
if [ -z "$windeployqt_path" ]; then
  echo "windeployqt was not found" >&2
  exit 1
fi

"$windeployqt_path" --release --compiler-runtime --dir "$app_bin" "$exe"
copy_dependency_closure

find "$app_bin" -maxdepth 1 -type f -name "*.dll" -printf "%f\n" | sort
