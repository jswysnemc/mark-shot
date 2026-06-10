#include "capture_cursor_policy.h"

#include "config_value.h"

#include <QJsonValue>

#include <optional>

namespace {

/// @brief 返回配置对象中的鼠标包含字段。
/// @param root 应用配置根对象。
/// @return 配置字段值，缺失时返回 undefined。
QJsonValue includeCursorValue(const QJsonObject &root)
{
    const QJsonObject capture =
        markshot::config::firstNonEmptyObjectValue(root,
                                                   {QStringLiteral("capture"),
                                                    QStringLiteral("screenshot"),
                                                    QStringLiteral("screenCapture")});
    const QJsonValue nestedValue =
        markshot::config::valueForKeys(capture,
                                       {QStringLiteral("includeCursor"),
                                        QStringLiteral("includeMouse"),
                                        QStringLiteral("includePointer"),
                                        QStringLiteral("showCursor"),
                                        QStringLiteral("showMouse"),
                                        QStringLiteral("captureCursor"),
                                        QStringLiteral("captureMouse")});
    if (!nestedValue.isUndefined() && !nestedValue.isNull()) {
        return nestedValue;
    }

    return markshot::config::valueForKeys(root,
                                          {QStringLiteral("captureIncludeCursor"),
                                           QStringLiteral("captureIncludeMouse"),
                                           QStringLiteral("includeCursor"),
                                           QStringLiteral("includeMouse"),
                                           QStringLiteral("includePointer")});
}

}  // namespace

namespace markshot {

bool defaultCaptureIncludeCursor()
{
    return false;
}

bool captureIncludeCursorFromConfigRoot(const QJsonObject &root)
{
    const std::optional<bool> value = config::boolValue(includeCursorValue(root));
    return value.value_or(defaultCaptureIncludeCursor());
}

}  // namespace markshot
