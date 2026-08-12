#include "selection_history.h"

#include "app_config_store.h"

#include <QJsonObject>
#include <QJsonValue>

namespace markshot {
namespace {

/// @brief 配置文件中选区历史的键路径：capture.selectionHistory。
const QStringList kSelectionHistoryConfigPath = {QStringLiteral("capture"),
                                                 QStringLiteral("selectionHistory")};

/**
 * 从 JSON 对象解析一条选区。
 *
 * @param value JSON 值。
 * @return 选区矩形；字段缺失或尺寸非法时返回空矩形。
 */
QRect selectionFromJsonValue(const QJsonValue &value)
{
    if (!value.isObject()) {
        return {};
    }
    const QJsonObject object = value.toObject();
    const int width = object.value(QStringLiteral("w")).toInt(0);
    const int height = object.value(QStringLiteral("h")).toInt(0);
    if (width <= 0 || height <= 0) {
        return {};
    }
    return QRect(object.value(QStringLiteral("x")).toInt(0),
                 object.value(QStringLiteral("y")).toInt(0),
                 width,
                 height);
}

}  // namespace

QJsonArray selectionHistoryToJsonArray(const QVector<QRect> &history)
{
    QJsonArray array;
    for (const QRect &selection : history) {
        if (selection.isEmpty()) {
            continue;
        }
        QJsonObject object;
        object.insert(QStringLiteral("x"), selection.x());
        object.insert(QStringLiteral("y"), selection.y());
        object.insert(QStringLiteral("w"), selection.width());
        object.insert(QStringLiteral("h"), selection.height());
        array.append(object);
    }
    return array;
}

QVector<QRect> selectionHistoryFromJsonArray(const QJsonArray &array, int limit)
{
    QVector<QRect> history;
    if (limit <= 0) {
        return history;
    }
    for (const QJsonValue &value : array) {
        const QRect selection = selectionFromJsonValue(value);
        if (selection.isEmpty() || history.contains(selection)) {
            continue;
        }
        history.append(selection);
        if (history.size() >= limit) {
            break;
        }
    }
    return history;
}

QVector<QRect> selectionHistoryWithSelection(const QVector<QRect> &history,
                                             const QRect &selection,
                                             int limit)
{
    QVector<QRect> result;
    if (selection.isEmpty() || limit <= 0) {
        return history;
    }

    result.append(selection.normalized());
    for (const QRect &item : history) {
        if (item.isEmpty() || result.contains(item)) {
            continue;
        }
        result.append(item);
        if (result.size() >= limit) {
            break;
        }
    }
    return result;
}

QVector<QRect> readSelectionHistory(int limit)
{
    bool ok = false;
    const QJsonObject root = readAppConfigRoot(&ok);
    if (!ok) {
        return {};
    }
    const QJsonValue captureValue = root.value(QStringLiteral("capture"));
    if (!captureValue.isObject()) {
        return {};
    }
    return selectionHistoryFromJsonArray(
        captureValue.toObject().value(QStringLiteral("selectionHistory")).toArray(), limit);
}

bool rememberSelection(const QRect &selection)
{
    if (selection.isEmpty()) {
        return true;
    }
    const QVector<QRect> history =
        selectionHistoryWithSelection(readSelectionHistory(), selection);
    return writeAppConfigValue(kSelectionHistoryConfigPath,
                               selectionHistoryToJsonArray(history));
}

}  // namespace markshot
