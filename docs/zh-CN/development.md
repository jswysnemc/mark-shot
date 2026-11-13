# 开发说明

[English](../development.md) | **简体中文**

## 仓库结构

```text
src/
  main.rs, cli/        程序入口、参数解析、命令调度
  app/                 截图会话：输入调度、帧节奏、导出流程
  backend/             平台后端
    wayland/           截图协议、覆盖层、窗口、贴图、剪贴板、文字输入
    x11/               X11 截图、窗口、剪贴板、XIM
    windows/           Win32 截图、窗口、剪贴板
    window_detection/  窗口几何信息来源：niri、Sway、X11、Windows
  editor/              编辑状态、撤销历史、损坏区域、缓存
  renderer/            标注、文字和像素合成，输出图像
  tools/               各标注工具的输入行为
  features/            功能模块：录制、文字识别结果窗口、历史、贴图、设置、扫码、
                       桌面集成，以及各类标注
  ui/                  工具栏、属性面板、弹层、文字编辑器、文件选择器
  ocr/                 模型、下载、推理、常驻进程
  translation/         翻译服务与任务
  config/              配置类型、设置注册表、持久化、旧格式兼容
  upload/              HTTP 上传与命令上传
crates/
  shot-image/          图像缓冲、模糊、像素化、颜色滤镜、导出外框
  shot-stitch/         滚动长截图拼接
  shot-media/          通过私有 C 库（native/*.c）完成录制和导出
assets/                字体、图标、应用图标
packaging/, debian/    安装包定义
scripts/               打包、媒体库构建、验证工具
tests/                 集成测试与测试数据
docs/                  对外发布的文档
```

`.docs/` 存放开发过程中产生的工作笔记。该目录被 git 忽略，不属于对外发布的文档。

## 架构概要

- **软件渲染。** 界面由 tiny-skia 和 cosmic-text 绘制到共享内存缓冲，没有 GPU 界面运行时，
  也不使用控件库。冻结的截图和动态标注分层存放，只重绘发生变化的区域。
- **后端以 trait 隔离。** `src/backend.rs` 定义截图、覆盖层、窗口和剪贴板的契约。
  功能代码不直接操作 Wayland 或 X11 连接；后端缺少某项能力时返回错误，不做模拟实现。
- **单个可执行文件，多个进程。** 贴图、剪贴板数据提供进程、文字识别常驻进程、文字识别结果窗口、
  历史窗口、设置窗口和录制控制窗口都是同一个程序通过内部参数启动的独立进程。
  因此关闭编辑器即可释放其缓冲和已加载的识别模型。
- **空闲时阻塞。** 没有输入和待处理工作时，事件循环阻塞等待。只有动画或倒计时进行期间才存在定时器。
- **按需创建资源。** 普通截图只初始化后端、图像缓冲和界面字体。文字识别、上传和翻译在首次使用时创建。
- **内存有上限。** 各类缓存都有明确的限额，例如每个滤镜区域最多 4 MiB，总量最多 16 MiB。
  长截图按固定行块增长，并逐行编码。

## 构建

依赖和构建命令见[安装说明](installation.md)。日常开发使用 `cargo build` 的调试构建即可；
`dev` 配置下依赖库已开启优化。

## 检查项

提交的修改需要通过与 `Native Rust Build` 工作流相同的检查：

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

根据修改范围执行下列附加检查：

| 范围 | 命令 |
| --- | --- |
| 启用 WebGPU 的文字识别（x86_64） | `cargo clippy --workspace --all-targets --no-default-features --features ocr-gpu --locked -- -D warnings` |
| Windows | `cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc --locked -- -D warnings` |
| Python 验证工具 | `uv run --with pillow --with numpy python -m unittest discover -s scripts -p 'test_*.py'` |
| 打包脚本 | `uv run --with pyyaml python -m unittest discover -s tests/packaging -v` |
| 工作流 | `bash scripts/packaging/check_workflows.sh` |
| 媒体库 | `MARK_SHOT_MEDIA_LIBRARY="$PWD/target/media-linux/libmarkshot-media.so" cargo test -p shot-media --locked -- --ignored` |

部分测试标记为 `#[ignore]`，因为它们依赖外部资源，例如识别模型、网络服务或媒体库。

### 界面测试

`scripts/wayland-*.py` 和 `scripts/x11-*.py` 在私有且不可见的会话中驱动真实的应用程序，
不会影响当前桌面。`scripts/wayland_test_runtime.py` 在 Xvfb 中启动嵌套的 niri，并在其中运行一个工具：

```sh
cargo build --release --locked
uv run --no-project --python /usr/bin/python3 python scripts/wayland_test_runtime.py -- \
  uv run --no-project --python /usr/bin/python3 python scripts/wayland-arrow.py
```

这些工具需要 niri、Xvfb、niri-use、xdotool、wtype、grim 和 Pillow，它们仅是测试依赖。

## 约定

- 单个源文件不超过 750 行，并且只承担一项职责。文件接近该行数之前按职责拆分。
- 函数的文档注释说明用途、参数和返回值。较大流程中的注释和日志统一使用 `【模块】【动作】`
  前缀，便于检索。
- 日志只使用 `info` 和 `error` 级别。
- 平台不提供某项能力时，返回说明具体原因的错误，不添加看似可用的空实现。
- 提交信息遵循 Conventional Commits：`feat:`、`fix:`、`docs:`、`test:`、`ci:`。

## 打包

`scripts/packaging/build_linux.sh` 构建可执行文件和媒体库，检查动态链接，并收集依赖的许可声明。
随后由 `scripts/packaging/install_linux.py` 生成安装目录：

```sh
bash scripts/packaging/build_linux.sh
python3 scripts/packaging/install_linux.py \
  --destination "$PWD/target/stage" --source-sha "$(git rev-parse HEAD)"
```

`package_deb.py`、`package_rpm.py`、`package_pacman.py`、`package_appimage.py`、
`package_flatpak.py` 和 `package_portable.py` 分别把该目录制作为对应格式的安装包。
发布 `native-v*` 标签后，各 `release-*.yml` 工作流运行其中一个脚本，`windows-build.yml`
生成 Windows 归档。

发行版构建在离线环境中进行：`prepare_source.py` 把 Rust 依赖和媒体库源码一并放入源码归档，
`build_linux.sh --offline` 只使用这些内容。

## 文档

- 对外发布的文档位于 `docs/`，每个页面的中文版本位于 `docs/zh-CN/`。行为变化时请同时更新两者。
- `README.md` 为英文，并链接到 `README.zh-CN.md`。
- 只写代码实际具备的行为。某项功能在某个平台上尚未验证时，请如实说明。
