# GitHub 插件市场方案

Mark Shot 插件市场不需要自建后端，也不需要 Python 环境。一个公开 GitHub 仓库即可承载市场索引，插件二进制文件放在各插件仓库的 GitHub Release 资产中。

## 仓库结构

推荐创建独立仓库，例如 `mark-shot-plugin-index`：

```text
mark-shot-plugin-index/
├── plugins.json
└── README.md
```

`plugins.json` 使用 `docs/plugin-index-schema.md` 定义的结构。Mark Shot 下载该文件后，用内置 C++/Qt 解析器校验字段、筛选当前平台资产、下载动态库并校验 SHA-256。

## 发布流程

插件作者在自己的插件仓库中发布 Release：

```text
mark-shot-plugin-sample-ocr/
└── Releases
    └── v0.1.0
        ├── mark-shot-sample-ocr.dll
        └── libmark-shot-sample-ocr.so
```

市场维护者只需要把 Release 资产 URL、文件名、平台、架构、SHA-256 和插件能力写入 `plugins.json`。示例见 `examples/plugin-index.example.json`。

## 安装链路

1. Mark Shot 读取 GitHub 上的 `plugins.json`。
2. 内置索引解析器校验 schema、插件字段和资产字段。
3. 应用筛选当前平台和架构，例如 `windows` + `x86_64`。
4. 应用下载对应 Release 资产到临时文件。
5. 应用校验 SHA-256。
6. 应用把动态库安装到用户级插件目录。
7. 重启 Mark Shot 后，Provider 插件注册器扫描目录并加载插件。

用户级插件目录以设置页 `Plugins` 显示为准。Windows 通常位于 `%LOCALAPPDATA%` 下，Linux 默认位于 `~/.local/share/mark-shot/plugins`。

## 资产格式

当前内置安装器只处理平台动态库：

| 平台 | 资产格式 |
|---|---|
| Windows | `.dll` |
| Linux | `.so` |
| macOS | `.dylib` |

压缩包仍可作为手工分发格式，但不作为市场一键安装格式。需要压缩包安装时，应在 Mark Shot 内实现 C++ 解压模块，而不是要求用户安装外部脚本环境。

## 安全边界

- `downloadUrl` 必须是 HTTPS。
- `sha256` 必须是 64 位十六进制摘要。
- `fileName` 只能是文件名，不能包含路径分隔符。
- 安装器只写入用户级插件目录。
- 插件仍是本机动态库，市场只解决发现、下载和校验，不改变插件的本机代码执行属性。
