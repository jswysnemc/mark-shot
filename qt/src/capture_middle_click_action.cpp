#include "capture_middle_click_action.h"

#include "config_value.h"

#include <QJsonValue>

#include <utility>

namespace {

/// @brief 返回配置对象中的中键单击动作字段。
/// @param root 应用配置根对象。
/// @return 配置字段值，缺失时返回 undefined。
QJsonValue middleClickActionValue(const QJsonObject &root)
{
    const QJsonObject capture =
        markshot::config::firstNonEmptyObjectValue(root,
                                                   {QStringLiteral("capture"),
                                                    QStringLiteral("screenshot"),
                                                    QStringLiteral("screenCapture")});
    const QJsonValue nestedValue =
        markshot::config::valueForKeys(capture,
                                       {QStringLiteral("middleClickAction"),
                                        QStringLiteral("middleClick"),
                                        QStringLiteral("selectionMiddleClickAction")});
    if (!nestedValue.isUndefined()) {
        return nestedValue;
    }

    return markshot::config::valueForKeys(root,
                                          {QStringLiteral("captureMiddleClickAction"),
                                           QStringLiteral("middleClickAction")});
}

}  // namespace

namespace markshot {

CaptureClickAction defaultCaptureMiddleClickAction()
{
    return CaptureClickAction::None;
}

std::optional<CaptureClickAction> captureMiddleClickActionFromText(QString value)
{
    // 1. 中键单击默认用于平移和切换全屏标注，这些名称都表示保留内置行为
    const QString text = config::normalizedKey(value);
    if (text == QStringLiteral("default")
        || text == QStringLiteral("zoom")
        || text == QStringLiteral("pan")
        || text == QStringLiteral("builtin")) {
        return CaptureClickAction::None;
    }

    // 2. 其余名称与双击动作共用同一张动作表
    return captureClickActionFromText(std::move(value));
}

CaptureClickAction captureMiddleClickActionFromConfigRoot(const QJsonObject &root)
{
    const QJsonValue value = middleClickActionValue(root);
    if (value.isString()) {
        if (const std::optional<CaptureClickAction> action =
                captureMiddleClickActionFromText(value.toString())) {
            return *action;
        }
    }
    return defaultCaptureMiddleClickAction();
}

QString captureMiddleClickActionName(CaptureClickAction action)
{
    if (action == CaptureClickAction::None) {
        return QStringLiteral("default");
    }
    return captureClickActionName(action);
}

}  // namespace markshot
