#pragma once

#include <QJsonArray>
#include <QRect>
#include <QVector>

namespace markshot {

/// @brief 截图选区历史的最大保留条数。
inline constexpr int kSelectionHistoryLimit = 10;

/**
 * 把选区历史序列化为 JSON 数组。
 *
 * @param history 选区列表（全局逻辑坐标，最新在前）。
 * @return JSON 数组，元素形如 {"x":0,"y":0,"w":800,"h":600}。
 */
QJsonArray selectionHistoryToJsonArray(const QVector<QRect> &history);

/**
 * 从 JSON 数组解析选区历史。
 *
 * @param array JSON 数组。
 * @param limit 最大条数。
 * @return 过滤掉无效条目后的选区列表。
 */
QVector<QRect> selectionHistoryFromJsonArray(const QJsonArray &array,
                                             int limit = kSelectionHistoryLimit);

/**
 * 在历史头部插入一条选区。
 *
 * 与现有条目重复时把该条提升到头部，超出上限时丢弃最旧的条目。
 *
 * @param history 现有历史（最新在前）。
 * @param selection 新选区（全局逻辑坐标）。
 * @param limit 最大条数。
 * @return 更新后的历史。
 */
QVector<QRect> selectionHistoryWithSelection(const QVector<QRect> &history,
                                             const QRect &selection,
                                             int limit = kSelectionHistoryLimit);

/**
 * 读取持久化的截图选区历史。
 *
 * 历史保存在应用配置文件的 capture.selectionHistory 键下，跨进程共享，
 * 因此每次由快捷键拉起的新截图进程都能浏览此前的选区。
 *
 * @param limit 最大条数。
 * @return 选区列表（全局逻辑坐标，最新在前）。
 */
QVector<QRect> readSelectionHistory(int limit = kSelectionHistoryLimit);

/**
 * 把一条选区写入持久化历史。
 *
 * @param selection 选区（全局逻辑坐标）。
 * @return 写入成功返回 true。
 */
bool rememberSelection(const QRect &selection);

}  // namespace markshot
