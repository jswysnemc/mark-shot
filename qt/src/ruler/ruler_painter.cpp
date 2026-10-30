#include "ruler/ruler_painter.h"

#include "shot_window_internal.h"
#include "ui/theme.h"

#include <QFontMetricsF>
#include <QPainter>
#include <QPen>
#include <QTransform>

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
// 刻度数字靠近测量框的一端与测量框的距离，以及横向刻度数字的行高
constexpr qreal kRulerNumberGap = 14.0;
constexpr qreal kRulerTickLabelHeight = 16.0;
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

/// 一把尺子的刻度数字放在测量框哪一侧，以及两行数字所在的条带
struct RulerTickLayout {
    // 横向刻度数字在测量框下方时为 true，默认在上方
    bool numbersBelow = false;
    // 纵向刻度数字在测量框右侧时为 true，默认在左侧
    bool numbersRight = false;
    // 横向、纵向刻度数字所在的条带，绘制坐标，未旋转
    QRectF horizontalBand;
    QRectF verticalBand;
};

/**
 * 计算横向刻度数字所在的条带：一行数字的高度，左右各伸出半个最长数字。
 * @param ruler 尺子范围及缩放倍数。
 * @param metrics 刻度数字字体度量。
 * @param below true 表示放在测量框下方。
 * @return 条带，绘制坐标，未旋转。
 */
QRectF horizontalNumberBand(const RulerGeometry &ruler, const QFontMetricsF &metrics, bool below)
{
    const QRectF &rect = ruler.paintRect;
    const qreal unit = ruler.unit;
    const qreal height = kRulerTickLabelHeight * unit;
    const qreal overhang = metrics.horizontalAdvance(QString::number(qRound(ruler.imageRect.width()))) / 2.0;
    const qreal top = below ? rect.bottom() + kRulerNumberGap * unit : rect.top() - kRulerNumberGap * unit - height;
    return QRectF(rect.left() - overhang, top, rect.width() + overhang * 2.0, height);
}

/**
 * 计算纵向刻度数字所在的条带：宽度为最长数字，贴着测量框一侧。
 * @param ruler 尺子范围及缩放倍数。
 * @param metrics 刻度数字字体度量。
 * @param right true 表示放在测量框右侧。
 * @return 条带，绘制坐标，未旋转。
 */
QRectF verticalNumberBand(const RulerGeometry &ruler, const QFontMetricsF &metrics, bool right)
{
    const QRectF &rect = ruler.paintRect;
    const qreal unit = ruler.unit;
    const qreal width = std::ceil(metrics.horizontalAdvance(QString::number(qRound(ruler.imageRect.height()))));
    const qreal halfHeight = kRulerTickLabelHeight * unit / 2.0;
    const qreal left = right ? rect.right() + kRulerNumberGap * unit : rect.left() - kRulerNumberGap * unit - width;
    return QRectF(left, rect.top() - halfHeight, width, rect.height() + halfHeight * 2.0);
}

/**
 * 计算一段条带绕测量框中心旋转后的外接矩形。
 * @param ruler 尺子范围及旋转角度。
 * @param localRect 未旋转的条带，绘制坐标。
 * @return 旋转后的外接矩形，绘制坐标。
 */
QRectF rotatedAroundRuler(const RulerGeometry &ruler, const QRectF &localRect)
{
    if (qFuzzyIsNull(ruler.rotationDegrees)) {
        return localRect;
    }
    const QPointF center = ruler.paintRect.center();
    QTransform transform;
    transform.translate(center.x(), center.y());
    transform.rotate(ruler.rotationDegrees);
    transform.translate(-center.x(), -center.y());
    return transform.mapRect(localRect);
}

/**
 * 判断一段刻度数字条带是否完整落在允许范围内。未旋转时只看与测量框垂直的方向，
 * 沿测量框方向超出的数字逐个移回范围内；旋转后按外接矩形整体判断。
 * @param ruler 尺子范围、旋转角度和允许范围。
 * @param band 未旋转的条带，绘制坐标。
 * @param horizontal true 表示横向刻度数字，条带沿测量框上下边。
 * @return 落在范围内时返回 true。
 */
bool numberBandFits(const RulerGeometry &ruler, const QRectF &band, bool horizontal)
{
    const QRectF &bounds = ruler.bounds;
    if (!qFuzzyIsNull(ruler.rotationDegrees)) {
        return bounds.contains(rotatedAroundRuler(ruler, band));
    }
    return horizontal ? band.top() >= bounds.top() && band.bottom() <= bounds.bottom()
                      : band.left() >= bounds.left() && band.right() <= bounds.right();
}

/**
 * 把一段区域沿一个方向移回允许范围内，另一个方向保持不变。
 * @param rect 原始区域。
 * @param bounds 允许范围。
 * @param horizontal true 表示左右移动，false 表示上下移动。
 * @return 移回范围内的区域。
 */
QRectF clampedAlong(QRectF rect, const QRectF &bounds, bool horizontal)
{
    if (horizontal) {
        rect.moveLeft(std::clamp(rect.left(), bounds.left(), std::max(bounds.left(), bounds.right() - rect.width())));
    } else {
        rect.moveTop(std::clamp(rect.top(), bounds.top(), std::max(bounds.top(), bounds.bottom() - rect.height())));
    }
    return rect;
}

/**
 * 判断尺子未旋转、有允许范围且与范围相交，此时刻度数字可以逐个移回范围内。
 * @param ruler 尺子范围、旋转角度和允许范围。
 * @return 可以移回时返回 true。
 */
bool numbersClampToBounds(const RulerGeometry &ruler)
{
    const QRectF &rect = ruler.paintRect;
    const QRectF &bounds = ruler.bounds;
    return qFuzzyIsNull(ruler.rotationDegrees) && !bounds.isEmpty() && rect.right() >= bounds.left()
        && rect.left() <= bounds.right() && rect.bottom() >= bounds.top() && rect.top() <= bounds.bottom();
}

/**
 * 决定刻度数字放在哪一侧：默认在上方和左侧，超出允许范围时换到对侧；两侧都放不下时，
 * 未旋转的尺子把数字条带移进范围内，旋转的尺子保持默认位置。
 * @param ruler 尺子范围、缩放倍数、旋转角度和允许范围。
 * @return 刻度数字的位置。
 */
RulerTickLayout rulerTickLayout(const RulerGeometry &ruler)
{
    const QFontMetricsF metrics(rulerFont(ruler.unit));
    RulerTickLayout layout;
    layout.horizontalBand = horizontalNumberBand(ruler, metrics, false);
    layout.verticalBand = verticalNumberBand(ruler, metrics, false);
    if (ruler.bounds.isEmpty()) {
        return layout;
    }

    // 1. 默认一侧超出范围而对侧放得下时换到对侧
    const QRectF below = horizontalNumberBand(ruler, metrics, true);
    if (!numberBandFits(ruler, layout.horizontalBand, true) && numberBandFits(ruler, below, true)) {
        layout.numbersBelow = true;
        layout.horizontalBand = below;
    }
    const QRectF right = verticalNumberBand(ruler, metrics, true);
    if (!numberBandFits(ruler, layout.verticalBand, false) && numberBandFits(ruler, right, false)) {
        layout.numbersRight = true;
        layout.verticalBand = right;
    }

    // 2. 两侧都放不下时把条带移进范围内，数字压在测量框上也比看不见好
    if (numbersClampToBounds(ruler)) {
        layout.horizontalBand = clampedAlong(layout.horizontalBand, ruler.bounds, false);
        layout.verticalBand = clampedAlong(layout.verticalBand, ruler.bounds, true);
    }
    return layout;
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

QRectF rulerScaleBounds(const RulerGeometry &ruler)
{
    // 1. 测量框连同四边刻度，再并上两行刻度数字，最后按旋转取外接矩形
    const RulerTickLayout layout = rulerTickLayout(ruler);
    const qreal tick = kRulerMajorTickLength * ruler.unit;
    const QRectF local = ruler.paintRect.adjusted(-tick, -tick, tick, tick)
                             .united(layout.horizontalBand)
                             .united(layout.verticalBand);
    return rotatedAroundRuler(ruler, local);
}

qreal rulerTopReach(const RulerGeometry &ruler)
{
    // 1. 数字换到下方或移进测量框时，上边只剩刻度线
    const qreal tick = kRulerMajorTickLength * ruler.unit;
    return std::max(tick, ruler.paintRect.top() - rulerTickLayout(ruler).horizontalBand.top());
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
    const qreal gap = kRulerFloatingGap * ruler.unit;
    // 1. 候选位置依次是下、上、右、左，都放在刻度区域之外，与旋转后的测量框左边或上边对齐
    const QRectF body = rotatedAroundRuler(ruler, ruler.paintRect);
    const QRectF scale = rulerScaleBounds(ruler);
    const QVector<QRectF> candidates = {
        QRectF(QPointF(body.left(), scale.bottom() + gap), labelSize),
        QRectF(QPointF(body.left(), scale.top() - labelSize.height() - gap), labelSize),
        QRectF(QPointF(scale.right() + gap, body.top()), labelSize),
        QRectF(QPointF(scale.left() - labelSize.width() - gap, body.top()), labelSize),
    };
    const QRectF scaleArea = scale.adjusted(-gap, -gap, gap, gap);

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
        occupied.append(rulerScaleBounds(ruler));
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

    // 2. 上下两边画横向刻度，主刻度默认在上方标数字，靠近边缘时换到下方；尺子较短时放大主刻度间隔
    const RulerTickLayout layout = rulerTickLayout(ruler);
    const bool clampNumbers = numbersClampToBounds(ruler);
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
                // 数字贴着刻度居中，刻度在范围内时整段数字移回范围内，不被屏幕边缘截掉
                const QString text = QString::number(tick);
                const qreal textWidth = metrics.horizontalAdvance(text);
                QRectF label(x - textWidth / 2.0, layout.horizontalBand.top(), textWidth,
                             layout.horizontalBand.height());
                if (clampNumbers && x >= ruler.bounds.left() && x <= ruler.bounds.right()) {
                    label = clampedAlong(label, ruler.bounds, true);
                }
                drawHaloText(painter, label, Qt::AlignCenter | Qt::TextDontClip, text, ruler.color, unit);
            }
        }
    }

    // 3. 左右两边画纵向刻度，主刻度默认在左侧标数字，靠近边缘时换到右侧
    if (imageRect.height() > 0.0) {
        for (int tick = 0; tick <= heightPx; tick += minorY) {
            const bool major = tick % stepY == 0;
            const qreal y = paintRect.top() + tick * paintRect.height() / imageRect.height();
            const qreal length = (major ? kRulerMajorTickLength : kRulerMinorTickLength) * unit;
            painter.setPen(tickPen);
            painter.drawLine(QPointF(paintRect.left(), y), QPointF(paintRect.left() - length, y));
            painter.drawLine(QPointF(paintRect.right(), y), QPointF(paintRect.right() + length, y));
            if (major) {
                // 数字贴着测量框一侧对齐，刻度在范围内时整段数字移回范围内
                QRectF label(layout.verticalBand.left(), y - kRulerTickLabelHeight * unit / 2.0,
                             layout.verticalBand.width(), kRulerTickLabelHeight * unit);
                if (clampNumbers && y >= ruler.bounds.top() && y <= ruler.bounds.bottom()) {
                    label = clampedAlong(label, ruler.bounds, false);
                }
                const int align = (layout.numbersRight ? Qt::AlignLeft : Qt::AlignRight) | Qt::AlignVCenter;
                drawHaloText(painter, label, align | Qt::TextDontClip, QString::number(tick), ruler.color, unit);
            }
        }
    }
    painter.restore();
}

}  // namespace markshot::ruler
