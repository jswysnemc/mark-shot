#include "pinned_window/pinned_image_window.h"

#include "pinned_window/pinned_focus_chrome.h"
#include "pinned_window/pinned_non_window_chrome.h"

#include <QEvent>
#include <QPen>

namespace markshot::shot {
namespace {

/**
 * 判断贴图当前是否按聚焦外观绘制边框和阴影。
 * 【贴图】【焦点外观】X11、Windows、GNOME 和 KDE 的贴图是普通顶层窗口，点击后由窗口管理器激活，
 * 只按活动窗口判断，新建贴图不抢焦点，因此先显示失焦外观。layer-shell 贴图的键盘焦点为 on-demand，
 * 是否以及何时交给贴图由合成器决定，部分合成器从不交出焦点；这里把指针悬停也视为聚焦，
 * 避免贴图一直停在失焦外观。
 * @param window 贴图窗口。
 * @param hovered 指针当前是否停留在贴图上。
 * @return 需要聚焦外观时返回 true；非窗口贴图功能关闭时始终返回 true，保持原有外观。
 */
bool pinnedChromeFocused(const QWidget *window, bool hovered)
{
    if (!FEATURE_PINNED_NON_WINDOW_CHROME_ENABLED) {
        return true;
    }
    const bool layerShell = window->property("markShotPinnedLayerShellActive").toBool();
    return pinnedShowsFocusChrome(window->isActiveWindow(), hovered, layerShell);
}

}  // namespace

void PinnedImageWindow::applyPinnedNonWindowChromeIfEnabled()
{
    if (!FEATURE_PINNED_NON_WINDOW_CHROME_ENABLED) {
        return;
    }
    // 1. 只避免激活。Qt::Tool 会变成截图覆盖层的附属窗口，覆盖层关闭时贴图一起消失
    setAttribute(Qt::WA_ShowWithoutActivating, true);
}

void PinnedImageWindow::updatePinnedNonWindowHoverIfEnabled(QEvent *event)
{
    if (!FEATURE_PINNED_NON_WINDOW_CHROME_ENABLED || !event) {
        return;
    }
    const QEvent::Type type = event->type();
    // 1. 【贴图】【焦点外观】激活状态变化只重绘边框和阴影，不改几何和层级
    if (type == QEvent::WindowActivate || type == QEvent::WindowDeactivate
        || type == QEvent::ActivationChange) {
        update();
        return;
    }
    if (type != QEvent::Enter && type != QEvent::Leave && type != QEvent::Hide) {
        return;
    }
    // 2. 记录悬停状态，隐藏时可能收不到离开事件，一并清除
    const bool hovered = type == QEvent::Enter;
    if (m_pinChromeHovered == hovered) {
        return;
    }
    const bool wasFocused = pinnedChromeFocused(this, m_pinChromeHovered);
    m_pinChromeHovered = hovered;
    // 3. 只有悬停改变了聚焦外观时才重绘，普通窗口贴图悬停不触发重绘
    if (pinnedChromeFocused(this, m_pinChromeHovered) != wasFocused) {
        update();
    }
}

bool PinnedImageWindow::paintPinnedNonWindowChromeIfEnabled(QPainter &painter, const QRectF &imageRect)
{
    if (!FEATURE_PINNED_NON_WINDOW_CHROME_ENABLED) {
        return false;
    }
    if (!m_config.borderEnabled || !m_config.borderColor.isValid() || m_config.borderWidth <= 0.0) {
        return true;
    }
    // 1. 【贴图】【焦点外观】粗细保持配置值，失焦时只降低边框颜色的不透明度
    const PinnedFocusChromeStyle style = pinnedFocusChromeStyle(pinnedChromeFocused(this, m_pinChromeHovered),
                                                                m_config.borderColor,
                                                                m_config.inactiveBorderOpacity);
    if (style.borderColor.alpha() <= 0) {
        return true;
    }
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, false);
    painter.setBrush(Qt::NoBrush);
    painter.setPen(QPen(style.borderColor, m_config.borderWidth));
    const qreal inset = m_config.borderWidth / 2.0;
    painter.drawRect(imageRect.adjusted(inset, inset, -inset, -inset));
    painter.restore();
    return true;
}

namespace {

constexpr bool FEATURE_PINNED_SHADOW_CONFIG_ENABLED = true;
constexpr int kPinnedShadowMargin = 28;

}  // namespace

void PinnedImageWindow::applyPinnedShadowLayoutIfEnabled()
{
    if (!FEATURE_PINNED_SHADOW_CONFIG_ENABLED || !m_config.shadowEnabled || m_shadowMargin > 0) {
        return;
    }
    // 1. 【贴图】【阴影边距】向外扩展窗口，图片仍停在原来的屏幕位置
    m_shadowMargin = kPinnedShadowMargin;
    setAttribute(Qt::WA_TranslucentBackground, true);
    setAttribute(Qt::WA_NoSystemBackground, true);
    const QRect expanded = m_logicalGeometry.adjusted(-m_shadowMargin, -m_shadowMargin,
                                                      m_shadowMargin, m_shadowMargin);
    setPinnedGeometry(expanded, true);
}

void PinnedImageWindow::insetPinnedImageForShadowIfEnabled(QRectF *rect) const
{
    if (!FEATURE_PINNED_SHADOW_CONFIG_ENABLED || !m_config.shadowEnabled || m_shadowMargin <= 0 || !rect) {
        return;
    }
    const qreal margin = m_shadowMargin;
    *rect = rect->adjusted(margin, margin, -margin, -margin);
}

void PinnedImageWindow::expandPinnedGeometryForShadowIfEnabled(QPoint *topLeft, QSize *size) const
{
    if (!FEATURE_PINNED_SHADOW_CONFIG_ENABLED || !m_config.shadowEnabled || m_shadowMargin <= 0
        || !topLeft || !size) {
        return;
    }
    *topLeft -= QPoint(m_shadowMargin, m_shadowMargin);
    *size += QSize(m_shadowMargin * 2, m_shadowMargin * 2);
}

void PinnedImageWindow::paintPinnedShadowIfEnabled(QPainter &painter, const QRectF &imageRect) const
{
    if (!FEATURE_PINNED_SHADOW_CONFIG_ENABLED || !m_config.shadowEnabled || m_shadowMargin <= 0
        || imageRect.isEmpty()) {
        return;
    }
    // 1. 【贴图】【焦点外观】聚焦时阴影更深更宽，失焦时更浅更窄
    const PinnedFocusChromeStyle style = pinnedFocusChromeStyle(pinnedChromeFocused(this, m_pinChromeHovered),
                                                                m_config.borderColor,
                                                                m_config.inactiveBorderOpacity);
    painter.save();
    painter.setPen(Qt::NoPen);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setBrush(QColor(0, 0, 0, style.shadowLayerAlpha));
    // 2. 【贴图】【阴影】由外向内叠若干层透明色块，形成柔和投影
    for (int layer = style.shadowLayers; layer >= 1; --layer) {
        const qreal spread = layer;
        const qreal offset = style.shadowOffsetY;
        painter.drawRoundedRect(imageRect.adjusted(-spread, -spread + offset - 1.0, spread, spread + offset + 1.0),
                                14.0,
                                14.0);
    }
    painter.restore();
}

}  // namespace markshot::shot
