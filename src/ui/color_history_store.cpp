#include "ui/color_history_store.h"

#include "app_config_store.h"

#include <QJsonArray>
#include <QJsonValue>

#include <algorithm>

namespace markshot::ui {
namespace {

/// @brief 判断两个颜色是否为同一个 RGBA 值。
/// @param left 左侧颜色。
/// @param right 右侧颜色。
/// @return RGBA 完全一致时返回 true。
bool sameRgba(const QColor &left, const QColor &right)
{
    return left.rgba() == right.rgba();
}

/// @brief 将历史颜色转换成 JSON 数组。
/// @param history 历史颜色列表。
/// @return JSON 数组。
QJsonArray colorHistoryToJsonArray(const QVector<QColor> &history)
{
    QJsonArray array;
    for (const QColor &color : history) {
        const QString value = colorHistoryConfigName(color);
        if (!value.isEmpty()) {
            array.append(value);
        }
    }
    return array;
}

/// @brief 归一化历史颜色列表。
/// @param history 原历史颜色。
/// @param limit 最大历史数量。
/// @return 去重并限制数量后的历史颜色。
QVector<QColor> normalizedColorHistory(const QVector<QColor> &history, int limit)
{
    QVector<QColor> result;
    if (limit <= 0) {
        return result;
    }
    for (const QColor &item : history) {
        if (!item.isValid()) {
            continue;
        }
        if (std::any_of(result.cbegin(), result.cend(), [&item](const QColor &existing) {
                return sameRgba(existing, item);
            })) {
            continue;
        }
        result.push_back(item);
        if (result.size() >= limit) {
            break;
        }
    }
    return result;
}

}  // namespace

QString colorHistoryConfigName(const QColor &color)
{
    if (!color.isValid()) {
        return {};
    }
    return QStringLiteral("#%1%2%3%4")
        .arg(color.red(), 2, 16, QLatin1Char('0'))
        .arg(color.green(), 2, 16, QLatin1Char('0'))
        .arg(color.blue(), 2, 16, QLatin1Char('0'))
        .arg(color.alpha(), 2, 16, QLatin1Char('0'))
        .toUpper();
}

std::optional<QColor> colorHistoryColorFromString(const QString &text)
{
    QString value = text.trimmed();
    if (value.isEmpty()) {
        return std::nullopt;
    }
    if (!value.startsWith(QLatin1Char('#'))) {
        value.prepend(QLatin1Char('#'));
    }

    int alpha = 255;
    QColor color;
    if (value.length() == 9) {
        bool ok = false;
        alpha = value.mid(7, 2).toInt(&ok, 16);
        if (!ok) {
            return std::nullopt;
        }
        color = QColor(value.left(7));
    } else {
        color = QColor(value);
    }
    if (!color.isValid()) {
        return std::nullopt;
    }
    color.setAlpha(alpha);
    return color;
}

QVector<QColor> colorHistoryWithRememberedColor(const QVector<QColor> &history, const QColor &color, int limit)
{
    QVector<QColor> result;
    if (!color.isValid() || limit <= 0) {
        return result;
    }

    result.push_back(color);
    for (const QColor &item : history) {
        if (!item.isValid()) {
            continue;
        }
        if (std::any_of(result.cbegin(), result.cend(), [&item](const QColor &existing) {
                return sameRgba(existing, item);
            })) {
            continue;
        }
        result.push_back(item);
        if (result.size() >= limit) {
            break;
        }
    }
    return result;
}

QVector<QColor> colorHistoryFromConfigRoot(const QJsonObject &root, int limit)
{
    QVector<QColor> parsedColors;
    if (limit <= 0) {
        return parsedColors;
    }

    const QJsonValue colorPickerValue = root.value(QStringLiteral("colorPicker"));
    const QJsonArray history = colorPickerValue.isObject()
        ? colorPickerValue.toObject().value(QStringLiteral("history")).toArray()
        : QJsonArray();
    for (const QJsonValue &item : history) {
        if (!item.isString()) {
            continue;
        }
        const std::optional<QColor> color = colorHistoryColorFromString(item.toString());
        if (!color.has_value()) {
            continue;
        }
        parsedColors.push_back(*color);
    }
    return normalizedColorHistory(parsedColors, limit);
}

QVector<QColor> readColorHistory(int limit)
{
    bool ok = false;
    const QJsonObject root = readAppConfigRoot(&ok);
    return ok ? colorHistoryFromConfigRoot(root, limit) : QVector<QColor>();
}

bool writeColorHistory(const QVector<QColor> &history, QString *error)
{
    return writeAppConfigValue({QStringLiteral("colorPicker"), QStringLiteral("history")},
                               colorHistoryToJsonArray(normalizedColorHistory(history, kColorHistoryLimit)),
                               error);
}

bool rememberColor(const QColor &color, QString *error)
{
    if (!color.isValid()) {
        return true;
    }
    const QVector<QColor> history = colorHistoryWithRememberedColor(readColorHistory(), color);
    return writeAppConfigValue({QStringLiteral("colorPicker"), QStringLiteral("history")},
                               colorHistoryToJsonArray(history),
                               error);
}

}  // namespace markshot::ui
