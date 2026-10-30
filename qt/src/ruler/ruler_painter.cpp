#include "ruler/ruler_painter.h"

#include "shot_window_internal.h"
#include "ui/theme.h"

#include <QFontMetricsF>
#include <QPainter>
#include <QPen>

#include <algorithm>
#include <array>
#include <cmath>

namespace markshot::ruler {

namespace {

// 以下尺寸均为默认粗细下的屏幕像素，绘制时乘以缩放倍数
// 浮动标签与边界、刻度和其他标签之间的间距
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
// 说明标签的字号、内边距和圆角
constexpr qreal kRulerFontPixelSize = 13.0;
constexpr qreal kRulerLabelPaddingX = 20.0;
constexpr qreal kRulerLabelPaddingY = 10.0;
constexpr qreal kRulerLabelRadius = 9.0;

/**
 * 按刻度颜色选择描边颜色，浅色刻度配深色描边，深色刻度配浅色描边。
 * @param color 刻度颜色。
 * @return 描边颜色。
 */
QColor haloColorFor(const QColor &color)
{
    const qreal luminance = 0.299 * color.redF() + 0.587 * color.greenF() + 0.114 * color.blueF();
    return luminance > 0.55 ? QColor(8, 13, 19, 200) : QColor(255, 255, 255, 220);
}

/**
 * 绘制带描边的刻度数字，任意背景上都能看清。
 * @param painter 当前绘制器。
 * @param rect 文字范围。
 * @param flags 对齐方式。
 * @param text 文字。
 * @param color 文字颜色。
 * @param unit 缩放倍数。
 * @return 无返回值。
 */
void drawHaloText(QPainter &painter, const QRectF &rect, int flags, const QString &text,
                  const QColor &color, qreal unit)
{
    // 1. 先在八个方向错开一点画描边色，再在原位画文字
    const qreal offset = std::max<qreal>(0.8, unit);
    painter.setPen(haloColorFor(color));
    static constexpr std::array<QPointF, 8> directions = {
        QPointF(-1, -1), QPointF(0, -1), QPointF(1, -1), QPointF(-1, 0),
        QPointF(1, 0), QPointF(-1, 1), QPointF(0, 1), QPointF(1, 1),
    };
    for (const QPointF &direction : directions) {
        painter.drawText(rect.translated(direction * offset), flags, text);
    }
    painter.setPen(color);
    painter.drawText(rect, flags, text);
}

/**
 * 选主刻度间隔：从按长度估算的间隔开始，按 1、2、5 的倍数放大，直到相邻刻度数字之间放得下。
 * @param imageLength 测量长度，图像像素。
 * @param paintLength 测量长度，绘制坐标。
 * @param minimumGap 相邻主刻度的最小间距，绘制坐标，取刻度数字的宽或高再加留白。
 * @return 主刻度间隔，图像像素。
 */
int majorTickStep(qreal imageLength, qreal paintLength, qreal minimumGap)
{
    int step = markshot::shot::rulerTickStep(imageLength);
    const qreal paintPerPixel = imageLength > 0.0 ? paintLength / imageLength : 0.0;
    while (paintPerPixel > 0.0 && step * paintPerPixel < minimumGap && step < 1'000'000) {
        const int magnitude = qRound(std::pow(10.0, std::floor(std::log10(static_cast<qreal>(step)))));
        const int mantissa = step / magnitude;
        step = mantissa == 1 ? 2 * magnitude : mantissa == 2 ? 5 * magnitude : 10 * magnitude;
    }
    return step;
}

}  // namespace

qreal rulerUnitForWidth(qreal width)
{
    return std::clamp<qreal>(1.0 + (width - 3.0) / 9.0, 0.75, 3.5);
}

QFont rulerFont(qreal unit)
{
    QFont font = markshot::theme::uiFont(10, QFont::DemiBold);
    font.setPixelSize(std::max(6, qRound(kRulerFontPixelSize * unit)));
    return font;
}

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

QRectF clampRulerFloatingRect(QRectF rect, const QRectF &bounds, qreal gap)
{
    if (rect.right() > bounds.right() - gap) {
        rect.moveRight(bounds.right() - gap);
    }
    if (rect.left() < bounds.left() + gap) {
        rect.moveLeft(bounds.left() + gap);
    }
    if (rect.bottom() > bounds.bottom() - gap) {
        rect.moveBottom(bounds.bottom() - gap);
    }
    if (rect.top() < bounds.top() + gap) {
        rect.moveTop(bounds.top() + gap);
    }
    return rect;
}

QRectF rulerScaleBounds(const QRectF &paintRect, qreal unit)
{
    return paintRect.adjusted(-kRulerLeftLabelOffset * unit,
                              -kRulerTopLabelOffset * unit,
                              kRulerMajorTickLength * unit,
                              kRulerMajorTickLength * unit);
}

QSizeF rulerLabelSize(const QFontMetricsF &metrics, const QString &text, qreal unit)
{
    return {std::ceil(metrics.horizontalAdvance(text)) + kRulerLabelPaddingX * unit,
            std::ceil(metrics.height()) + kRulerLabelPaddingY * unit};
}

QRectF placeRulerInfoLabel(const RulerGeometry &ruler,
                           const QSizeF &labelSize,
                           const QRectF &bounds,
                           const QVector<QRectF> &occupied)
{
    const QRectF &rect = ruler.paintRect;
    const qreal gap = kRulerFloatingGap * ruler.unit;
    // 1. 候选位置依次是下、上、右、左，都放在刻度区域之外
    const QVector<QRectF> candidates = {
        QRectF(QPointF(rect.left(), rect.bottom() + kRulerMajorTickLength * ruler.unit + gap), labelSize),
        QRectF(QPointF(rect.left(), rect.top() - labelSize.height() - kRulerTopLabelOffset * ruler.unit - gap),
               labelSize),
        QRectF(QPointF(rect.right() + kRulerMajorTickLength * ruler.unit + gap, rect.top()), labelSize),
        QRectF(QPointF(rect.left() - labelSize.width() - kRulerLeftLabelOffset * ruler.unit - gap, rect.top()),
               labelSize),
    };
    const QRectF scaleArea = rulerScaleBounds(rect, ruler.unit).adjusted(-gap, -gap, gap, gap);

    // 2. 移回边界后仍不压住自身刻度和已占用范围的第一个位置胜出
    auto overlaps = [&](const QRectF &candidate) {
        if (candidate.intersects(scaleArea)) {
            return true;
        }
        return std::any_of(occupied.cbegin(), occupied.cend(), [&](const QRectF &other) {
            return candidate.intersects(other.adjusted(-gap, -gap, gap, gap));
        });
    };
    for (const QRectF &candidate : candidates) {
        const QRectF clamped = clampRulerFloatingRect(candidate, bounds, gap);
        if (!overlaps(clamped)) {
            return clamped;
        }
    }
    return clampRulerFloatingRect(candidates.first(), bounds, gap);
}

QVector<QRectF> layoutRulerInfoLabels(const QVector<RulerGeometry> &rulers,
                                      const QRectF &bounds,
                                      QVector<QRectF> occupied)
{
    // 1. 每把尺子的测量框和刻度都登记为占用，说明不会压住别的尺子
    for (const RulerGeometry &ruler : rulers) {
        occupied.append(rulerScaleBounds(ruler.paintRect, ruler.unit));
    }

    // 2. 按绘制先后放说明，放好的说明也登记为占用
    QVector<QRectF> labels;
    labels.reserve(rulers.size());
    for (const RulerGeometry &ruler : rulers) {
        const QFontMetricsF metrics(rulerFont(ruler.unit));
        const QSizeF labelSize = rulerLabelSize(metrics, rulerInfoText(ruler.imageRect), ruler.unit);
        const QRectF labelRect = placeRulerInfoLabel(ruler, labelSize, bounds, occupied);
        labels.append(labelRect);
        occupied.append(labelRect);
    }
    return labels;
}

void drawRulerLabel(QPainter &painter, const QRectF &rect, const QString &text, qreal unit)
{
    painter.save();
    painter.setFont(rulerFont(unit));
    painter.setPen(Qt::NoPen);
    painter.setBrush(QColor(8, 13, 19, 225));
    painter.drawRoundedRect(rect, kRulerLabelRadius * unit, kRulerLabelRadius * unit);
    painter.setPen(QColor(229, 231, 235));
    painter.drawText(rect, Qt::AlignCenter, text);
    painter.restore();
}

void drawRulerScale(QPainter &painter, const RulerGeometry &ruler)
{
    const QRectF &imageRect = ruler.imageRect;
    const QRectF &paintRect = ruler.paintRect;
    const qreal unit = ruler.unit;
    painter.save();
    painter.setFont(rulerFont(unit));

    // 1. 测量框，填充色取标注颜色的一成透明度
    QColor fill = ruler.color;
    fill.setAlphaF(ruler.color.alphaF() * 0.1);
    painter.setPen(QPen(ruler.color, std::max<qreal>(1.0, 2.0 * unit)));
    painter.setBrush(fill);
    painter.drawRoundedRect(paintRect, 3.0 * unit, 3.0 * unit);

    // 2. 上下两边画横向刻度，主刻度在上方标数字；尺子较短时放大主刻度间隔，数字不挤在一起
    const int widthPx = qRound(imageRect.width());
    const int heightPx = qRound(imageRect.height());
    const QFontMetricsF metrics(painter.font());
    const qreal labelGap = 6.0 * unit;
    const int stepX = majorTickStep(imageRect.width(), paintRect.width(),
                                    metrics.horizontalAdvance(QString::number(widthPx)) + labelGap);
    const int stepY = majorTickStep(imageRect.height(), paintRect.height(), metrics.height() + labelGap / 2.0);
    const int minorX = std::max(1, stepX / 5);
    const int minorY = std::max(1, stepY / 5);
    const QPen tickPen(ruler.color, std::max<qreal>(1.0, unit));
    if (imageRect.width() > 0.0) {
        for (int tick = 0; tick <= widthPx; tick += minorX) {
            const bool major = tick % stepX == 0;
            const qreal x = paintRect.left() + tick * paintRect.width() / imageRect.width();
            const qreal length = (major ? kRulerMajorTickLength : kRulerMinorTickLength) * unit;
            painter.setPen(tickPen);
            painter.drawLine(QPointF(x, paintRect.top()), QPointF(x, paintRect.top() - length));
            painter.drawLine(QPointF(x, paintRect.bottom()), QPointF(x, paintRect.bottom() + length));
            if (major) {
                drawHaloText(painter,
                             QRectF(x - kRulerTopLabelWidth * unit / 2.0,
                                    paintRect.top() - kRulerTopLabelOffset * unit,
                                    kRulerTopLabelWidth * unit,
                                    kRulerTickLabelHeight * unit),
                             Qt::AlignCenter, QString::number(tick), ruler.color, unit);
            }
        }
    }

    // 3. 左右两边画纵向刻度，主刻度在左侧标数字
    if (imageRect.height() > 0.0) {
        for (int tick = 0; tick <= heightPx; tick += minorY) {
            const bool major = tick % stepY == 0;
            const qreal y = paintRect.top() + tick * paintRect.height() / imageRect.height();
            const qreal length = (major ? kRulerMajorTickLength : kRulerMinorTickLength) * unit;
            painter.setPen(tickPen);
            painter.drawLine(QPointF(paintRect.left(), y), QPointF(paintRect.left() - length, y));
            painter.drawLine(QPointF(paintRect.right(), y), QPointF(paintRect.right() + length, y));
            if (major) {
                drawHaloText(painter,
                             QRectF(paintRect.left() - kRulerLeftLabelOffset * unit,
                                    y - kRulerTickLabelHeight * unit / 2.0,
                                    kRulerLeftLabelWidth * unit,
                                    kRulerTickLabelHeight * unit),
                             Qt::AlignRight | Qt::AlignVCenter, QString::number(tick), ruler.color, unit);
            }
        }
    }
    painter.restore();
}

}  // namespace markshot::ruler
