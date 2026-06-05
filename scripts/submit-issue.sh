#!/usr/bin/env bash

# 脚本功能：引导用户通过 GitHub CLI (gh) 提交 Issue 并自动收集系统环境信息

# 检查 gh 工具是否安装
if ! command -v gh &> /dev/null; then
    echo "未检测到 gh (GitHub CLI) 工具。请先安装。安装方法可参考: https://cli.github.com"
    exit 1
fi

# 检查 gh 登录状态
if ! gh auth status &> /dev/null; then
    echo "gh 工具未登录。请先运行 'gh auth login' 登录您的 GitHub 账号。"
    exit 1
fi

# 获取当前项目根目录
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "请选择提交的 Issue 类型:"
echo "1) Bug Report (Bug 报告，自动收集环境信息)"
echo "2) Feature Request (新功能请求)"
read -r -p "请输入序号 (1 或 2): " choice

if [ "$choice" = "1" ]; then
    TEMPLATE_FILE="$REPO_DIR/.github/ISSUE_TEMPLATE/bug_report.md"
    if [ ! -f "$TEMPLATE_FILE" ]; then
        echo "未找到 Bug 报告模板文件: $TEMPLATE_FILE"
        exit 1
    fi

    echo "正在收集系统环境信息..."
    
    OS_NAME="未知"
    if [ -f /etc/os-release ]; then
        OS_NAME=$(grep -oP '(?<=^PRETTY_NAME=")[^"]+' /etc/os-release || grep -oP '(?<=^PRETTY_NAME=)[^[:space:]]+' /etc/os-release)
    fi

    DESKTOP_ENV="${XDG_CURRENT_DESKTOP:-未知}"
    DISPLAY_TYPE="${XDG_SESSION_TYPE:-未知}"
    
    # 尝试运行 mark-shot 获取版本
    MS_VERSION="未知 (未编译或未安装)"
    if command -v mark-shot &> /dev/null; then
        MS_VERSION=$(mark-shot --version 2>&1)
    elif [ -f "$REPO_DIR/build/mark-shot" ]; then
        MS_VERSION=$("$REPO_DIR/build/mark-shot" --version 2>&1)
    fi

    # 获取 Qt 版本
    QT_VERSION="未知"
    if command -v qmake6 &> /dev/null; then
        QT_VERSION=$(qmake6 --version | grep -oP '(?<=Qt version )[0-9.]+')
    elif command -v qmake &> /dev/null; then
        QT_VERSION=$(qmake --version | grep -oP '(?<=Qt version )[0-9.]+')
    fi

    # 创建临时文件来填充模板
    TEMP_ISSUE=$(mktemp)
    
    # 读取模板并替换环境占位符
    sed \
        -e "s/- Mark Shot 版本:.*/- Mark Shot 版本: $MS_VERSION/" \
        -e "s/- 操作系统 (OS):.*/- 操作系统 (OS): $OS_NAME/" \
        -e "s/- 窗口管理器 (Window Manager,.*) \/ 桌面环境 (Desktop,.*):.*/- 窗口管理器 (Window Manager) \/ 桌面环境 (Desktop): $DESKTOP_ENV/" \
        -e "s/- 显示服务器类型 (Display Server):.*/- 显示服务器类型 (Display Server): $DISPLAY_TYPE/" \
        -e "s/- Qt 版本 (Qt Version):.*/- Qt 版本 (Qt Version): $QT_VERSION/" \
        "$TEMPLATE_FILE" > "$TEMP_ISSUE"

    echo "环境信息收集完毕。正在启动 gh 创建 Bug 报告..."
    # 调用 gh 并将预填环境信息的临时文件作为 body 传入
    gh issue create --body-file "$TEMP_ISSUE" --label "bug"
    
    rm -f "$TEMP_ISSUE"

elif [ "$choice" = "2" ]; then
    TEMPLATE_FILE="$REPO_DIR/.github/ISSUE_TEMPLATE/feature_request.md"
    if [ ! -f "$TEMPLATE_FILE" ]; then
        echo "未找到新功能请求模板文件: $TEMPLATE_FILE"
        exit 1
    fi
    echo "正在启动 gh 创建新功能请求..."
    gh issue create --body-file "$TEMPLATE_FILE" --label "enhancement"
else
    echo "输入无效。已取消操作。"
    exit 1
fi
