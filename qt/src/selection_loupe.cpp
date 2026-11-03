#include "selection_loupe.h"

#include "capture_cross_cursor.h"
#include "ui/overlay_chip_colors.h"

#include <QColor>
#include <QFont>
#include <QFontMetrics>
#include <QPainter>
#include <QPainterPath>
#include <QPen>

#include <algorithm>

namespace markshot::shot {
namespace {

constexpr qreal kLoupeOffset = 22.0;
constexpr qreal kViewportMargin = 12.0;
constexpr int kSampleRadius = 6;
constexpr qreal kDirtyPadding = 8.0;

/**
 * 把矩形限制在窗口边距内。
 * @param rect 待夹紧矩形。
 * @param viewport 窗口尺寸。
 * @return 夹紧后的矩形。
 */
QRectF clampLoupeRect(QRectF rect, QSize viewport)
{
    const qreal maxLeft = std::max(kViewportMargin,
                                   static_cast<qreal>(viewport.width()) - rect.width() - kViewportMargin);
    const qreal maxTop = std::max(kViewportMargin,
                                  static_cast<qreal>(viewport.height()) - rect.height() - kViewportMargin);
    rect.moveLeft(std::clamp(rect.left(), kViewportMargin, maxLeft));
    rect.moveTop(std::clamp(rect.top(), kViewportMargin, maxTop));
    return rect;
}

}  // namespace

SelectionLoupeLayout selectionLoupeLayout(QPointF widgetPoint,
                                          QSize viewport,
                                          qreal loupeSize,
                                          QPointF imagePoint,
                                          QSize imageSize,
                                          const QVector<QRect> &obstacles)
{
    SelectionLoupeLayout layout;
    const qreal size = std::max(48.0, loupeSize);
    QRectF loupe(widgetPoint.x() + kLoupeOffset,
                 widgetPoint.y() + kLoupeOffset,
                 size,
                 size);
    if (loupe.right() > viewport.width() - kViewportMargin) {
        loupe.moveRight(widgetPoint.x() - kLoupeOffset);
    }
    if (loupe.bottom() > viewport.height() - kViewportMargin) {
        loupe.moveBottom(widgetPoint.y() - kLoupeOffset);
    }
    layout.loupe = clampLoupeRect(loupe, viewport);

    // 1. 【截图】【选区放大镜】在指针四周选择遮挡最少的位置，避开编辑工具栏
    const auto overlapArea = [&obstacles](QRectF candidate) {
        qreal area = 0.0;
        for (const QRect &obstacle : obstacles) {
            const QRectF overlap = candidate.adjusted(-8.0, -8.0, 8.0, 8.0).intersected(obstacle);
            if (!overlap.isEmpty()) {
                area += overlap.width() * overlap.height();
            }
        }
        return area;
    };
    qreal bestOverlap = overlapArea(layout.loupe);
    for (const QPointF offset : {QPointF(kLoupeOffset, kLoupeOffset),
                                 QPointF(-kLoupeOffset - size, kLoupeOffset),
                                 QPointF(kLoupeOffset, -kLoupeOffset - size),
                                 QPointF(-kLoupeOffset - size, -kLoupeOffset - size)}) {
        if (bestOverlap == 0.0) {
            break;
        }
        const QRectF candidate = clampLoupeRect(QRectF(widgetPoint + offset, QSizeF(size, size)), viewport);
        const qreal overlap = overlapArea(candidate);
        if (overlap < bestOverlap) {
            layout.loupe = candidate;
            bestOverlap = overlap;
        }
    }

    // 2. 【截图】【选区放大镜】显示位置变化不影响指针附近的原图像素取样
    const QPoint center(qRound(imagePoint.x()), qRound(imagePoint.y()));
    const QRect candidate(center.x() - kSampleRadius,
                          center.y() - kSampleRadius,
                          kSampleRadius * 2 + 1,
                          kSampleRadius * 2 + 1);
    layout.sourceRect = candidate.intersected(QRect(QPoint(0, 0), imageSize));
    return layout;
}

QRect selectionLoupeDirtyRect(const SelectionLoupeLayout &layout)
{
    return layout.loupe.adjusted(-kDirtyPadding,
                                 -kDirtyPadding,
                                 kDirtyPadding,
                                 kDirtyPadding + 20.0)
        .toAlignedRect();
}

void drawSelectionLoupe(QPainter &painter,
                        const QImage &frame,
                        const SelectionLoupeLayout &layout,
                        const QString &caption,
                        const markshot::ui::ToolbarPalette &palette)
{
    if (frame.isNull() || layout.loupe.isEmpty() || layout.sourceRect.isEmpty()) {
        return;
    }

    painter.save();
    QPainterPath clip;
    clip.addEllipse(layout.loupe);
    painter.setClipPath(clip);
    painter.setRenderHint(QPainter::SmoothPixmapTransform, false);
    painter.drawImage(layout.loupe, frame.copy(layout.sourceRect));
    painter.setClipping(false);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setPen(QPen(QColor(59, 40, 46), 3.0));
    painter.setBrush(Qt::NoBrush);
    painter.drawEllipse(layout.loupe);

    const QPointF center = layout.loupe.center();
    painter.setPen(QPen(QColor(96, 165, 250), 2.0));
    painter.drawLine(QPointF(center.x() - 12.0, center.y()),
                     QPointF(center.x() + 12.0, center.y()));
    painter.drawLine(QPointF(center.x(), center.y() - 12.0),
                     QPointF(center.x(), center.y() + 12.0));

    if (!caption.isEmpty()) {
        QFont font = painter.font();
        font.setPointSize(11);
        font.setWeight(QFont::DemiBold);
        painter.setFont(font);
        const QFontMetrics metrics(painter.font());
        const QRectF label(layout.loupe.center().x()
                               - (metrics.horizontalAdvance(caption) + 20.0) / 2.0,
                           layout.loupe.bottom() - metrics.height() - 11.0,
                           metrics.horizontalAdvance(caption) + 20.0,
                           metrics.height() + 7.0);
        // 1. 【截图】【放大镜】说明标签跟随截图会话主题，暗色保持原配色
        const markshot::ui::OverlayChipColors chip = markshot::ui::overlayChipColors(
            palette, {QColor(8, 13, 19, 210), QColor(0, 0, 0, 0), QColor(226, 232, 240), {}, {}});
        painter.setPen(chip.border.alpha() > 0 ? QPen(chip.border, 1.0) : QPen(Qt::NoPen));
        painter.setBrush(chip.fill);
        painter.drawRoundedRect(label, 8.0, 8.0);
        painter.setPen(chip.text);
        painter.drawText(label, Qt::AlignCenter, caption);
    }
    painter.restore();
}

void drawSelectionPointer(QPainter &painter, QPointF widgetPoint)
{
    drawCaptureCrossCursor(painter, widgetPoint);
}

QRect selectionPointerDirtyRect(QPointF widgetPoint)
{
    return captureCrossCursorRect(widgetPoint);
}

}  // namespace markshot::shot
