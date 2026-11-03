#include "pinned_window/pinned_focus_chrome.h"

#include <algorithm>
#include <cmath>

namespace markshot::shot {
namespace {

// 聚焦阴影沿用原有贴图阴影参数，失焦阴影更窄更淡
constexpr int kFocusedShadowLayers = 16;
constexpr int kFocusedShadowLayerAlpha = 7;
constexpr qreal kFocusedShadowOffsetY = 7.0;
constexpr int kInactiveShadowLayers = 10;
constexpr int kInactiveShadowLayerAlpha = 5;
constexpr qreal kInactiveShadowOffsetY = 4.0;

}  // namespace

PinnedFocusChromeStyle pinnedFocusChromeStyle(bool focused,
                                              const QColor &configuredBorderColor,
                                              double inactiveBorderOpacity)
{
    PinnedFocusChromeStyle style;
    style.borderColor = configuredBorderColor;
    if (focused) {
        // 1. 聚焦时边框保持配置颜色原样，阴影更深
        style.shadowLayers = kFocusedShadowLayers;
        style.shadowLayerAlpha = kFocusedShadowLayerAlpha;
        style.shadowOffsetY = kFocusedShadowOffsetY;
        return style;
    }

    // 2. 失焦时只降低边框透明度，颜色和粗细不变，阴影更浅
    style.shadowLayers = kInactiveShadowLayers;
    style.shadowLayerAlpha = kInactiveShadowLayerAlpha;
    style.shadowOffsetY = kInactiveShadowOffsetY;
    if (style.borderColor.isValid()) {
        const double opacity = std::isfinite(inactiveBorderOpacity)
            ? std::clamp(inactiveBorderOpacity, 0.0, 1.0)
            : kPinnedInactiveBorderOpacityDefault;
        style.borderColor.setAlphaF(static_cast<float>(configuredBorderColor.alphaF() * opacity));
    }
    return style;
}

bool pinnedShowsFocusChrome(bool windowActive, bool hovered, bool hoverCountsAsFocus)
{
    return windowActive || (hoverCountsAsFocus && hovered);
}

}  // namespace markshot::shot
