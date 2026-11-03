#include "capture_double_click_action.h"

#include "config_value.h"

#include <QJsonValue>

#include <utility>

namespace {

/// @brief 返回配置对象中的双击动作字段。
/// @param root 应用配置根对象。
/// @return 配置字段值，缺失时返回 undefined。
QJsonValue doubleClickActionValue(const QJsonObject &root)
{
    const QJsonObject capture =
        markshot::config::firstNonEmptyObjectValue(root,
                                                   {QStringLiteral("capture"),
                                                    QStringLiteral("screenshot"),
                                                    QStringLiteral("screenCapture")});
    const QJsonValue nestedValue =
        markshot::config::valueForKeys(capture,
                                       {QStringLiteral("doubleClickAction"),
                                        QStringLiteral("doubleClick"),
                                        QStringLiteral("selectionDoubleClickAction"),
                                        QStringLiteral("doubleClickGesture")});
    if (!nestedValue.isUndefined()) {
        return nestedValue;
    }

    return markshot::config::valueForKeys(root,
                                          {QStringLiteral("captureDoubleClickAction"),
                                           QStringLiteral("doubleClickAction")});
}

}  // namespace

namespace markshot {

CaptureDoubleClickAction defaultCaptureDoubleClickAction()
{
    return CaptureDoubleClickAction::Copy;
}

std::optional<CaptureDoubleClickAction> captureDoubleClickActionFromText(QString value)
{
    return captureClickActionFromText(std::move(value));
}

CaptureDoubleClickAction captureDoubleClickActionFromConfigRoot(const QJsonObject &root)
{
    const QJsonValue value = doubleClickActionValue(root);
    if (value.isString()) {
        if (const std::optional<CaptureDoubleClickAction> action =
                captureDoubleClickActionFromText(value.toString())) {
            return *action;
        }
    }
    // 布尔 false 视为关闭双击手势,方便手工编辑配置的用户直接禁用
    if (value.isBool() && !value.toBool()) {
        return CaptureDoubleClickAction::None;
    }
    return defaultCaptureDoubleClickAction();
}

QString captureDoubleClickActionName(CaptureDoubleClickAction action)
{
    return captureClickActionName(action);
}

}  // namespace markshot
