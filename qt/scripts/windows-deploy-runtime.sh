#!/usr/bin/env bash
# Copy the Qt and native runtime DLLs that mark-shot needs next to the executable.
#
# Environment:
#   MARK_SHOT_FFMPEG_PREFIX  Prefix produced by build-minimal-ffmpeg.sh. Its bin/
#                            directory is searched before $MINGW_PREFIX/bin so the
#                            trimmed FFmpeg DLLs win over the full MSYS2 package.
#   MARK_SHOT_RUNTIME_EXTRA_PREFIX  Private side-by-side ABI compatibility DLLs.
#   MARK_SHOT_QT_EXTRA_PLUGINS  Comma-separated Qt plugins needed by test runners.
set -euo pipefail

app_bin="${1:-$PWD/app/bin}"
exe="${2:-$app_bin/mark-shot.exe}"
app_root="$(dirname "$app_bin")"
mingw_bin="${MINGW_PREFIX:-/ucrt64}/bin"

if [ ! -x "$exe" ]; then
  echo "Expected executable not found: $exe" >&2
  exit 1
fi

# 1. 依赖搜索目录：裁剪版 FFmpeg 优先，其次为 MSYS2 运行时
search_dirs=()
if [ -n "${MARK_SHOT_FFMPEG_PREFIX:-}" ]; then
  search_dirs+=("$MARK_SHOT_FFMPEG_PREFIX/bin")
fi
search_dirs+=("$mingw_bin")
if [ -n "${MARK_SHOT_RUNTIME_EXTRA_PREFIX:-}" ]; then
  search_dirs+=("$MARK_SHOT_RUNTIME_EXTRA_PREFIX/bin")
fi

# 【打包】【依赖解析】Windows 导入名不区分大小写，按优先级建立一次索引
declare -A dependency_paths=()
for directory in "${search_dirs[@]}"; do
  for dependency in "$directory"/*; do
    [[ -f "$dependency" ]] || continue
    name="$(basename "$dependency")"
    key="${name,,}"
    if [[ -z "${dependency_paths[$key]:-}" ]]; then
      dependency_paths["$key"]="$dependency"
    fi
  done
done

# 判断参数名称是否属于 FFmpeg 库，匹配时返回成功
is_ffmpeg_dependency() {
  [[ "${1,,}" =~ ^(lib)?(avcodec|avformat|avutil|avfilter|swscale|swresample)(-[0-9]+)?\.dll$ ]]
}

# 判断参数名称是否由 Windows 提供，匹配时返回成功，不把未知 DLL 当系统库忽略
# 构建前缀中存在的同名 DLL 必须随包发布：Windows 11 在 System32 自带旧版 onnxruntime.dll，
# 若按系统库跳过，OCR 插件会加载到不兼容的系统版本
is_system_dependency() {
  local name="${1,,}" system_dir
  if [[ -n "${dependency_paths[$name]:-}" ]]; then
    return 1
  fi
  case "$name" in
    api-ms-*.dll|ext-ms-*.dll|kernel32.dll|kernelbase.dll|ntdll.dll|user32.dll|gdi32.dll|advapi32.dll|\
    shell32.dll|shlwapi.dll|ole32.dll|oleaut32.dll|comdlg32.dll|comctl32.dll|ws2_32.dll|winmm.dll|\
    imm32.dll|version.dll|setupapi.dll|cfgmgr32.dll|dwmapi.dll|uxtheme.dll|d3d11.dll|d3d12.dll|\
    dxgi.dll|dxguid.dll|dwrite.dll|d2d1.dll|opengl32.dll|glu32.dll|bcrypt.dll|crypt32.dll|ncrypt.dll|\
    secur32.dll|sspicli.dll|userenv.dll|psapi.dll|iphlpapi.dll|dnsapi.dll|netapi32.dll|winhttp.dll|\
    wininet.dll|wtsapi32.dll|powrprof.dll|avrt.dll|propsys.dll|rpcrt4.dll|ksuser.dll|mf.dll|\
    mfplat.dll|mfuuid.dll|mfreadwrite.dll|msvcrt.dll|ucrtbase.dll) return 0 ;;
  esac
  system_dir="${WINDIR:-}/System32"
  if command -v cygpath >/dev/null 2>&1 && [[ -n "${WINDIR:-}" ]]; then
    system_dir="$(cygpath -u "$WINDIR")/System32"
  fi
  [[ -n "${WINDIR:-}" && -f "$system_dir/$1" ]]
}

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

  find "${MINGW_PREFIX:-/ucrt64}" -type f -name windeployqt.exe -print -quit
}

# 解析参数 DLL 的实际路径，输出路径；裁剪库 ABI 不匹配或依赖缺失时返回失败
resolve_dependency() {
  local name="$1" key="${1,,}" resolved
  resolved="${dependency_paths[$key]:-}"
  if [[ -n "${MARK_SHOT_FFMPEG_PREFIX:-}" ]] && is_ffmpeg_dependency "$name"; then
    if [[ "$resolved" != "$MARK_SHOT_FFMPEG_PREFIX/bin/"* ]]; then
      echo "FFmpeg ABI mismatch: $name is missing from $MARK_SHOT_FFMPEG_PREFIX/bin; rebuild against this prefix" >&2
      return 1
    fi
  fi
  if [[ -z "$resolved" ]]; then
    echo "Unresolved runtime dependency: $name" >&2
    return 1
  fi
  printf '%s\n' "$resolved"
}

copy_dependency_closure() {
  local current current_key dep_name dep_path dest imports name
  local -a queue
  declare -A seen=()
  mapfile -t queue < <(find "$app_root" -type f \( -iname "*.exe" -o -iname "*.dll" \))

  # 1. 【打包】【OCR运行库】动态加载不出现在 PE 导入表中，显式部署构建前缀提供的运行库并递归计算依赖
  for current in "${queue[@]}"; do
    name="$(basename "$current")"
    if [[ "${name,,}" == "mark-shot-ocr-rapid.dll" || "${name,,}" == "libmark-shot-ocr-rapid.dll" ]]; then
      dep_path="${dependency_paths[onnxruntime.dll]:-${dependency_paths[libonnxruntime.dll]:-}}"
      if [[ -z "$dep_path" ]]; then
        echo "ONNX Runtime is missing from the build prefixes; refusing to package the OCR plugin" >&2
        return 1
      fi
      dest="$app_bin/$(basename "$dep_path")"
      cmp -s "$dep_path" "$dest" || cp "$dep_path" "$dest"
      queue+=("$dest")
      break
    fi
  done

  # 2. 【打包】【媒体裁剪】替换部署工具可能预先复制的完整 FFmpeg，再计算闭包
  if [[ -n "${MARK_SHOT_FFMPEG_PREFIX:-}" ]]; then
    for current in "${queue[@]}"; do
      name="$(basename "$current")"
      if is_ffmpeg_dependency "$name"; then
        dep_path="$(resolve_dependency "$name")"
        cmp -s "$dep_path" "$current" || cp "$dep_path" "$current"
      fi
    done
  fi

  while ((${#queue[@]})); do
    current="${queue[0]}"
    queue=("${queue[@]:1}")
    current_key="$(cygpath -am "$current" 2>/dev/null || printf '%s' "$current")"
    if [[ -n "${seen[$current_key]:-}" ]]; then
      continue
    fi
    seen["$current_key"]=1

    # 3. 【打包】【完整性】解析失败必须终止，不能生成缺依赖的压缩包
    imports="$(objdump -p "$current")"
    while IFS= read -r dep_name; do
      [[ -n "$dep_name" ]] || continue
      if is_system_dependency "$dep_name"; then
        continue
      fi
      if ! dep_path="$(resolve_dependency "$dep_name")"; then
        echo "Required by: $current" >&2
        return 1
      fi
      dest="$app_bin/$(basename "$dep_path")"
      if [ ! -f "$dest" ]; then
        cp "$dep_path" "$dest"
      fi
      queue+=("$dest")
    done < <(printf '%s\n' "$imports" | awk '/DLL Name:/ { print $3 }')
  done
}

windeployqt_path="$(find_windeployqt)"
if [ -z "$windeployqt_path" ]; then
  echo "windeployqt was not found" >&2
  exit 1
fi

# 2. 有 Schannel 时才排除 OpenSSL，缺少系统 TLS 插件的 Qt 构建仍须能访问 HTTPS
tls_options=()
extra_plugin_options=()
if [[ -n "${MARK_SHOT_QT_EXTRA_PLUGINS:-}" ]]; then
  extra_plugin_options+=(--include-plugins "$MARK_SHOT_QT_EXTRA_PLUGINS")
fi
schannel="$(find "${MINGW_PREFIX:-/ucrt64}" -type f -iname qschannelbackend.dll -print -quit)"
if [[ -n "$schannel" ]]; then
  tls_options+=(--exclude-plugins "qopensslbackend,qcertonlybackend")
fi
"$windeployqt_path" --release --compiler-runtime \
  --no-translations \
  --no-system-d3d-compiler \
  --no-opengl-sw \
  --no-quick-import \
  --skip-plugin-types generic,networkinformation,qmltooling \
  "${tls_options[@]}" \
  "${extra_plugin_options[@]}" \
  --dir "$app_bin" "$exe"
if [[ -n "$schannel" && ! -f "$app_bin/tls/qschannelbackend.dll" ]]; then
  echo "Schannel TLS backend was not deployed; refusing to package without HTTPS support" >&2
  exit 1
fi
rm -rf "$app_bin/translations"
copy_dependency_closure

# 3. 输出部署结果与体积，便于在 CI 日志中比对
find "$app_bin" -maxdepth 1 -type f -name "*.dll" -printf "%f\n" | sort
du -sh "$app_root"
