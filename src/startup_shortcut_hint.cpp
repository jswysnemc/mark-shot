#include "startup_shortcut_hint.h"

#include "ui/theme.h"

#include <QFontMetrics>
#include <QFont>
#include <QLinearGradient>
#include <QPainter>
#include <QPainterPath>
#include <QPen>
#include <QPointF>

#include <algorithm>

namespace markshot::startup_hint {
namespace {

constexpr qreal kViewportMargin = 24.0;
constexpr qreal kPanelPaddingX = 18.0;
constexpr qreal kPanelPaddingY = 16.0;
constexpr qreal kRowGap = 10.0;
constexpr qreal kColumnGap = 24.0;
constexpr qreal kKeyMinWidth = 108.0;
constexpr qreal kKeyHeight = 28.0;
constexpr qreal kInputIconSize = 16.0;
constexpr qreal kInputIconGap = 7.0;
constexpr qreal kPanelRadius = 18.0;
constexpr qreal kAvoidancePadding = 34.0;

/**
 * 返回快捷键标签字体。
 * @return 用于绘制左侧键帽的字体。
 */
QFont keyFont()
{
    return markshot::theme::uiFont(14, QFont::DemiBold);
}

/**
 * 返回说明文本字体。
 * @return 用于绘制右侧说明文字的字体。
 */
QFont labelFont()
{
    return markshot::theme::uiFont(14, QFont::DemiBold);
}

/**
 * 计算键帽列宽度。
 * @param items 需要展示的快捷键说明项。
 * @return 能容纳所有键帽文本的宽度。
 */
qreal keyColumnWidth(const QVector<ShortcutHintItem> &items)
{
    const QFontMetrics metrics(keyFont());
    qreal width = kKeyMinWidth;
    for (const ShortcutHintItem &item : items) {
        width = std::max<qreal>(width,
                                metrics.horizontalAdvance(item.key) + kInputIconSize + kInputIconGap + 24.0);
    }
    return width;
}

/**
 * 计算说明文本列宽度。
 * @param items 需要展示的快捷键说明项。
 * @return 能容纳所有说明文本的宽度。
 */
qreal labelColumnWidth(const QVector<ShortcutHintItem> &items)
{
    const QFontMetrics metrics(labelFont());
    qreal width = 0.0;
    for (const ShortcutHintItem &item : items) {
        width = std::max<qreal>(width, metrics.horizontalAdvance(item.label));
    }
    return width;
}

/**
 * 绘制键盘输入图标。
 * @param painter 当前绘图对象。
 * @param rect 图标绘制区域。
 * @param ink 图标颜色。
 */
void drawKeyboardIcon(QPainter &painter, QRectF rect, const QColor &ink)
{
    painter.setPen(QPen(ink, 1.25, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.setBrush(Qt::NoBrush);
    painter.drawRoundedRect(rect.adjusted(1.5, 3.5, -1.5, -3.0), 2.0, 2.0);

    painter.setPen(Qt::NoPen);
    painter.setBrush(ink);
    constexpr qreal keyRadius = 0.9;
    for (int row = 0; row < 2; ++row) {
        for (int col = 0; col < 3; ++col) {
            painter.drawEllipse(QPointF(rect.left() + 5.0 + col * 3.5,
                                        rect.top() + 7.0 + row * 3.0),
                                keyRadius,
                                keyRadius);
        }
    }
}

/**
 * 绘制鼠标输入图标。
 * @param painter 当前绘图对象。
 * @param rect 图标绘制区域。
 * @param ink 图标颜色。
 */
void drawMouseIcon(QPainter &painter, QRectF rect, const QColor &ink)
{
    const QRectF body(rect.left() + 3.0, rect.top() + 1.5, rect.width() - 6.0, rect.height() - 3.0);
    painter.setPen(QPen(ink, 1.35, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.setBrush(Qt::NoBrush);
    painter.drawRoundedRect(body, body.width() / 2.0, body.width() / 2.0);
    painter.drawLine(QPointF(body.center().x(), body.top() + 1.8),
                     QPointF(body.center().x(), body.top() + 5.0));
    painter.drawLine(QPointF(body.left() + 1.8, body.top() + 5.0),
                     QPointF(body.right() - 1.8, body.top() + 5.0));
}

/**
 * 绘制鼠标滚轮输入图标。
 * @param painter 当前绘图对象。
 * @param rect 图标绘制区域。
 * @param ink 图标颜色。
 */
void drawWheelIcon(QPainter &painter, QRectF rect, const QColor &ink)
{
    const QRectF body(rect.left() + 3.0, rect.top() + 1.5, rect.width() - 6.0, rect.height() - 3.0);
    painter.setPen(QPen(ink, 1.25, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.setBrush(Qt::NoBrush);
    painter.drawRoundedRect(body, body.width() / 2.0, body.width() / 2.0);

    painter.setPen(QPen(ink, 1.55, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.drawLine(QPointF(body.center().x(), body.top() + 4.0),
                     QPointF(body.center().x(), body.top() + 8.8));
    painter.drawLine(QPointF(body.center().x() - 2.2, body.top() + 6.4),
                     QPointF(body.center().x() + 2.2, body.top() + 6.4));
}

/**
 * 根据输入类型绘制键帽内的操作图标。
 * @param painter 当前绘图对象。
 * @param icon 输入图标类型。
 * @param rect 图标绘制区域。
 */
void drawInputIcon(QPainter &painter, InputIcon icon, QRectF rect)
{
    const QColor ink(5, 14, 26, 218);
    switch (icon) {
    case InputIcon::Keyboard:
        drawKeyboardIcon(painter, rect, ink);
        break;
    case InputIcon::Mouse:
        drawMouseIcon(painter, rect, ink);
        break;
    case InputIcon::Wheel:
        drawWheelIcon(painter, rect, ink);
        break;
    }
}

}  // namespace

PanelLayout layoutPanel(const QVector<ShortcutHintItem> &items, QSize viewportSize, PanelAnchor anchor)
{
    PanelLayout layout;
    if (items.isEmpty() || viewportSize.isEmpty()) {
        return layout;
    }

    const qreal keyWidth = keyColumnWidth(items);
    const qreal labelWidth = labelColumnWidth(items);
    const qreal rowHeight = kKeyHeight;
    const qsizetype gapCount = std::max<qsizetype>(0, items.size() - 1);
    const qreal panelWidth = kPanelPaddingX * 2.0 + keyWidth + kColumnGap + labelWidth;
    const qreal panelHeight = kPanelPaddingY * 2.0
        + rowHeight * items.size()
        + kRowGap * gapCount;

    const qreal x = kViewportMargin;
    qreal y = kViewportMargin;
    if (anchor == PanelAnchor::BottomLeft) {
        y = std::max<qreal>(kViewportMargin, viewportSize.height() - panelHeight - kViewportMargin);
    }

    layout.panelRect = QRectF(x, y, panelWidth, panelHeight);
    for (int i = 0; i < items.size(); ++i) {
        const qreal rowTop = y + kPanelPaddingY + i * (rowHeight + kRowGap);
        layout.keyRects.append(QRectF(x + kPanelPaddingX, rowTop, keyWidth, rowHeight));
        layout.labelRects.append(QRectF(x + kPanelPaddingX + keyWidth + kColumnGap,
                                        rowTop,
                                        labelWidth,
                                        rowHeight));
    }
    return layout;
}

QRectF avoidanceRect(const PanelLayout &layout)
{
    return layout.panelRect.adjusted(-kAvoidancePadding,
                                     -kAvoidancePadding,
                                     kAvoidancePadding,
                                     kAvoidancePadding);
}

PanelAnchor preferredAnchor(QPointF pointer, const QVector<ShortcutHintItem> &items, QSize viewportSize)
{
    const PanelLayout bottomLayout = layoutPanel(items, viewportSize, PanelAnchor::BottomLeft);
    if (avoidanceRect(bottomLayout).contains(pointer)) {
        return PanelAnchor::TopLeft;
    }
    return PanelAnchor::BottomLeft;
}

void drawPanel(QPainter &painter, const QVector<ShortcutHintItem> &items, const PanelLayout &layout)
{
    if (items.isEmpty() || layout.panelRect.isEmpty()) {
        return;
    }

    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);

    QPainterPath shadowPath;
    shadowPath.addRoundedRect(layout.panelRect.adjusted(0.0, 2.0, 0.0, 2.0), kPanelRadius, kPanelRadius);
    painter.fillPath(shadowPath, QColor(0, 0, 0, 92));

    QLinearGradient background(layout.panelRect.topLeft(), layout.panelRect.bottomRight());
    background.setColorAt(0.0, QColor(4, 9, 12, 226));
    background.setColorAt(1.0, QColor(0, 0, 0, 206));
    QPainterPath panelPath;
    panelPath.addRoundedRect(layout.panelRect, kPanelRadius, kPanelRadius);
    painter.fillPath(panelPath, background);
    painter.setPen(QPen(QColor(255, 255, 255, 22), 1.0));
    painter.drawPath(panelPath);

    painter.setFont(keyFont());
    const QFontMetrics keyMetrics(painter.font());
    for (int i = 0; i < items.size() && i < layout.keyRects.size() && i < layout.labelRects.size(); ++i) {
        const QRectF keyRect = layout.keyRects.at(i);
        const QRectF keyShadow = keyRect.translated(0.0, 1.5);
        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(0, 0, 0, 90));
        painter.drawRoundedRect(keyShadow, 7.0, 7.0);

        QLinearGradient keyFill(keyRect.topLeft(), keyRect.bottomLeft());
        keyFill.setColorAt(0.0, QColor(255, 255, 255, 244));
        keyFill.setColorAt(1.0, QColor(226, 232, 240, 244));
        painter.setBrush(keyFill);
        painter.setPen(QPen(QColor(11, 18, 32, 84), 1.0));
        painter.drawRoundedRect(keyRect, 7.0, 7.0);

        const QRectF iconRect(keyRect.left() + 10.0,
                              keyRect.center().y() - kInputIconSize / 2.0,
                              kInputIconSize,
                              kInputIconSize);
        drawInputIcon(painter, items.at(i).icon, iconRect);

        const QRectF textRect = keyRect.adjusted(10.0 + kInputIconSize + kInputIconGap, 0.0, -9.0, -1.0);
        painter.setPen(QColor(5, 14, 26, 242));
        painter.drawText(textRect,
                         Qt::AlignVCenter | Qt::AlignLeft,
                         keyMetrics.elidedText(items.at(i).key, Qt::ElideRight, qRound(textRect.width())));
    }

    painter.setFont(labelFont());
    painter.setPen(QColor(232, 250, 255, 244));
    for (int i = 0; i < items.size() && i < layout.labelRects.size(); ++i) {
        painter.drawText(layout.labelRects.at(i), Qt::AlignVCenter | Qt::AlignLeft, items.at(i).label);
    }

    painter.restore();
}

}  // namespace markshot::startup_hint
