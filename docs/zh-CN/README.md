# Mark Shot 文档

[English](../README.md) | **简体中文**

本目录的文档说明 Mark Shot 的原生 Rust 版本（0.2.0）。

## 使用文档

| 文档 | 适用场景 |
| --- | --- |
| [安装说明](installation.md) | 构建 Mark Shot、添加录制媒体库、了解已有的安装包格式 |
| [使用说明](usage.md) | 截图、完成截图、贴图、查看历史、扫码、上传、使用命令行 |
| [标注工具](annotation-tools.md) | 了解各工具的用途，以及如何编辑已有标注 |
| [快捷键](shortcuts.md) | 查询或修改按键 |
| [配置](configuration.md) | 编辑配置文件，查找数据存放位置 |
| [文字识别与翻译](ocr-and-translation.md) | 配置文字识别和翻译服务 |
| [屏幕录制](recording.md) | 把屏幕录制为 MP4、MKV 或 GIF |
| [桌面集成](desktop-integration.md) | 在合成器中绑定按键，启用托盘图标或全局快捷键 |
| [平台支持](platform-support.md) | 确认当前桌面支持哪些功能 |

## 开发文档

| 文档 | 内容 |
| --- | --- |
| [开发说明](development.md) | 仓库结构、架构、检查项、打包 |

## 约定

- 命令中的 `mark-shot` 指构建或安装得到的可执行文件。在源码目录中请使用
  `./target/release/mark-shot`。
- `~/.config/mark-shot/` 这类路径是 Linux 的默认位置，Windows 的对应位置见[配置](configuration.md)。
- 按键名称与配置文件一致，例如 `Ctrl+Shift+S`、`Return`、`Escape`。
