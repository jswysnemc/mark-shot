# 原生多平台构建与发布

Qt 版 `origin/main` 的十个工作流已迁移为原生 Rust 入口。平台范围与 Qt 版一致：Linux x86_64、Linux ARM64、Windows x86_64，没有 macOS 构建。

| 工作流 | 原生目标与验证 |
| --- | --- |
| `build.yml` | Linux x86_64/ARM64、Windows x86_64；调用既有原生构建与测试 |
| `release-binaries.yml` | Debian 12 基线的两种 Linux 便携包、Windows MSVC 便携包及可选签名 |
| `release-deb.yml` | Debian 12/13、Ubuntu 24.04/26.04，各含 amd64/arm64，共八组；实际安装验证 |
| `release-rpm.yml` | Fedora x86_64/aarch64；从离线源码构建二进制 RPM 与源码 RPM，实际安装验证 |
| `release-aur-bin.yml` | x86_64/ARM64 pacman 包、摘要与 AUR 二进制元数据 |
| `release-appimage.yml` | x86_64；Debian 12 构建，全部载荷 ELF 的 glibc 需求不超过 2.36，在 Ubuntu 24.04 验证最终包 |
| `release-flatpak.yml` | x86_64；Freedesktop 25.08 SDK，离线源码构建、安装与沙箱启动验证 |
| `debian-source.yml` | Ubuntu 26.04 Resolute；四个上游组件、源码重建、二进制构建、Lintian 与安装测试 |
| `aur-publish.yml` | 两种架构的 AUR 源码包，固定源码提交，生成规范 `.SRCINFO` |
| `ppa-publish.yml` | 正式原生版本校验、Resolute 离线源码准备、GPG 签名验证及 Launchpad 上传 |

公共工作流 `native-release-context.yml` 校验版本和来源，`native-linux-package.yml` 统一 Linux 打包与验证。原有 `native-build.yml`、`windows-build.yml` 和 `native-release.yml` 继续保留，历史预览发布的产物名称及清单契约不变。

## 标签与发布边界

- 自动发布只处理 `native-v*` 标签；Qt 版 `v*` 标签不进入原生发布。
- 发布版本必须匹配目标提交的 `Cargo.toml`，且源码必须包含原生媒体模块与默认动态 OCR 功能。
- 正式资产必须来自标签准确指向的提交，每个发行包附带 SHA-256 和来源 JSON。
- 已上传资产内容相同时跳过；内容不同时失败，不覆盖既有发布。
- 手动执行默认仅生成工作流产物。`upload_to_release`、AUR 的 `publish`/`push_to_aur`、PPA 的 `publish` 均默认为 `false`；显式开启后仍须通过正式标签校验。
- AUR 与 PPA 只接受稳定版本，PPA 上传前还要求对应 GitHub Release 已正式发布，不能是草稿或预发布。

Qt 插件由原生内置 OCR、翻译、扫码与媒体模块替代，不生成不兼容的 Qt 插件资产。默认发行包不嵌入 ONNX Runtime；用户仍通过资源面板明确下载固定摘要的 CPU 推理库和模型。

## 来源、依赖与安装布局

常规构建固定 Rust 1.99.1；Flatpak 使用 Freedesktop SDK 提供的 rust-stable 扩展，并保持默认动态 CPU OCR 配置。Linux 系统包使用 `/usr/lib/mark-shot/mark-shot` 与同目录下 `media/`；`/usr/bin/mark-shot` 使用相对符号链接。Flatpak 使用相同布局，前缀为 `/app`。程序无需修改现有媒体库查找规则。

Ubuntu 26.04 官方仓库当前提供 Rust 1.93.1，不能满足原生项目的 1.95 最低要求。源码持续集成分别验证实际 Rustup 1.99.1 工具链和全部 dpkg 系统依赖，再执行构建；源码包仍保留 `rustc/cargo >= 1.95` 约束。Launchpad 不能联网安装 Rustup，因此 PPA 生产上传还要求提前配置满足约束的构建依赖来源；工作流在发现发行版候选版本不足时明确失败，不上传无法构建的源码。

媒体库保留 LGPL 构建配置、匹配源码、版权与摘要，不启用 FFmpeg 的 GPL 或 nonfree 组件。Debian/PPA 提交四个源码组件：原生 Git 源码、离线 Cargo vendor、原始媒体源码、固定测试字体归档。字体组件保留 Noto Sans 2.015 原始归档及 OFL 许可，仅用于私有字体配置中的 Qt 几何对照测试，不改变程序字体或安装系统字体。源码 RPM 与 Flatpak 同样包含完整离线输入，不以预编译库充当源码。

vendor 通过固定版本 `cargo-vendor-filterer` 生成，保留 `Cargo.lock`。当前锁文件中 libloading、system-deps、vcpkg 的预编译测试样本及 ring 的 Windows 对象文件不参与 Linux 构建，因此单独剔除并更新 vendor 校验清单；其余源码与版权保留。依赖更新后如果出现其他预编译库，源码准备直接失败，不能静默交付。

DEB 的动态依赖在各目标容器内通过 `dpkg-shlibdeps` 生成；RPM 使用目标 Fedora 的自动依赖扫描。pacman x86_64 包在 Arch 容器构建并安装验证。与 Qt 工作流一致，ARM64 pacman 包在 Ubuntu 24.04 ARM 主机生成并验证，不宣称完成 Arch Linux ARM 实机验证；系统库代际不匹配时应使用 AUR 源码包。

RPM 每次使用独立临时目录准备离线源码和执行 `rpmbuild`，只将本轮二进制、源码包及摘要复制到输出目录。RPM 任务不缓存 Cargo target 目录，避免缓存恢复或重试复用旧打包树；已有输出和其他文件不因工作目录清理而删除。

## 渠道配置

| 渠道 | 必要配置 |
| --- | --- |
| GitHub Release | 工作流的 `contents: write` 权限，仅发布任务启用 |
| Windows 签名 | 可选秘密 `WINDOWS_CODESIGN_CERTIFICATE_BASE64`、`WINDOWS_CODESIGN_CERTIFICATE_PASSWORD`；可选变量 `WINDOWS_CODESIGN_TIMESTAMP_URL` |
| AUR | 秘密 `AUR_SSH_PRIVATE_KEY`；变量 `AUR_SSH_KNOWN_HOSTS`，内容为独立核对过的 AUR SSH 主机公钥记录 |
| PPA | `ppa-production` 环境；变量 `PPA_TARGET`，如 `ppa:team/name`；秘密 `LAUNCHPAD_GPG_PRIVATE_KEY`、可选秘密 `LAUNCHPAD_GPG_PASSPHRASE` |

当 Ubuntu 自带 Rust 不足时，可用变量 `PPA_BUILD_DEPENDENCY_PPA` 指定提供新版 Rust 的签名 PPA；生产验证会检查该来源的候选版本。Launchpad 目标 PPA 必须同时配置相同的构建依赖关系，不能只在持续集成容器中添加软件源。

Windows 未配置证书时生成未签名包；配置后签名或验签失败必须阻止交付。只签名应用程序和私有媒体库，不重签系统运行库。签名后重建媒体摘要与最终 ZIP 摘要。AppImage 在路径修正后刷新库摘要，再通过同一固定工具的输出插件封装；DEB 与 RPM 在符号处理后刷新库摘要。所有刷新都要求原始媒体源码归档保持不变，安装验证同时检查包内媒体摘要。

AUR 不采用未校验的即时 SSH 主机扫描。缺少渠道配置时，产物构建仍可测试，但真实上传必须失败并报告缺失配置。

## 本地校验

```sh
uv run --with pyyaml python -m unittest discover -s tests/packaging -v
uv run --with ruff ruff check --select F scripts/packaging tests/packaging
uv run --with ruff ruff format --check scripts/packaging tests/packaging
bash scripts/packaging/check_workflows.sh
desktop-file-validate packaging/desktop/io.github.jswysnemc.MarkShot.desktop
appstreamcli validate --no-net packaging/desktop/io.github.jswysnemc.MarkShot.metainfo.xml
```

源码组件重建测试可使用本机 `dpkg-source`；真实发行版依赖、ARM64 运行和沙箱验证以对应 GitHub 工作流结果为准。
