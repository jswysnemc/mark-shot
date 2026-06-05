# 使用 GitHub CLI (gh) 提交 Issue 说明

GitHub CLI (命令为 `gh`) 是 GitHub 官方提供的命令行工具。本文件介绍如何在本项目中使用 `gh` 工具提交 Issue，以提高问题反馈与处理的效率。

## 准备工作

在提交 Issue 之前，需要安装并配置好 GitHub CLI 工具。

### 1. 安装 GitHub CLI
根据您的操作系统，选择对应的安装方式。详细安装指南请参阅 [GitHub CLI 官方安装说明](https://cli.github.com)。

### 2. 身份验证
安装完成后，在终端运行以下命令进行登录认证：
```bash
gh auth login
```
按照终端提示选择您的 GitHub 账号进行登录，并完成授权。

---

## 提交方式

为了方便收集关键的系统运行环境，本项目提供了两种提交 Issue 的方式。由于本软件（mark-shot）作为截图标注工具，其运行表现与显示服务器（Wayland 或 X11）及窗口管理器（如 niri、sway、GNOME 等）密切相关，因此推荐使用“自动收集环境脚本”方式提交 Bug。

### 方式一：使用辅助脚本自动收集环境（推荐用于 Bug 反馈）

本项目在 [scripts/submit-issue.sh](file:///home/snemc/workspace/mark-shot/scripts/submit-issue.sh) 中提供了一个自动化脚本。此脚本会自动收集当前主机的以下环境信息：
- 操作系统版本
- 当前窗口管理器或桌面环境
- 显示服务器类型（Wayland 或 X11）
- 安装的 Qt 版本
- mark-shot 软件版本

运行此脚本即可开始提交流程：
```bash
./scripts/submit-issue.sh
```

**执行流程**：
1. 运行后，脚本会提示选择 Issue 类型（Bug 报告或功能请求）。
2. 若选择 Bug 报告，脚本将自动提取当前系统的各项参数，填入 [bug_report.md](file:///home/snemc/workspace/mark-shot/.github/ISSUE_TEMPLATE/bug_report.md) 模板中。
3. 接着脚本会调用 `gh issue create`，在终端引导您输入 Issue 标题，并提供进一步编辑描述内容或直接提交的选项。
4. 若选择功能请求，脚本会直接载入 [feature_request.md](file:///home/snemc/workspace/mark-shot/.github/ISSUE_TEMPLATE/feature_request.md) 模板供您填写。

---

### 方式二：直接使用 gh 命令（交互式）

如果您不想运行辅助脚本，也可以直接在项目根目录下使用 `gh` 的标准交互式命令。

#### 1. 提交 Bug 报告
在终端执行以下命令：
```bash
gh issue create --template "bug_report.md"
```
终端会提示您输入标题，接着让您选择如何编辑 Issue 内容（例如选择在终端编辑器中编辑，或者在 Web 浏览器中打开编辑）。请在对应的“运行环境”一节中手动填写您的系统配置。

#### 2. 提交新功能请求
在终端执行以下命令：
```bash
gh issue create --template "feature_request.md"
```
终端会载入新功能模板，按照提示输入标题并填写您所期望的设计方案及背景信息即可。
