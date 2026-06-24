#pragma once

#include <QColor>
#include <QJsonObject>
#include <QString>
#include <QVector>

#include <optional>

namespace markshot::ui {

inline constexpr int kColorHistoryLimit = 7;

/// @brief 将颜色转换成历史记录配置文本。
/// @param color 颜色。
/// @return #RRGGBBAA 格式文本，无效颜色返回空字符串。
QString colorHistoryConfigName(const QColor &color);

/// @brief 从历史记录配置文本解析颜色。
/// @param text 配置文本。
/// @return 解析成功的颜色。
std::optional<QColor> colorHistoryColorFromString(const QString &text);

/// @brief 生成加入新颜色后的历史记录。
/// @param history 原历史颜色。
/// @param color 新颜色。
/// @param limit 最大历史数量。
/// @return 去重并限制数量后的历史颜色。
QVector<QColor> colorHistoryWithRememberedColor(const QVector<QColor> &history,
                                                const QColor &color,
                                                int limit = kColorHistoryLimit);

/// @brief 从应用配置根对象读取历史取色。
/// @param root 应用配置根对象。
/// @param limit 最大历史数量。
/// @return 历史颜色列表。
QVector<QColor> colorHistoryFromConfigRoot(const QJsonObject &root, int limit = kColorHistoryLimit);

/// @brief 从应用配置文件读取历史取色。
/// @param limit 最大历史数量。
/// @return 历史颜色列表。
QVector<QColor> readColorHistory(int limit = kColorHistoryLimit);

/// @brief 写入历史取色。
/// @param history 历史颜色列表。
/// @param error 失败时写入错误信息。
/// @return 写入成功返回 true，否则返回 false。
bool writeColorHistory(const QVector<QColor> &history, QString *error = nullptr);

/// @brief 将颜色记录到历史取色。
/// @param color 需要记录的颜色。
/// @param error 失败时写入错误信息。
/// @return 写入成功返回 true，否则返回 false。
bool rememberColor(const QColor &color, QString *error = nullptr);

}  // namespace markshot::ui
