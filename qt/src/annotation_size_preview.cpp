#include "annotation_size_preview.h"

#include "capture_cross_cursor.h"
#include "ui/overlay_chip_colors.h"
#include "ui/theme.h"

#include <QFontMetricsF>
#include <QPainter>
#include <QPainterPath>

#include <algorithm>
#include <array>
#include <cmath>
#include <limits>

namespace markshot::shot {
namespace {

/// @brief 在光标四周选择能容纳提示且不遮挡控件的位置
/// @param position 光标热点
/// @param size 提示尺寸
/// @param footprintRadius 已绘制轮廓的半径，没有轮廓时为零
/// @param viewport 窗口可见区域
/// @param obstacles 需要避开的控件
/// @return 保持在窗口内的提示矩形
QRectF sizeBadgeRect(QPointF position, QSizeF size, qreal footprintRadius, QRectF viewport,
                     const QVector<QRect> &obstacles)
{
    const QRectF available = viewport.adjusted(8, 8, -8, -8);
    const qreal distance = std::max(18.0, footprintRadius + 12);
    const qreal pointerRadius = std::max(12.0, footprintRadius + 4);
    const QRectF pointerArea(position - QPointF(pointerRadius, pointerRadius),
                             QSizeF(pointerRadius * 2, pointerRadius * 2));
    const std::array<QPointF, 4> offsets = {
        QPointF(distance, 16), QPointF(distance, -size.height() - 16),
        QPointF(-distance - size.width(), 16), QPointF(-distance - size.width(), -size.height() - 16),
    };
    QRectF fallback;
    qreal leastOverlap = std::numeric_limits<qreal>::max();
    for (const QPointF &offset : offsets) {
        QRectF candidate(position + offset, size);
        candidate.moveLeft(std::clamp(candidate.left(), available.left(),
                                      std::max(available.left(), available.right() - size.width())));
        candidate.moveTop(std::clamp(candidate.top(), available.top(),
                                     std::max(available.top(), available.bottom() - size.height())));
        const QRectF coveredPointer = candidate.intersected(pointerArea);
        qreal overlap = coveredPointer.width() * coveredPointer.height();
        for (const QRect &obstacle : obstacles) {
            const QRectF covered = candidate.intersected(QRectF(obstacle).adjusted(-4, -4, 4, 4));
            overlap += covered.width() * covered.height();
        }
        if (qFuzzyIsNull(overlap)) {
            return candidate;
        }
        if (overlap < leastOverlap) {
            leastOverlap = overlap;
            fallback = candidate;
        }
    }
    return fallback;
}

/// @brief 创建提示内部的颜色及粗细示意，文字和马赛克使用对应形状
/// @param preview 当前标注参数
/// @param slot 示意图可占用的区域
/// @return 居中的示意路径
QPainterPath sizeSamplePath(const AnnotationSizePreview &preview, QRectF slot)
{
    QPainterPath path;
    const qreal extent = std::clamp(preview.width * preview.imageScale, 2.0, 12.0);
    if (preview.tool == types::Tool::Text) {
        path.addText(QPointF(), markshot::theme::uiFont(10, QFont::DemiBold), QStringLiteral("T"));
        path.translate(slot.center() - path.boundingRect().center());
    } else if (preview.tool == types::Tool::Mosaic) {
        path.addRect(QRectF(slot.center() - QPointF(extent / 2, extent / 2), QSizeF(extent, extent)));
    } else if (preview.tool == types::Tool::Number) {
        path.addEllipse(QRectF(slot.center() - QPointF(6, 6), QSizeF(12, 12)));
    } else {
        path.addRoundedRect(QRectF(slot.left(), slot.center().y() - extent / 2, slot.width(), extent),
                             std::min(3.0, extent / 2), std::min(3.0, extent / 2));
    }
    return path;
}

}  // namespace

void drawAnnotationSizePreview(QPainter &painter, const AnnotationSizePreview &preview,
                               QPointF position, QRectF viewport, const QVector<QRect> &obstacles,
                               const ui::ToolbarPalette &palette)
{
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);

    // 1. 【标注】【粗细预览】字体显示实际字号，序号显示气泡直径，其余工具显示图像像素尺寸
    const bool text = preview.tool == types::Tool::Text;
    const qreal imageExtent = text ? 19.0 + preview.width
        : preview.tool == types::Tool::Number ? std::max(26.0, 26.0 + preview.width * 2.7)
        : preview.width;
    const QString label = QStringLiteral("%1 %2").arg(qRound(imageExtent))
        .arg(text ? QStringLiteral("pt") : QStringLiteral("px"));
    const qreal extent = imageExtent * preview.imageScale;

    // 2. 【标注】【粗细预览】小尺寸只保留固定十字；轮廓按真实比例绘制，过大时仅显示数值
    const bool showFootprint = !text && extent >= 28.0 && extent <= 96.0;
    if (showFootprint) {
        const QPointF center = QPointF(position.toPoint()) + QPointF(0.5, 0.5);
        const QRectF footprint(center - QPointF(extent / 2, extent / 2), QSizeF(extent, extent));
        QPainterPath outline;
        if (preview.tool == types::Tool::Mosaic) {
            outline.addRect(footprint);
        } else {
            outline.addEllipse(footprint);
        }
        painter.strokePath(outline, QPen(QColor(17, 24, 39, 235), 3));
        painter.strokePath(outline, QPen(QColor(243, 244, 246), 1));
    }
    drawCaptureCrossCursor(painter, position);

    // 3. 【标注】【粗细预览】颜色示意与精确数值放在热点旁，位置随窗口边缘和控件避让
    painter.setFont(markshot::theme::uiFont(10, QFont::DemiBold));
    const QFontMetricsF metrics(painter.font());
    const QSizeF badgeSize(std::ceil(metrics.horizontalAdvance(label)) + 44,
                           std::max(26.0, std::ceil(metrics.height()) + 10));
    const QRectF badge = sizeBadgeRect(position, badgeSize, showFootprint ? extent / 2 : 0,
                                      viewport, obstacles);
    const ui::OverlayChipColors chip = ui::overlayChipColors(
        palette, {QColor(17, 24, 39, 242), QColor(148, 163, 184, 90), QColor(243, 244, 246),
                  QColor(226, 232, 240, 150), {}});
    painter.setPen(QPen(chip.border, 1));
    painter.setBrush(chip.fill);
    painter.drawRoundedRect(badge, 6, 6);
    const QRectF sample(badge.left() + 9, badge.center().y() - 8, 16, 16);
    painter.setPen(QPen(chip.mutedText, 0.75));
    painter.setBrush(preview.color);
    painter.drawPath(sizeSamplePath(preview, sample));
    painter.setPen(chip.text);
    painter.drawText(badge.adjusted(33, 0, -10, 0), Qt::AlignVCenter | Qt::AlignLeft, label);
    painter.restore();
}

}  // namespace markshot::shot
