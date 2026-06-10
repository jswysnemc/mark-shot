#include "kde_capture_config.h"

#include "app_config_store.h"
#include "config_value.h"

#include <QJsonValue>

#include <optional>

namespace {

/// @brief 返回 capture.wayland.kde.kwinScreenshot.enabled 配置字段。
/// @param root 应用配置根对象。
/// @return 配置字段值，缺失时返回 undefined。
QJsonValue kwinScreenshotEnabledValue(const QJsonObject &root)
{
    const QJsonObject capture = markshot::config::objectValue(root, QStringLiteral("capture"));
    const QJsonObject wayland = markshot::config::objectValue(capture, QStringLiteral("wayland"));
    const QJsonObject kde = markshot::config::objectValue(wayland, QStringLiteral("kde"));
    const QJsonObject kwinScreenshot =
        markshot::config::objectValue(kde, QStringLiteral("kwinScreenshot"));
    return kwinScreenshot.value(QStringLiteral("enabled"));
}

}  // namespace

namespace markshot {

bool defaultKdeKWinScreenshotEnabled()
{
    return true;
}

bool kdeKWinScreenshotEnabledFromConfigRoot(const QJsonObject &root)
{
    const std::optional<bool> value = config::boolValue(kwinScreenshotEnabledValue(root));
    return value.value_or(defaultKdeKWinScreenshotEnabled());
}

bool configuredKdeKWinScreenshotEnabled()
{
    bool ok = false;
    const QJsonObject root = readAppConfigRoot(&ok);
    if (!ok) {
        return defaultKdeKWinScreenshotEnabled();
    }
    return kdeKWinScreenshotEnabledFromConfigRoot(root);
}

}  // namespace markshot
