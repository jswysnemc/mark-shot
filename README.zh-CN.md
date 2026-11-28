<div align="center">
  <img src="assets/hicolor/scalable/apps/mark-shot.svg" alt="Mark Shot 标志" width="112" />
  <h1>Mark Shot</h1>
  <p>面向 Linux 与 Windows 的原生截图、标注与屏幕录制工具。</p>
  <p><a href="README.md">English</a> | <strong>简体中文</strong></p>
  <p>
    <a href="https://github.com/jswysnemc/mark-shot/releases">
      <img src="https://img.shields.io/github/v/release/jswysnemc/mark-shot?color=6da0f2&labelColor=4a5054&label=release&style=flat-square&logo=github" alt="发行版本" />
    </a>
    <a href="https://gitter.im/mark-shot/community">
      <img src="https://img.shields.io/badge/gitter-join%20chat-46bc99?labelColor=4a5054&style=flat-square&logo=gitter" alt="Gitter" />
    </a>
    <img src="https://img.shields.io/badge/language-Rust-dfb56c?labelColor=4a5054&style=flat-square&logo=rust" alt="语言：Rust" />
    <img src="https://img.shields.io/badge/platform-Linux%20%7C%20Windows-28c0e7?labelColor=4a5054&style=flat-square" alt="平台：Linux、Windows" />
    <img src="https://img.shields.io/badge/display-Wayland%20%7C%20X11-9979d9?labelColor=4a5054&style=flat-square" alt="显示协议：Wayland、X11" />
    <img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-92d076?labelColor=4a5054&style=flat-square" alt="许可：MIT 或 Apache-2.0" />
  </p>
</div>

Mark Shot 冻结屏幕画面，由用户框选区域并添加标注，然后复制、保存、贴图、识别文字、翻译或上传。
程序使用 Rust 编写，界面由自身的软件渲染器绘制，不依赖 Qt、GTK 或 WebView。

> **当前状态。** `main` 分支为原生 Rust 重写版本。
> 0.1.x 安装包属于此前的 Qt 实现，源码保留在
> [`qt-old`](https://github.com/jswysnemc/mark-shot/tree/qt-old) 分支。
> Linux 的 Wayland 与 X11 是主要目标平台。Windows 版本可以运行，但尚未在真实 Windows 设备上完成全部验证。

## 功能

| 类别 | 内容 |
| --- | --- |
| 截图 | 区域、窗口、显示器与整个桌面截图，延时截图，滚动长截图，输出 JSON 报告的无界面截图 |
| 标注 | 画笔、荧光笔、直线、箭头、矩形、椭圆、形状标记、序号、文字、马赛克与模糊、放大镜、尺子、激光笔 |
| 编辑 | 移动、缩放、旋转标注，多选，撤销与重做，画布缩放与平移，带留白和阴影的导出外框 |
| 贴图 | 把截图、图片文件或剪贴板内容置顶显示，保留剪贴板 HTML/RTF 格式：可读取的本地图片正常渲染，网络图片和无法读取的图片显示为带标签的占位框，同一段落的多张图片排成对齐的行，`align` 与居中布局保留（不含 RTF 嵌入图片和 Office 矢量对象），支持文字识别后选择文字和翻译 |
| 文字识别 | 本地 PP-OCRv5 识别，可编辑的结果窗口，表格可导出为 TSV、CSV、Markdown 和 HTML |
| 翻译 | 哔哩哔哩 Index-Translate、OpenAI、Anthropic、Gemini、腾讯、百度、有道，或自定义命令 |
| 录制 | MP4、MKV 与 GIF，支持暂停和继续、系统声音或麦克风，硬件可用时使用硬件编码 |
| 其他 | 二维码与条形码识别、截图历史、图片上传、托盘图标、全局快捷键、浅色与深色主题 |

## 平台支持

| 平台 | 截图方式 | 说明 |
| --- | --- | --- |
| Wayland：niri、Sway 等 wlroots 合成器 | ext-image-copy-capture、wlr-screencopy | niri 与 Sway 支持窗口吸附 |
| Wayland：KDE Plasma | KWin ScreenShot2，可回退到桌面门户 | |
| Wayland：GNOME | 桌面门户与 PipeWire | 首次使用时门户会请求授权 |
| X11 | XGetImage 与 RandR | 通过 EWMH 支持窗口吸附 |
| Windows | GDI | 实验性支持 |

各桌面的能力和已知限制见[平台支持](docs/zh-CN/platform-support.md)。

GNOME Wayland 可在本仓库目录执行 `bash scripts/install-gnome-pin-extension.sh`，
注销并重新登录后执行 `gnome-extensions enable mark-shot-pin@jswysnemc`。
该可选扩展只在 `mark-shot-pin` 窗口出现时置顶一次；之后可在 GNOME 窗口菜单中取消
“总在最前”，独占全屏窗口仍可能遮挡贴图。其他 Wayland 合成器需要提供
wlr-layer-shell，程序才能控制贴图置顶。

## 安装

### 1. 包管理器快速安装（推荐）

#### Arch Linux (AUR)

Mark Shot 在 Arch Linux 用户软件仓库（AUR）提供预编译包 `mark-shot-bin` 与源码包 `mark-shot`（两者二选一，推荐免去本地编译等待的预编译包）：

```sh
# 方式 A（推荐）：安装预编译版本
yay -S mark-shot-bin
# 或使用 paru
paru -S mark-shot-bin
```

若需在本地由源码编译，使用源码包：

```sh
# 方式 B（二选一）：从源码构建安装
yay -S mark-shot
```

注意：AUR 仓库版本由维护者独立维护，可能滞后于 GitHub 官方发布。安装前可使用 `yay -Si mark-shot-bin` 核对当前仓库版本；若需立即使用最新正式版，也可在 Releases 页面直接下载对应架构的 `mark-shot-bin-*.pkg.tar.zst`，并通过 `sudo pacman -U` 安装。

#### Debian / Ubuntu

在 [Releases](https://github.com/jswysnemc/mark-shot/releases) 页面按系统版本与硬件架构下载对应的 `.deb` 安装包。例如在 Ubuntu 24.04 (x86_64) 下执行：

```sh
sudo apt install ./mark-shot_0.2.4-1_amd64.ubuntu24.04.deb
```

请按实际使用的系统版本与架构替换文件名。

#### Fedora 43

当前预编译 RPM 安装包针对 Fedora 43 构建。在 [Releases](https://github.com/jswysnemc/mark-shot/releases) 页面下载对应架构的 `.rpm` 文件后执行：

```sh
sudo dnf install ./mark-shot-0.2.4-1.fc43.x86_64.rpm
```

请按实际使用的硬件架构替换文件名。

#### Nix / NixOS

使用 Flake 直接运行或加入系统配置：

```sh
nix run github:jswysnemc/mark-shot
```

### 2. 通用包与独立文件

- **AppImage**：下载 `mark-shot-native-*-linux-x86_64.AppImage`，添加可执行权限后直接运行：
  ```sh
  chmod +x mark-shot-native-*-linux-x86_64.AppImage
  ./mark-shot-native-*-linux-x86_64.AppImage
  ```
- **Flatpak**：下载 `.flatpak` 单文件安装包，执行 `flatpak install mark-shot-native-*-linux-x86_64.flatpak`。
- **便携归档（Tarball / Zip）**：在 [Releases](https://github.com/jswysnemc/mark-shot/releases) 页面下载 Linux 或 Windows 便携包，解压后直接执行。
- **Windows 安装程序**：下载 `mark-shot-native-*-windows-x86_64-msvc-setup.exe` 运行安装向导。

发布页面的折叠列表展示全部安装包文件及 SHA-256 校验摘要。完整校验方法与包格式详情见[安装说明](docs/zh-CN/installation.md)。

### 3. 从源码构建

从源码构建时，需要 Rust 1.95 或更新版本；
`rustup` 会按 `rust-toolchain.toml` 自动选用固定的工具链。

```sh
git clone https://github.com/jswysnemc/mark-shot.git
cd mark-shot
bash scripts/packaging/install_dependencies.sh   # Debian、Ubuntu、Fedora 或 Arch
cargo build --release --locked
./target/release/mark-shot --help
```

预编译安装包已包含录制媒体库。从源码构建时还需单独构建媒体库，具体步骤及各发行版的依赖见
[安装说明](docs/zh-CN/installation.md)。

## 快速开始

```sh
mark-shot                    # 框选区域后进入标注
mark-shot -r -t c            # 拖动选区并立即复制
mark-shot -m -f -t s         # 保存指针所在的显示器画面
mark-shot --scroll           # 滚动长截图
mark-shot --ocr              # 识别选区内的文字
mark-shot --record           # 打开录制控制窗口
mark-shot --pin-clipboard    # 把剪贴板中的图片、富文本或文字贴到屏幕上；Mark Shot 复制出的图片保留设备像素比，本地图片正常渲染，网络图片显示为占位框
mark-shot picture.png        # 标注已有图片
mark-shot --settings         # 打开设置
```

可以在合成器中把 `mark-shot` 绑定到按键，也可以在设置窗口启用内置的全局快捷键和托盘图标。
编辑器中 `Enter` 完成截图，`Ctrl+C` 复制，`Ctrl+S` 保存，`Ctrl+P` 贴图，`Esc` 取消。

## 文档

| 文档 | 内容 |
| --- | --- |
| [安装说明](docs/zh-CN/installation.md) | 构建依赖、构建步骤、媒体库、安装包 |
| [Nix](docs/zh-CN/nix.md) | 原生软件包、开发环境和 Flake 验证 |
| [使用说明](docs/zh-CN/usage.md) | 截图方式、完成截图、贴图、历史、扫码、上传、命令行 |
| [标注工具](docs/zh-CN/annotation-tools.md) | 全部工具、选中后的编辑操作、导出外框 |
| [快捷键](docs/zh-CN/shortcuts.md) | 默认按键及修改方法 |
| [配置](docs/zh-CN/configuration.md) | 配置文件、设置窗口、环境变量、数据位置 |
| [文字识别与翻译](docs/zh-CN/ocr-and-translation.md) | 模型、推理库、结果窗口、翻译服务 |
| [屏幕录制](docs/zh-CN/recording.md) | 格式、音频、控制命令、各平台实现 |
| [桌面集成](docs/zh-CN/desktop-integration.md) | 合成器按键绑定、全局快捷键、托盘、后台服务 |
| [平台支持](docs/zh-CN/platform-support.md) | 各桌面的能力与限制 |
| [界面字体](doc/zh-CN/ui-fonts.md) | Linux 界面字体匹配与 Fontconfig 渲染配置 |
| [开发说明](docs/zh-CN/development.md) | 仓库结构、架构、测试、打包 |

英文版本位于 [docs](docs/README.md)。

## 参与贡献

缺陷报告和合并请求请提交到[问题跟踪页](https://github.com/jswysnemc/mark-shot/issues)。
报告缺陷时请附上桌面环境、显示协议和 `mark-shot --version` 的输出。
[开发说明](docs/zh-CN/development.md)列出了提交修改前需要通过的检查。

## 友情链接

- [LINUX DO](https://linux.do/)

## 许可

Mark Shot 采用 [MIT](LICENSE-MIT) 或 [Apache-2.0](LICENSE-APACHE) 双许可，可任选其一。
原生 Wayland 界面、截图后端和部分编辑器代码基于 LumineCapture，后者采用相同的双许可。
内置字体采用 SIL Open Font License（[Inter](assets/fonts/LICENSE-OFL.txt)、
[Mark Shot Numerals](assets/fonts/LICENSE-NUMERALS.txt)），工具栏图标采用
[MIT 许可](assets/icons/mark-shot/LICENSE)。
