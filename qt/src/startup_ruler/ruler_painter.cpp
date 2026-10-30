#include "startup_ruler/ruler_painter.h"

#include "shot_window_internal.h"

#include <QFontMetricsF>
#include <QPainter>
#include <QPen>

#include <algorithm>
#include <cmath>

namespace markshot::startup_ruler {

namespace {

// 浮动标签与窗口边缘、刻度和其他标签之间的间距
constexpr qreal kRulerFloatingGap = 8.0;
// 主刻度与次刻度的长度
constexpr qreal kRulerMajorTickLength = 13.0;
constexpr qreal kRulerMinorTickLength = 6.0;
// 上边刻度数字的位置和尺寸
constexpr qreal kRulerTopLabelOffset = 30.0;
constexpr qreal kRulerTopLabelWidth = 68.0;
constexpr qreal kRulerTickLabelHeight = 16.0;
// 左边刻度数字的位置和尺寸
constexpr qreal kRulerLeftLabelOffset = 62.0;
constexpr qreal kRulerLeftLabelWidth = 48.0;
// 尺寸说明与测量框四边的距离，刻度和刻度数字之外再留一段间距
constexpr qreal kRulerTopScaleGap = kRulerTopLabelOffset + kRulerFloatingGap;
constexpr qreal kRulerBottomScaleGap = kRulerMajorTickLength + kRulerFloatingGap;
constexpr qreal kRulerRightScaleGap = kRulerMajorTickLength + kRulerFloatingGap;
constexpr qreal kRulerLeftScaleGap = kRulerLeftLabelOffset + kRulerFloatingGap;

}  // namespace

QString rulerInfoText(const QRectF &imageRect)
{
    const int widthPx = qRound(imageRect.width());
    const int heightPx = qRound(imageRect.height());
    const qreal diagonal = std::hypot(imageRect.width(), imageRect.height());
    return QStringLiteral("%1 x %2 px   diag %3 px   area %4 px")
        .arg(widthPx)
        .arg(heightPx)
        .arg(qRound(diagonal))
        .arg(widthPx * heightPx);
}

QRectF clampRulerFloatingRect(QRectF rect, const QSizeF &windowSize)
{
    if (rect.right() > windowSize.width() - kRulerFloatingGap) {
        rect.moveRight(windowSize.width() - kRulerFloatingGap);
    }
    if (rect.left() < kRulerFloatingGap) {
        rect.moveLeft(kRulerFloatingGap);
    }
    if (rect.bottom() > windowSize.height() - kRulerFloatingGap) {
        rect.moveBottom(windowSize.height() - kRulerFloatingGap);
    }
    if (rect.top() < kRulerFloatingGap) {
        rect.moveTop(kRulerFloatingGap);
    }
    return rect;
}

QRectF rulerScaleBounds(const QRectF &widgetRect)
{
    return widgetRect.adjusted(-kRulerLeftLabelOffset,
                               -kRulerTopLabelOffset,
                               kRulerMajorTickLength,
                               kRulerMajorTickLength);
}

QRectF placeRulerInfoLabel(const QRectF &widgetRect,
                           const QSizeF &labelSize,
                           const QSizeF &windowSize,
                           const QVector<QRectF> &occupied)
{
    // 1. 候选位置依次是下、上、右、左，都放在刻度区域之外
    const QVector<QRectF> candidates = {
        QRectF(QPointF(widgetRect.left(), widgetRect.bottom() + kRulerBottomScaleGap), labelSize),
        QRectF(QPointF(widgetRect.left(), widgetRect.top() - labelSize.height() - kRulerTopScaleGap), labelSize),
        QRectF(QPointF(widgetRect.right() + kRulerRightScaleGap, widgetRect.top()), labelSize),
        QRectF(QPointF(widgetRect.left() - labelSize.width() - kRulerLeftScaleGap, widgetRect.top()), labelSize),
    };
    const QRectF scaleArea = rulerScaleBounds(widgetRect).adjusted(-kRulerFloatingGap,
                                                                   -kRulerFloatingGap,
                                                                   kRulerFloatingGap,
                                                                   kRulerFloatingGap);

    // 2. 移回窗口后仍不压住自身刻度和已占用范围的第一个位置胜出
    auto overlaps = [&](const QRectF &rect) {
        if (rect.intersects(scaleArea)) {
            return true;
        }
        return std::any_of(occupied.cbegin(), occupied.cend(), [&rect](const QRectF &other) {
            return rect.intersects(other.adjusted(-kRulerFloatingGap,
                                                  -kRulerFloatingGap,
                                                  kRulerFloatingGap,
                                                  kRulerFloatingGap));
        });
    };
    for (const QRectF &candidate : candidates) {
        const QRectF clamped = clampRulerFloatingRect(candidate, windowSize);
        if (!overlaps(clamped)) {
            return clamped;
        }
    }
    return clampRulerFloatingRect(candidates.first(), windowSize);
}

void drawRulerScale(QPainter &painter, const RulerGeometry &ruler)
{
    const QRectF &imageRect = ruler.imageRect;
    const QRectF &widgetRect = ruler.widgetRect;
    painter.save();

    // 1. 测量框
    painter.setPen(QPen(QColor(45, 212, 191), 2.0));
    painter.setBrush(QColor(45, 212, 191, 26));
    painter.drawRoundedRect(widgetRect, 3.0, 3.0);

    // 2. 上下两边画横向刻度，主刻度在上方标数字
    const int widthPx = qRound(imageRect.width());
    const int heightPx = qRound(imageRect.height());
    const int stepX = markshot::shot::rulerTickStep(imageRect.width());
    const int stepY = markshot::shot::rulerTickStep(imageRect.height());
    const int minorX = std::max(1, stepX / 5);
    const int minorY = std::max(1, stepY / 5);
    painter.setPen(QPen(QColor(204, 251, 241, 230), 1.0));
    if (imageRect.width() > 0.0) {
        for (int tick = 0; tick <= widthPx; tick += minorX) {
            const bool major = tick % stepX == 0;
            const qreal x = widgetRect.left() + tick * widgetRect.width() / imageRect.width();
            const qreal length = major ? kRulerMajorTickLength : kRulerMinorTickLength;
            painter.drawLine(QPointF(x, widgetRect.top()), QPointF(x, widgetRect.top() - length));
            painter.drawLine(QPointF(x, widgetRect.bottom()), QPointF(x, widgetRect.bottom() + length));
            if (major) {
                painter.drawText(QRectF(x - kRulerTopLabelWidth / 2.0,
                                        widgetRect.top() - kRulerTopLabelOffset,
                                        kRulerTopLabelWidth,
                                        kRulerTickLabelHeight),
                                 Qt::AlignCenter,
                                 QString::number(tick));
            }
        }
    }

    // 3. 左右两边画纵向刻度，主刻度在左侧标数字
    if (imageRect.height() > 0.0) {
        for (int tick = 0; tick <= heightPx; tick += minorY) {
            const bool major = tick % stepY == 0;
            const qreal y = widgetRect.top() + tick * widgetRect.height() / imageRect.height();
            const qreal length = major ? kRulerMajorTickLength : kRulerMinorTickLength;
            painter.drawLine(QPointF(widgetRect.left(), y), QPointF(widgetRect.left() - length, y));
            painter.drawLine(QPointF(widgetRect.right(), y), QPointF(widgetRect.right() + length, y));
            if (major) {
                painter.drawText(QRectF(widgetRect.left() - kRulerLeftLabelOffset,
                                        y - kRulerTickLabelHeight / 2.0,
                                        kRulerLeftLabelWidth,
                                        kRulerTickLabelHeight),
                                 Qt::AlignRight | Qt::AlignVCenter,
                                 QString::number(tick));
            }
        }
    }
    painter.restore();
}

QVector<QRectF> layoutRulerInfoLabels(const QVector<RulerGeometry> &rulers,
                                      const QFontMetricsF &metrics,
                                      const QSizeF &windowSize,
                                      QVector<QRectF> occupied)
{
    // 1. 每把尺子的测量框和刻度都登记为占用，说明不会压住别的尺子
    for (const RulerGeometry &ruler : rulers) {
        occupied.append(rulerScaleBounds(ruler.widgetRect));
    }

    // 2. 按拖出先后放说明，放好的说明也登记为占用
    QVector<QRectF> labels;
    labels.reserve(rulers.size());
    for (const RulerGeometry &ruler : rulers) {
        const QString info = rulerInfoText(ruler.imageRect);
        const QSizeF labelSize(std::ceil(metrics.horizontalAdvance(info)) + 20.0, std::ceil(metrics.height()) + 10.0);
        const QRectF labelRect = placeRulerInfoLabel(ruler.widgetRect, labelSize, windowSize, occupied);
        labels.append(labelRect);
        occupied.append(labelRect);
    }
    return labels;
}

}  // namespace markshot::startup_ruler
