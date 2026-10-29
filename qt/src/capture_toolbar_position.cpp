#include "capture_toolbar_position.h"

#include "config_value.h"

#include <QJsonValue>

#include <utility>

namespace {

/// @brief 返回配置对象中的工具栏摆放字段。
/// @param root 应用配置根对象。
/// @return 配置字段值，缺失时返回 undefined。
QJsonValue toolbarPositionValue(const QJsonObject &root)
{
    const QJsonObject capture =
        markshot::config::firstNonEmptyObjectValue(root,
                                                   {QStringLiteral("capture"),
                                                    QStringLiteral("screenshot"),
                                                    QStringLiteral("screenCapture")});
    const QJsonValue nestedValue =
        markshot::config::valueForKeys(capture,
                                       {QStringLiteral("toolbarPosition"),
                                        QStringLiteral("toolbarDock"),
                                        QStringLiteral("toolbarPlacement")});
    if (!nestedValue.isUndefined()) {
        return nestedValue;
    }

    return markshot::config::valueForKeys(root, {QStringLiteral("captureToolbarPosition")});
}

}  // namespace

namespace markshot {

CaptureToolbarPosition defaultCaptureToolbarPosition()
{
    return CaptureToolbarPosition::FollowSelection;
}

std::optional<CaptureToolbarPosition> captureToolbarPositionFromText(QString value)
{
    const QString text = markshot::config::normalizedKey(std::move(value));
    if (text.isEmpty()) {
        return std::nullopt;
    }

    if (text == QStringLiteral("selection")
        || text == QStringLiteral("followselection")
        || text == QStringLiteral("follow")
        || text == QStringLiteral("auto")) {
        return CaptureToolbarPosition::FollowSelection;
    }

    if (text == QStringLiteral("bottom")
        || text == QStringLiteral("screenbottom")
        || text == QStringLiteral("dockbottom")) {
        return CaptureToolbarPosition::ScreenBottom;
    }

    if (text == QStringLiteral("right")
        || text == QStringLiteral("screenright")
        || text == QStringLiteral("dockright")) {
        return CaptureToolbarPosition::ScreenRight;
    }

    if (text == QStringLiteral("top")
        || text == QStringLiteral("screentop")
        || text == QStringLiteral("docktop")) {
        return CaptureToolbarPosition::ScreenTop;
    }

    return std::nullopt;
}

CaptureToolbarPosition captureToolbarPositionFromConfigRoot(const QJsonObject &root)
{
    const QJsonValue value = toolbarPositionValue(root);
    if (value.isString()) {
        if (const std::optional<CaptureToolbarPosition> position =
                captureToolbarPositionFromText(value.toString())) {
            return *position;
        }
    }
    return defaultCaptureToolbarPosition();
}

QString captureToolbarPositionName(CaptureToolbarPosition position)
{
    switch (position) {
    case CaptureToolbarPosition::FollowSelection:
        return QStringLiteral("selection");
    case CaptureToolbarPosition::ScreenBottom:
        return QStringLiteral("bottom");
    case CaptureToolbarPosition::ScreenRight:
        return QStringLiteral("right");
    case CaptureToolbarPosition::ScreenTop:
        return QStringLiteral("top");
    }
    return QStringLiteral("selection");
}

}  // namespace markshot
