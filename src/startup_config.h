#pragma once

#include "debug_log.h"
#include "shot_window.h"
#include "ui/theme.h"

#include <QColor>
#include <QString>

#include <optional>

namespace markshot {

enum class DefaultColorSource {
    BuiltIn,
    Config,
    CommandLine,
};

struct DefaultTools {
    ShotWindow::Tool normal = ShotWindow::Tool::Move;
    ShotWindow::Tool fullscreen = ShotWindow::Tool::Move;
    ShotWindow::Tool file = ShotWindow::Tool::Move;
    QColor color = markshot::theme::kDefaultAnnotationColor;
    DefaultColorSource colorSource = DefaultColorSource::BuiltIn;
};

struct DebugRuntimeConfig {
    bool enabled = markshot::debugEnabled();
    QString logPath = markshot::debugLogPath();
};

/// @brief 展开配置路径中的用户目录写法。
/// @param path 原始配置路径。
/// @return 展开后的路径。
QString expandedConfigPath(QString path);

/// @brief 从字符串解析默认标注颜色。
/// @param value 颜色字符串。
/// @return 解析出的颜色。
std::optional<QColor> colorFromString(QString value);

/// @brief 应用配置文件中的进程环境变量。
/// @return 无返回值。
void applyConfiguredEnvironment();

/// @brief 读取调试运行时配置。
/// @return 调试运行时配置。
DebugRuntimeConfig configuredDebugRuntimeConfig();

/// @brief 读取默认工具和默认颜色配置。
/// @param warning 输出配置警告。
/// @return 默认工具配置。
DefaultTools configuredDefaultTools(QString *warning);

/// @brief 判断指定来源的默认颜色是否应覆盖持久化颜色。
/// @param colorSource 默认颜色来源。
/// @param annotationStateExists 标注状态文件是否存在。
/// @return 需要应用默认颜色时返回 true。
constexpr bool shouldApplyDefaultColor(DefaultColorSource colorSource, bool annotationStateExists)
{
    return colorSource == DefaultColorSource::CommandLine
        || (colorSource == DefaultColorSource::Config && !annotationStateExists);
}

/// @brief 判断默认颜色是否应覆盖窗口中已经加载的持久化颜色。
/// @param tools 默认工具配置。
/// @return 需要应用默认颜色时返回 true。
bool shouldApplyDefaultColor(const DefaultTools &tools);

/// @brief 禁用 Qt 默认门户服务以便应用自行注册门户会话。
/// @return 无返回值。
void disableQtPortalServicesForHostApp();

}  // namespace markshot
