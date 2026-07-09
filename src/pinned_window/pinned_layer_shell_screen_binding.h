#pragma once

#include <QRect>
#include <QVector>

namespace markshot::shot {

struct PinnedLayerShellScreenBinding {
    int targetScreenIndex = -1;
    bool screenChanged = false;
};

/**
 * 解析贴图几何对应的 layer-shell 输出绑定。
 * @param geometry 贴图全局逻辑几何。
 * @param screenGeometries 当前屏幕全局逻辑几何列表。
 * @param boundScreenIndex 当前 surface 绑定的屏幕索引，未绑定时为 -1。
 * @return 目标屏幕索引及是否需要重建 surface。
 */
PinnedLayerShellScreenBinding resolvePinnedLayerShellScreenBinding(
    QRect geometry,
    const QVector<QRect> &screenGeometries,
    int boundScreenIndex);

}  // namespace markshot::shot
