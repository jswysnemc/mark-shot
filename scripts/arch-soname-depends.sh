#!/usr/bin/env bash
# 从已安装树中的 ELF 文件提取共享库 soname，输出 pacman 风格的 soname 依赖行。
#
# 手写 .PKGINFO 的预编译包（mark-shot-bin）绕过了 makepkg 的 find_libdepends，
# 因而只声明 `depend = ffmpeg` 这类无版本约束的包名。FFmpeg 大版本升级时
# soname 会整体 +1（例如 libavformat.so.62 -> .so.63），旧包仍然“满足”依赖检查，
# 但运行时找不到库而崩溃。这里按 makepkg 的规则生成 `libavformat.so=62-64`
# 形式的依赖，让 pacman 在库代际不匹配时直接拒绝安装或提示升级。
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: arch-soname-depends.sh --root DIR [--filter REGEX]

Options:
  --root DIR       安装树根目录，递归扫描其中的 ELF 文件
  --filter REGEX   只输出 soname 匹配该扩展正则的依赖，默认输出全部

Output:
  每行一个 pacman 依赖，形如 libavformat.so=62-64
EOF
}

ROOT=""
FILTER=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --root) ROOT="$2"; shift 2 ;;
        --filter) FILTER="$2"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "unknown argument: $1" >&2; usage >&2; exit 1 ;;
    esac
done

if [[ -z "$ROOT" ]]; then
    usage >&2
    exit 1
fi

if [[ ! -d "$ROOT" ]]; then
    echo "root directory does not exist: $ROOT" >&2
    exit 1
fi

declare -A LIBDEPS=()

# readelf 缺失时下面的循环会静默跳过每个文件并输出空结果，让调用方误以为
# 安装树没有共享库依赖。这里显式检查，把工具缺失和“确实没有依赖”区分开。
if ! command -v readelf >/dev/null 2>&1; then
    echo "readelf was not found; install binutils to generate soname dependencies" >&2
    exit 1
fi

ELF_COUNT=0

# 1. 遍历安装树中的所有可执行 ELF 文件，收集 DT_NEEDED 条目
while IFS= read -r -d '' filename; do
    # readelf -h 读不到 ELF Class 说明不是 ELF 文件，跳过
    soarch="$(LC_ALL=C readelf -h "$filename" 2>/dev/null | sed -n 's/.*Class.*ELF\(32\|64\)/\1/p')"
    [[ -n "$soarch" ]] || continue
    ELF_COUNT=$((ELF_COUNT + 1))

    while IFS= read -r sofile; do
        [[ -n "$sofile" ]] || continue
        # 2. 按 makepkg 规则拆出库名与主版本：libavformat.so.62 -> libavformat.so + 62
        case "$sofile" in
            *.so.*)
                soname="${sofile%%.so.*}.so"
                soversion="${sofile##*.so.}"
                ;;
            *)
                continue
                ;;
        esac
        # 只接受纯数字主版本，形如 libfoo.so.1.2 的次版本一并忽略
        soversion="${soversion%%.*}"
        [[ "$soversion" =~ ^[0-9]+$ ]] || continue

        entry="${soversion}-${soarch}"
        if [[ -n "${LIBDEPS[$soname]:-}" ]]; then
            if [[ " ${LIBDEPS[$soname]} " != *" $entry "* ]]; then
                LIBDEPS[$soname]+=" $entry"
            fi
        else
            LIBDEPS[$soname]="$entry"
        fi
    done < <(LC_ALL=C readelf -d "$filename" 2>/dev/null \
        | sed -nr 's/.*Shared library: \[(.*)\].*/\1/p')
done < <(find "$ROOT" -type f -perm -u+x -print0)

# 安装树里一个 ELF 都没有，通常意味着传错了目录或安装步骤没执行
if [[ "$ELF_COUNT" -eq 0 ]]; then
    {
        echo "No ELF binaries were found under $ROOT"
        echo "Files considered (executable bit set):"
        find "$ROOT" -type f -perm -u+x -printf '  %M %p\n' 2>/dev/null | head -20
    } >&2
    exit 1
fi

# 3. 按库名排序输出，保证同一安装树生成的依赖顺序稳定
for soname in $(printf '%s\n' "${!LIBDEPS[@]}" | sort); do
    if [[ -n "$FILTER" ]] && ! [[ "$soname" =~ $FILTER ]]; then
        continue
    fi
    for version in ${LIBDEPS[$soname]}; do
        echo "${soname}=${version}"
    done
done
