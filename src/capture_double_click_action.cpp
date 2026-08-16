#include "capture_double_click_action.h"

#include "config_value.h"

#include <QJsonValue>

#include <utility>

namespace {

/// @brief 归一化用户配置字符串。
/// @param value 原始配置字符串。
/// @return 去除分隔符并转小写后的字符串。
QString normalizedActionText(QString value)
{
    return markshot::config::normalizedKey(std::move(value));
}

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
    const QString text = normalizedActionText(std::move(value));
    if (text.isEmpty()) {
        return std::nullopt;
    }

    if (text == QStringLiteral("none")
        || text == QStringLiteral("disabled")
        || text == QStringLiteral("off")
        || text == QStringLiteral("noop")
        || text == QStringLiteral("nothing")) {
        return CaptureDoubleClickAction::None;
    }

    if (text == QStringLiteral("copy")
        || text == QStringLiteral("copyandclose")
        || text == QStringLiteral("copyclipboard")
        || text == QStringLiteral("clipboard")) {
        return CaptureDoubleClickAction::Copy;
    }

    if (text == QStringLiteral("save")
        || text == QStringLiteral("saveandclose")
        || text == QStringLiteral("quicksave")
        || text == QStringLiteral("savedefault")) {
        return CaptureDoubleClickAction::Save;
    }

    if (text == QStringLiteral("saveas")
        || text == QStringLiteral("savedialog")
        || text == QStringLiteral("saveasdialog")) {
        return CaptureDoubleClickAction::SaveAs;
    }

    if (text == QStringLiteral("pin")
        || text == QStringLiteral("pintoscreen")
        || text == QStringLiteral("pinned")) {
        return CaptureDoubleClickAction::Pin;
    }

    if (text == QStringLiteral("cancel")
        || text == QStringLiteral("close")
        || text == QStringLiteral("abort")
        || text == QStringLiteral("discard")) {
        return CaptureDoubleClickAction::Cancel;
    }

    return std::nullopt;
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
    switch (action) {
    case CaptureDoubleClickAction::None:
        return QStringLiteral("none");
    case CaptureDoubleClickAction::Copy:
        return QStringLiteral("copy");
    case CaptureDoubleClickAction::Save:
        return QStringLiteral("save");
    case CaptureDoubleClickAction::SaveAs:
        return QStringLiteral("save-as");
    case CaptureDoubleClickAction::Pin:
        return QStringLiteral("pin");
    case CaptureDoubleClickAction::Cancel:
        return QStringLiteral("cancel");
    }
    return QStringLiteral("copy");
}

}  // namespace markshot
