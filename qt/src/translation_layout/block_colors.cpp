#include "translation_layout/block_colors.h"

#include <QHash>
#include <QVector>

#include <algorithm>
#include <cmath>
#include <utility>

namespace markshot::translation_layout {

namespace {

// 每条边最多取样的像素数，控制大段落的取样开销
constexpr int kMaxSamplesPerEdge = 160;
// 框内每个方向最多取样的像素数
constexpr int kMaxInteriorSamples = 64;
// 文字颜色取与背景差别不低于最大差别该比例的像素
constexpr qreal kForegroundShare = 0.7;
// 文字与背景的亮度差低于该值时改用黑色或白色
constexpr qreal kMinLumaContrast = 90.0;

/// @brief 同一量化颜色格中的像素累计
struct ColorBin {
    qint64 red = 0;
    qint64 green = 0;
    qint64 blue = 0;
    int count = 0;
};

/**
 * 计算颜色的近似亮度。
 * @param color 颜色。
 * @return 0 到 255 的亮度。
 */
qreal luma(const QColor &color)
{
    return 0.299 * color.red() + 0.587 * color.green() + 0.114 * color.blue();
}

/**
 * 计算两种颜色的欧氏距离。
 * @param left 第一种颜色。
 * @param right 第二种颜色。
 * @return RGB 空间中的距离。
 */
qreal colorDistance(QRgb left, const QColor &right)
{
    const qreal red = qRed(left) - right.red();
    const qreal green = qGreen(left) - right.green();
    const qreal blue = qBlue(left) - right.blue();
    return std::sqrt(red * red + green * green + blue * blue);
}

/**
 * 把一个像素累计进量化颜色格，每个通道分成 8 档。
 * @param bins 颜色格。
 * @param pixel 像素值。
 * @return 无返回值。
 */
void addToBin(QHash<int, ColorBin> *bins, QRgb pixel)
{
    const int key = ((qRed(pixel) >> 5) << 6) | ((qGreen(pixel) >> 5) << 3) | (qBlue(pixel) >> 5);
    ColorBin &bin = (*bins)[key];
    bin.red += qRed(pixel);
    bin.green += qGreen(pixel);
    bin.blue += qBlue(pixel);
    ++bin.count;
}

/**
 * 在矩形内按步长取样并累计进颜色格。
 * @param image 原图。
 * @param rect 取样矩形，已限制在图片内。
 * @param bins 颜色格。
 * @return 无返回值。
 */
void sampleRect(const QImage &image, const QRect &rect, QHash<int, ColorBin> *bins)
{
    if (rect.isEmpty()) {
        return;
    }
    const int stepX = std::max(1, rect.width() / kMaxSamplesPerEdge);
    const int stepY = std::max(1, rect.height() / kMaxSamplesPerEdge);
    for (int y = rect.top(); y <= rect.bottom(); y += stepY) {
        for (int x = rect.left(); x <= rect.right(); x += stepX) {
            addToBin(bins, image.pixel(x, y));
        }
    }
}

/**
 * 取样本数最多的颜色格的平均色。
 * @param bins 颜色格。
 * @return 平均色，没有样本时返回无效颜色。
 */
QColor dominantColor(const QHash<int, ColorBin> &bins)
{
    const ColorBin *best = nullptr;
    for (auto it = bins.cbegin(); it != bins.cend(); ++it) {
        if (!best || it.value().count > best->count) {
            best = &it.value();
        }
    }
    if (!best || best->count == 0) {
        return {};
    }
    return QColor(static_cast<int>(best->red / best->count), static_cast<int>(best->green / best->count),
                  static_cast<int>(best->blue / best->count));
}

}  // namespace

BlockColors sampleBlockColors(const QImage &image, const QRectF &box, qreal lineHeight)
{
    BlockColors colors{QColor(255, 255, 255), QColor(17, 24, 39)};
    const QRect inner = box.toAlignedRect().intersected(image.rect());
    if (image.isNull() || inner.isEmpty()) {
        return colors;
    }

    // 1. 背景取文字框外侧一圈；框贴着图片边缘没有外圈时，退回取框内最外一圈
    const int margin = std::clamp(qRound(lineHeight * 0.15), 2, 6);
    const QRect outer = inner.adjusted(-margin, -margin, margin, margin).intersected(image.rect());
    QHash<int, ColorBin> ringBins;
    sampleRect(image, QRect(outer.left(), outer.top(), outer.width(), inner.top() - outer.top()), &ringBins);
    sampleRect(image, QRect(outer.left(), inner.bottom() + 1, outer.width(), outer.bottom() - inner.bottom()),
               &ringBins);
    sampleRect(image, QRect(outer.left(), inner.top(), inner.left() - outer.left(), inner.height()), &ringBins);
    sampleRect(image, QRect(inner.right() + 1, inner.top(), outer.right() - inner.right(), inner.height()),
               &ringBins);
    if (ringBins.isEmpty()) {
        sampleRect(image, QRect(inner.left(), inner.top(), inner.width(), 1), &ringBins);
        sampleRect(image, QRect(inner.left(), inner.bottom(), inner.width(), 1), &ringBins);
    }
    const QColor background = dominantColor(ringBins);
    if (!background.isValid()) {
        return colors;
    }
    colors.background = background;

    // 2. 文字颜色取框内与背景差别最大的一批像素，抗锯齿的过渡像素不计入
    const int stepX = std::max(1, inner.width() / kMaxInteriorSamples);
    const int stepY = std::max(1, inner.height() / kMaxInteriorSamples);
    QVector<std::pair<qreal, QRgb>> samples;
    qreal maxDistance = 0.0;
    for (int y = inner.top(); y <= inner.bottom(); y += stepY) {
        for (int x = inner.left(); x <= inner.right(); x += stepX) {
            const QRgb pixel = image.pixel(x, y);
            const qreal distance = colorDistance(pixel, background);
            maxDistance = std::max(maxDistance, distance);
            samples.append({distance, pixel});
        }
    }
    qint64 red = 0;
    qint64 green = 0;
    qint64 blue = 0;
    int count = 0;
    for (const auto &[distance, pixel] : samples) {
        if (maxDistance > 0.0 && distance >= maxDistance * kForegroundShare) {
            red += qRed(pixel);
            green += qGreen(pixel);
            blue += qBlue(pixel);
            ++count;
        }
    }
    if (count > 0) {
        colors.foreground = QColor(static_cast<int>(red / count), static_cast<int>(green / count),
                                   static_cast<int>(blue / count));
    }

    // 3. 文字与背景太接近时按背景明暗改用深色或浅色
    if (std::abs(luma(colors.foreground) - luma(background)) < kMinLumaContrast) {
        colors.foreground = luma(background) > 140.0 ? QColor(17, 24, 39) : QColor(248, 250, 252);
    }
    return colors;
}

}  // namespace markshot::translation_layout
