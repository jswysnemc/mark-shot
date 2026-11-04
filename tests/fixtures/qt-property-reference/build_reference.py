"""【属性对照】【只读构建】验证原版对象未过期，替换程序入口后链接测试探针，不写入原版构建目录"""

import argparse
import json
from pathlib import Path
import shlex
import subprocess


def flags(path, key):
    """【属性对照】【编译参数】从 path 的 Makefile 参数中读取 key，返回保留原版宏定义与 ABI 的参数列表"""
    line = next(line for line in path.read_text().splitlines() if line.startswith(key + " ="))
    return shlex.split(line.split("=", 1)[1])


def build(original, output, source=None, qt_test=False):
    """【属性对照】【探针链接】读取 original 当前对象与依赖，将 source 链接到 output；qt_test 仅增加官方测试库，返回构建证据，拒绝过期对象"""
    original, output = original.resolve(), output.resolve()
    target = original / "CMakeFiles/mark-shot.dir"
    command = shlex.split((target / "link.txt").read_text())
    objects = [(original / arg).resolve() for arg in command if arg.endswith(".o")]
    source_root = Path(__file__).resolve().parent
    source = Path(source).resolve() if source else source_root / "main.cpp"
    output.parent.mkdir(parents=True, exist_ok=True)

    # 1. 【属性对照】【依赖校验】原版源文件和头文件晚于对象时停止，不把旧对象误当作当前源码的实际实现
    newest = 0.0
    for obj in objects:
        dep = Path(str(obj) + ".d")
        if not obj.is_file() or not dep.is_file():
            raise RuntimeError(f"missing original object dependency: {obj}")
        dependencies = shlex.split(dep.read_text().replace("\\\n", " ").split(":", 1)[1])
        for name in dependencies:
            path = Path(name)
            if path.is_file():
                newest = max(newest, path.stat().st_mtime)
                if path.stat().st_mtime > obj.stat().st_mtime:
                    raise RuntimeError(f"stale original object: {obj}; newer dependency: {path}")

    # 2. 【属性对照】【入口编译】只编译本目录的探针，原版对象、资源与配置代码保持不变
    parameter_file = target / "flags.make"
    compile_flags = sum((flags(parameter_file, key) for key in ("CXX_DEFINES", "CXX_INCLUDES", "CXX_FLAGS")), [])
    qt_source = next(Path(arg[2:]) for arg in compile_flags if arg.startswith("-I") and arg.endswith("/qt/src"))
    compile_flags.append("-I" + str(qt_source.parent / "tests"))
    probe = output.with_suffix(".o")
    subprocess.run([command[0], *compile_flags, "-c", str(source), "-o", str(probe)], check=True)

    # 3. 【属性对照】【输出隔离】绝对路径读取原版对象，链接器输出与依赖文件只放在本次证据目录
    linked = []
    index = 0
    while index < len(command):
        arg = command[index]
        if arg == "-o":
            linked.extend((arg, str(output)))
            index += 2
            continue
        if arg.startswith("-Wl,--dependency-file="):
            index += 1
            continue
        if arg.endswith("/src/main.cpp.o"):
            arg = str(probe)
        elif not arg.startswith("-") and index > 0:
            arg = str((original / arg).resolve())
        linked.append(arg)
        index += 1
    if qt_test:
        linked.append("-lQt6Test")
    subprocess.run(linked, cwd=output.parent, check=True)
    return {"original_build": str(original), "objects": len(objects), "stale_objects": 0,
            "newest_dependency_mtime": newest, "output": str(output), "probe_source": str(source)}


def main():
    """【属性对照】【构建入口】解析原版构建目录与探针目标，将验证结果写入目标旁 JSON，无返回值"""
    parser = argparse.ArgumentParser()
    parser.add_argument("--original-build", type=Path, default=Path.home() / "workspace/mark-shot/build/qt")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--source", type=Path)
    parser.add_argument("--qt-test", action="store_true")
    options = parser.parse_args()
    result = build(options.original_build, options.output, options.source, options.qt_test)
    options.output.with_suffix(".build.json").write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
