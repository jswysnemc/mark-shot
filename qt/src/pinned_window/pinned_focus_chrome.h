#pragma once

#include <QColor>

namespace markshot::shot {

/// 失焦贴图边框相对配置颜色的默认不透明度
inline constexpr double kPinnedInactiveBorderOpacityDefault = 0.4;

/// @brief 贴图在当前焦点状态下使用的边框颜色和阴影参数。
struct PinnedFocusChromeStyle {
    /// @brief 边框颜色，配置颜色无效时保持无效。
    QColor borderColor;
    /// @brief 阴影叠加层数，每层向外多扩展 1px。
    int shadowLayers = 0;
    /// @brief 每层阴影的透明度，0 到 255。
    int shadowLayerAlpha = 0;
    /// @brief 阴影整体向下的偏移量。
    qreal shadowOffsetY = 0.0;
};

/**
 * 按焦点状态选择贴图边框颜色和阴影参数。
 * @param focused 贴图是否按聚焦外观绘制。
 * @param configuredBorderColor 用户配置的边框颜色。
 * @param inactiveBorderOpacity 失焦时边框颜色的不透明度系数，超出 0 到 1 时截断。
 * @return 聚焦时为配置颜色原样和较深阴影；失焦时为降低透明度的配置颜色和较浅阴影。
 */
PinnedFocusChromeStyle pinnedFocusChromeStyle(bool focused,
                                              const QColor &configuredBorderColor,
                                              double inactiveBorderOpacity);

/**
 * 判断贴图是否按聚焦外观绘制。
 * @param windowActive 窗口当前是否为活动窗口。
 * @param hovered 指针当前是否停留在贴图上。
 * @param hoverCountsAsFocus 窗口无法可靠获得活动状态时，是否把悬停视为聚焦。
 * @return 活动窗口，或允许悬停替代且正在悬停时返回 true。
 */
bool pinnedShowsFocusChrome(bool windowActive, bool hovered, bool hoverCountsAsFocus);

}  // namespace markshot::shot
