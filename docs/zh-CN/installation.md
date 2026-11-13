# 安装说明

[English](../installation.md) | **简体中文**

当前原生预发布为
[0.2.0-preview.7](https://github.com/jswysnemc/mark-shot/releases/tag/native-v0.2.0-preview.7)。
0.1.x 安装包属于此前的 Qt 实现，源码保留在 `qt-old` 分支。
本次预发布不更新 AUR 或 PPA 稳定渠道。

## 下载预编译安装包

在发布页面按发行版和架构选择安装包。当前提供 8 个 DEB 安装包、RPM 二进制包与源码包、
Arch 安装包、AppImage、Flatpak、Linux 便携归档和 Windows x86_64 归档。
预编译安装包已包含录制媒体库和许可声明，只有从源码构建时才需要 Rust。

Linux x86_64 便携归档解压后应保留完整目录：

```sh
tar -xzf mark-shot-native-0.2.0-preview.7-linux-x86_64.tar.gz
./mark-shot-native-0.2.0-preview.7-linux-x86_64/mark-shot
```

ARM64 使用 `linux-aarch64` 归档。Windows 解压完整 ZIP 后运行 `mark-shot.exe`，
不要将程序与随包提供的媒体库和资源文件分开。

## 校验下载文件

展开发布页面的 **Files and SHA-256 / 文件列表与 SHA-256**，核对完整摘要。
摘要文件和内部 JSON 元数据不再作为独立发布资产上传。

```sh
sha256sum mark-shot-native-0.2.0-preview.7-linux-x86_64.tar.gz
```

Windows 使用 PowerShell：

```powershell
Get-FileHash .\mark-shot-native-0.2.0-preview.7-windows-x86_64-msvc.zip -Algorithm SHA256
```

发布文件名用 `.` 替换 `~`，以符合 GitHub 资产命名规则。
Debian 和 RPM 包内版本仍保留预发布排序规则。

## 环境要求

以下环境要求和命令适用于从源码构建。

- x86_64 或 aarch64 的 Linux，或 x86_64 的 Windows
- Rust 1.95 或更新版本。仓库在 `rust-toolchain.toml` 中固定了工具链，`rustup` 会在首次构建时自动安装。
- 下文列出的开发库

## 构建依赖

`scripts/packaging/install_dependencies.sh` 可以在 Debian、Ubuntu、Fedora 和 Arch Linux
上安装全部依赖。脚本通过 `sudo` 调用系统包管理器：

```sh
bash scripts/packaging/install_dependencies.sh
```

也可以手动安装：

**Debian 与 Ubuntu**

```sh
sudo apt install build-essential clang libclang-dev pkg-config git python3 curl \
  libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev libxcb1-dev libpipewire-0.3-dev \
  libssl-dev libfontconfig-dev fontconfig fonts-noto-core fonts-noto-cjk nasm \
  libva-dev libdrm-dev libgbm-dev libpulse-dev xz-utils zstd
```

**Fedora**

```sh
sudo dnf install gcc gcc-c++ make clang clang-devel pkgconf-pkg-config git python3 curl \
  wayland-devel libxkbcommon-devel libxkbcommon-x11-devel libxcb-devel pipewire-devel \
  openssl-devel fontconfig-devel google-noto-sans-fonts google-noto-sans-cjk-fonts nasm \
  libva-devel libdrm-devel mesa-libgbm-devel pulseaudio-libs-devel xz zstd
```

**Arch Linux**

```sh
sudo pacman -S --needed base-devel clang pkgconf git python curl wayland libxkbcommon \
  libxkbcommon-x11 libxcb pipewire openssl fontconfig noto-fonts noto-fonts-cjk nasm \
  libva libdrm mesa libpulse xz zstd
```

## 构建

```sh
git clone https://github.com/jswysnemc/mark-shot.git
cd mark-shot
cargo build --release --locked
./target/release/mark-shot --version
```

构建结果是单个可执行文件 `target/release/mark-shot`。截图、标注、贴图、历史和扫码只需要这一个文件。

### 录制媒体库

屏幕录制需要加载独立的媒体库，`cargo build` 不会生成它。构建一次后，把结果放到可执行文件旁的
`media` 目录：

```sh
bash scripts/media/build-libraries.sh linux
mkdir -p target/release/media
cp -r target/media-linux/. target/release/media/
```

脚本下载固定版本的 FFmpeg 与 OpenH264 源码归档，校验摘要后构建一个私有动态库，
不安装也不启动任何录制程序。

### 文字识别构建选项

默认构建在运行时加载 ONNX Runtime。只有在文字识别面板中点击下载按钮后，程序才会下载推理库和语言模型。

| 命令 | 结果 |
| --- | --- |
| `cargo build --release --locked` | 默认方式，推理库在文字识别面板中下载 |
| `cargo build --release --locked --no-default-features --features bundled-ocr` | 内嵌推理库，模型仍按需下载 |
| `cargo build --release --locked --no-default-features --features ocr-gpu` | 内嵌推理库并启用 WebGPU 加速 |

`ocr-dynamic`（默认）与 `bundled-ocr` 互斥，因此不能使用 `--all-features`。

### Windows

使用 MSVC 工具链构建：

```sh
cargo build --release --locked --target x86_64-pc-windows-msvc
```

MKV 与 GIF 导出使用的媒体库是可选组件，需要在 Linux 上用
`bash scripts/media/build-libraries.sh windows` 交叉编译，该命令依赖 MinGW 工具链。
把构建结果放到 `mark-shot.exe` 旁的 `media` 文件夹。

## 运行依赖

源码构建的程序会链接下列系统库。使用安装包时，这些依赖已在包中声明。

| 用途 | 库 |
| --- | --- |
| 核心 | glibc、fontconfig、libxkbcommon、libxkbcommon-x11、libxcb、OpenSSL |
| 截图与录制 | PipeWire、libpulse、libva、libdrm、Mesa（GBM） |
| 可选 | `xdg-desktop-portal` 用于门户截图和 Wayland 全局快捷键；Noto 字体用于显示中日韩文字 |

Mark Shot 不使用 Qt、GTK 或 WebView，也不启动 `grim`、`wl-copy`、`xclip`、`ffmpeg`
这类外部截图、剪贴板或编码工具。程序调用的外部命令只有 fontconfig 提供的 `fc-match`，
用于为文字标注匹配系统字体。

## 安装包

发布 `native-v*` 标签后，发布流程会构建下列安装包。每个 Linux 安装包都包含可执行文件、媒体库和许可声明。

| 格式 | 目标 | 文件名 |
| --- | --- | --- |
| DEB | Debian 12、13，Ubuntu 24.04、26.04；amd64 与 arm64 | `mark-shot_<版本>_<架构><后缀>.deb` |
| RPM | Fedora；x86_64 与 aarch64 | `mark-shot-<版本>-1.<发行版>.<架构>.rpm` |
| Arch 软件包 | x86_64 与 aarch64 | `mark-shot-bin-<版本>-1-<架构>.pkg.tar.zst` |
| AUR | `mark-shot`（源码构建）与 `mark-shot-bin`（预编译） | |
| AppImage | x86_64 | `mark-shot-native-<版本>-linux-x86_64.AppImage` |
| Flatpak 单文件包 | x86_64 | `mark-shot-native-<版本>-linux-x86_64.flatpak` |
| 便携归档 | Linux x86_64 与 aarch64 | `mark-shot-native-<版本>-linux-<架构>.tar.gz` |
| Windows 归档 | x86_64 MSVC | `mark-shot-native-<版本>-windows-x86_64-msvc.zip` |

安装后的目录布局如下：

```text
/usr/bin/mark-shot                      指向可执行文件的符号链接
/usr/lib/mark-shot/mark-shot            可执行文件
/usr/lib/mark-shot/media/               媒体库及其源码和许可
/usr/share/applications/io.github.jswysnemc.MarkShot.desktop
/usr/share/icons/hicolor/scalable/apps/io.github.jswysnemc.MarkShot.svg
/usr/share/licenses/mark-shot/
```

在本地生成该布局的方法见[开发说明](development.md#打包)。

## 安装之后

1. 运行一次 `mark-shot`，确认当前桌面可以正常截图。
2. 为它绑定按键，见[桌面集成](desktop-integration.md)。
3. 运行 `mark-shot --settings`，选择主题、保存目录和完成动作。
