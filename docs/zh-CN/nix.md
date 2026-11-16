# Nix

[English](../nix.md) | **简体中文**

Flake 为 `x86_64-linux` 和 `aarch64-linux` 构建当前 Rust 原生版本。
程序版本读取 `Cargo.toml`，编译器和组件遵循 `rust-toolchain.toml`；
`flake.lock` 固定 nixpkgs、flake-utils 和 rust-overlay。

## 构建与运行

在仓库根目录执行：

```sh
nix build
nix build .#mark-shot
nix run . -- --settings
```

两个构建命令生成相同软件包，程序入口为 `result/bin/mark-shot`。
软件包包含可执行文件、桌面入口、图标、依赖版权通知，以及可替换的进程内录制媒体库和对应源码。

媒体库复用项目的精简 FFmpeg/OpenH264 构建。Nix 在构建前按固定摘要获取源码归档，
Cargo 依赖来自 `Cargo.lock`，构建阶段使用这些本地输入。
默认文字识别功能仍使用动态运行库，可通过应用中的识别资源控件安装运行库和语言模型。

桌面门户、音频服务和显卡驱动由运行系统提供。软件包配置了 PulseAudio、C++ 运行库和
`/run/opengl-driver/lib` 的搜索路径；后台服务直接启动真实程序时也采用这些路径。
程序通过完整路径调用包内的 `fc-match`，字体匹配无需依赖用户的 `PATH` 配置。

## 开发与验证

```sh
nix develop
cargo build --release --locked
./target/release/mark-shot --settings
```

开发环境包含固定版本的 Rust、bindgen 和原生依赖头文件，并通过
`MARK_SHOT_MEDIA_LIBRARY` 提供已构建媒体库。

```sh
nix flake check
nix flake check --all-systems --no-build
nix fmt -- flake.nix packaging/nix/*.nix
```

构建时使用独立配置目录和固定字节的参考字体执行工作区测试，再验证实际软件编码与导出。
安装检查覆盖命令行启动、桌面文件、媒体库接口及摘要。
独立 Nix 工作流分别使用 x86_64 和 ARM64 原生执行器。

程序私有目录包含 `SOURCE-COMMIT`。基于 Git 的 Flake 输入记录对应提交，修改过的工作区附加 `-dirty` 标记；
没有 Git 元数据的本地路径输入明确标记为 `local-working-tree`。

## NixOS 软件包配置

把仓库加入 Flake 输入，并按系统架构选择软件包：

```nix
inputs.mark-shot.url = "github:jswysnemc/mark-shot";

environment.systemPackages = [
  inputs.mark-shot.packages.${pkgs.stdenv.hostPlatform.system}.default
];
```
