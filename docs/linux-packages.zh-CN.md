# Linux 安装包选择

从 [GitHub Releases 页面](https://github.com/jswysnemc/mark-shot/releases)下载安装包。不同发行版的 FFmpeg 运行库名称不同，必须选择与目标系统匹配的安装包。

## Debian 与 Ubuntu

| 发行版 | 安装包 | 运行库基线 |
| --- | --- | --- |
| Debian 12 / Deepin | `mark-shot_<version>_amd64.deb` | FFmpeg 5 与旧版 Qt |
| Debian 13 / LMDE 7 | `mark-shot_<version>_amd64.debian13.deb` | FFmpeg 7 与 Debian 13 zxing-cpp |
| Ubuntu 24.04 | `mark-shot_<version>_amd64.ubuntu24.04.deb` | FFmpeg 6 与 Qt 6.4 t64 |
| Ubuntu 26.04 | `mark-shot_<version>_amd64.ubuntu26.04.deb` | 当前 FFmpeg 与 Qt t64 |

使用以下命令安装下载的文件：

```bash
sudo apt install ./<downloaded-mark-shot-package>.deb
```

Debian 12 包不链接 LayerShellQt，以保持对 Deepin 和旧版 Debian 衍生系统的兼容性。在 niri 等合成器中，Wayland 覆盖层可能降级为普通窗口。

Ubuntu 26.04 包会在发行版提供兼容开发包时启用 layer-shell 插件。生成依赖时会同时检查主程序和已经安装的共享库插件。

## AppImage

AppImage 在 Debian 12 基线上构建，拒绝高于 `GLIBC_2.36` 的符号需求，并在发布前通过 Ubuntu 24.04 容器启动测试。因此它不会继承滚动发行版构建机中过新的 glibc 依赖，可用于 Ubuntu 24.04 与 Debian 13。

Debian 12 不提供 Qt 6 LayerShellQt 开发包，因此兼容版 AppImage 不包含这个可选插件，覆盖层会使用普通窗口回退。发行版原生安装包可在存在兼容软件包时启用该插件。

## Fedora

使用以下命令安装 RPM 包：

```bash
sudo dnf install ./mark-shot-<version>-1.x86_64.rpm
```

构建环境仅在依赖可用时编入对应插件。不要混用针对不同发行版构建的独立共享库插件。
