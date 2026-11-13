"""【媒体构建】【编译测试】确保项目包装代码严格检查，不把上游 LTO 诊断错误地提升为项目错误"""

from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]


class MediaBuildContracts(unittest.TestCase):
    def test_wrapper_compilation_and_upstream_link_checks_are_separate(self):
        """【媒体构建】【链接测试】无传参，执行实际编译命令片段，拒绝在共享链接阶段向上游传递 Werror，无返回值"""
        script = (ROOT / "scripts/media/build-libraries.sh").read_text()
        start = script.index("read -r -a media_cflags")
        end = script.index('"$stripper" "$build_root/$library"', start)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            native = root / "crates/shot-media/native"
            native.mkdir(parents=True)
            (native / "fixture.c").write_text("int fixture(void) { return 1; }\n")
            environment = """
set -euo pipefail
task_root="$1"
build_root="$1/build"
mkdir -p "$build_root"
library=libmarkshot-media.so
link_flags=()
native_libs=()
compiler=compiler_probe
# 【媒体构建】【测试查询】接收 pkg-config 参数并返回空配置，避免依赖真实 FFmpeg 构建
pkg-config() { :; }
# 【媒体构建】【编译探针】接收实际编译参数，返回成功或 67/68 以标识检查范围错误
compiler_probe() {
  shared=false
  strict=false
  compiling=false
  for argument in "$@"; do
    case "$argument" in
      -shared) shared=true ;;
      -Werror) strict=true ;;
      -c) compiling=true ;;
    esac
  done
  if [[ "$shared" == true && "$strict" == true ]]; then
    echo '【媒体构建】【编译探针】上游 LTO 诊断不应提升为项目错误' >&2
    return 67
  fi
  if [[ "$compiling" == true && "$strict" != true ]]; then
    echo '【媒体构建】【编译探针】项目 C 包装必须保留严格编译检查' >&2
    return 68
  fi
}
"""
            subprocess.run(
                [
                    "bash",
                    "-c",
                    environment + script[start:end],
                    "media-build-test",
                    str(root),
                ],
                check=True,
                capture_output=True,
            )


if __name__ == "__main__":
    unittest.main()
