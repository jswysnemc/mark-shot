#include "shot_window_internal.h"

#include <QThread>
#include <QtConcurrent/QtConcurrentMap>

#include <algorithm>
#include <cmath>
#include <vector>

namespace markshot::shot {
namespace {

/// @brief 单个输入像素及其重采样权重
struct AxisSample {
    /// @brief 输入像素索引
    int index = 0;
    /// @brief 像素参与计算的权重
    double weight = 0.0;
};

/// @brief 单个坐标轴的重采样查找表
struct AxisTable {
    /// @brief 每个输出坐标使用的预计算样本
    std::vector<std::vector<AxisSample>> samples;
    /// @brief 查找表引用的最小输入坐标
    int first = 0;
    /// @brief 查找表引用的最大输入坐标
    int last = -1;
};

/// @brief 计算锐化滤波核的权重
/// @param distance 距离采样中心的偏移
/// @return 对应偏移处的滤波权重
double sharpKernel(double distance)
{
    const double x = std::abs(distance);
    if (x > kSharpKernelRadius) {
        return 0.0;
    }
    if (x <= 0.5) {
        return 17.0 / 16.0 - 7.0 / 4.0 * x * x;
    }
    if (x <= 1.5) {
        return x * x - 11.0 / 4.0 * x + 7.0 / 4.0;
    }
    return -1.0 / 8.0 * x * x + 5.0 / 8.0 * x - 25.0 / 32.0;
}

/// @brief 为单个坐标轴构造缩放查找表
/// @param inputSize 输入坐标轴的像素数
/// @param outputSize 输出坐标轴的像素数
/// @param sourceStart 输入图像中的起始坐标
/// @param scale 输出与输入的缩放比例
/// @return 已填充样本与范围的查找表
AxisTable buildAxisTable(int inputSize, int outputSize, double sourceStart, double scale)
{
    // 1. 【截图】【视口缩放】根据缩放比例确定每个输出像素的采样范围
    AxisTable table;
    table.samples.resize(outputSize);
    table.first = inputSize;

    const double filterScale = std::min(scale, 1.0);
    const double radius = kSharpKernelRadius / filterScale;
    for (int output = 0; output < outputSize; ++output) {
        const double center = sourceStart + (static_cast<double>(output) + 0.5) / scale - 0.5;
        const int first = std::max(0, static_cast<int>(std::floor(center - radius)));
        const int last = std::min(inputSize - 1, static_cast<int>(std::ceil(center + radius)));

        // 2. 【截图】【视口缩放】累计范围内各输入像素的滤波权重
        double sum = 0.0;
        std::vector<AxisSample> samples;
        samples.reserve(std::max(0, last - first + 1));
        for (int input = first; input <= last; ++input) {
            const double weight = sharpKernel((static_cast<double>(input) - center) * filterScale);
            if (weight == 0.0) {
                continue;
            }
            samples.push_back({input, weight});
            sum += weight;
        }

        // 3. 【截图】【视口缩放】归一化权重，无有效样本时使用最近像素
        if (samples.empty() || qFuzzyIsNull(sum)) {
            const int nearest = std::clamp(static_cast<int>(std::round(center)), 0, inputSize - 1);
            samples.push_back({nearest, 1.0});
            table.first = std::min(table.first, nearest);
            table.last = std::max(table.last, nearest);
        } else {
            for (AxisSample &sample : samples) {
                sample.weight /= sum;
                table.first = std::min(table.first, sample.index);
                table.last = std::max(table.last, sample.index);
            }
        }

        table.samples[output] = std::move(samples);
    }

    if (table.first > table.last) {
        table.first = 0;
        table.last = 0;
    }
    return table;
}

/// @brief 把加权浮点值转换为颜色字节
/// @param value 待转换的加权颜色值
/// @return 限制在 0 至 255 的整数值
int byteFromWeightedSum(double value)
{
    return std::clamp(static_cast<int>(std::lround(value)), 0, 255);
}

/// @brief 将颜色通道组合为预乘透明度像素
/// @param red 红色通道值
/// @param green 绿色通道值
/// @param blue 蓝色通道值
/// @param alpha 透明度通道值
/// @return 各颜色通道不超过透明度的像素值
QRgb premultipliedPixel(double red, double green, double blue, double alpha)
{
    const int a = byteFromWeightedSum(alpha);
    const int r = std::min(byteFromWeightedSum(red), a);
    const int g = std::min(byteFromWeightedSum(green), a);
    const int b = std::min(byteFromWeightedSum(blue), a);
    return qRgba(r, g, b, a);
}

/// @brief 按行分配图像处理任务并等待全部完成
/// @param rowCount 待处理的总行数
/// @param function 接收起始行和结束行的处理函数
/// @return 无返回值
template <typename Function>
void forRowRanges(int rowCount, Function function)
{
    if (rowCount <= 0) {
        return;
    }

    // 1. 【截图】【视口缩放】按行数和可用线程数确定并发度
    const int idealThreads = std::max(1, QThread::idealThreadCount());
    const int threadCount = std::clamp(rowCount / kMinSharpRowsPerThread, 1, idealThreads);
    if (threadCount == 1) {
        function(0, rowCount);
        return;
    }

    // 2. 【截图】【视口缩放】把全部行分配为互不重叠的处理区间
    std::vector<std::pair<int, int>> ranges;
    ranges.reserve(threadCount);
    const int rowsPerThread = rowCount / threadCount;
    int begin = 0;
    for (int i = 0; i < threadCount; ++i) {
        const int end = i == threadCount - 1 ? rowCount : begin + rowsPerThread;
        ranges.push_back({begin, end});
        begin = end;
    }

    // 3. 【截图】【视口缩放】并行处理各区间并等待所有任务完成
    QtConcurrent::blockingMap(ranges, [&function](const std::pair<int, int> &range) {
        function(range.first, range.second);
    });
}

}

/// @brief 裁切并缩放图像，保持像素边缘清晰
/// @param source 原始图像
/// @param sourceRect 原始图像中的取景区域
/// @param targetSize 输出图像尺寸
/// @return 缩放后的图像，无效输入返回空图像
QImage renderSharpViewport(const QImage &source, const QRectF &sourceRect, const QSize &targetSize)
{
    if (source.isNull() || sourceRect.isEmpty() || targetSize.isEmpty()) {
        return {};
    }

    // 1. 【截图】【视口缩放】规范像素格式并限制取景范围
    const QImage src = source.format() == QImage::Format_ARGB32_Premultiplied
        ? source
        : source.convertToFormat(QImage::Format_ARGB32_Premultiplied);
    const QRectF sourceBounds(QPointF(0.0, 0.0), QSizeF(src.size()));
    const QRectF clampedSourceRect = sourceRect.intersected(sourceBounds);
    if (clampedSourceRect.isEmpty()) {
        return {};
    }

    // 2. 【截图】【视口缩放】分别构造横向和纵向采样表
    const double scaleX = static_cast<double>(targetSize.width()) / clampedSourceRect.width();
    const double scaleY = static_cast<double>(targetSize.height()) / clampedSourceRect.height();
    const AxisTable xTable =
        buildAxisTable(src.width(), targetSize.width(), clampedSourceRect.left(), scaleX);
    const AxisTable yTable =
        buildAxisTable(src.height(), targetSize.height(), clampedSourceRect.top(), scaleY);

    const int intermediateHeight = yTable.last - yTable.first + 1;
    QImage horizontal(targetSize.width(), intermediateHeight, QImage::Format_ARGB32_Premultiplied);
    const uchar *sourceBits = src.constBits();
    const int sourceStride = src.bytesPerLine();
    uchar *horizontalBits = horizontal.bits();
    const int horizontalStride = horizontal.bytesPerLine();

    // 3. 【截图】【视口缩放】先完成横向重采样
    forRowRanges(intermediateHeight, [&](int begin, int end) {
        for (int row = begin; row < end; ++row) {
            const int y = yTable.first + row;
            const auto *sourceLine =
                reinterpret_cast<const QRgb *>(sourceBits + y * sourceStride);
            auto *targetLine =
                reinterpret_cast<QRgb *>(horizontalBits + row * horizontalStride);
            for (int x = 0; x < targetSize.width(); ++x) {
                double a = 0.0;
                double r = 0.0;
                double g = 0.0;
                double b = 0.0;
                for (const AxisSample &sample : xTable.samples[x]) {
                    const QRgb pixel = sourceLine[sample.index];
                    a += sample.weight * qAlpha(pixel);
                    r += sample.weight * qRed(pixel);
                    g += sample.weight * qGreen(pixel);
                    b += sample.weight * qBlue(pixel);
                }
                targetLine[x] = premultipliedPixel(r, g, b, a);
            }
        }
    });

    QImage target(targetSize, QImage::Format_ARGB32_Premultiplied);
    const uchar *horizontalReadBits = horizontal.constBits();
    const int horizontalReadStride = horizontal.bytesPerLine();
    uchar *targetBits = target.bits();
    const int targetStride = target.bytesPerLine();

    // 4. 【截图】【视口缩放】完成纵向重采样并返回结果
    forRowRanges(targetSize.height(), [&](int begin, int end) {
        for (int y = begin; y < end; ++y) {
            auto *targetLine = reinterpret_cast<QRgb *>(targetBits + y * targetStride);
            for (int x = 0; x < targetSize.width(); ++x) {
                double a = 0.0;
                double r = 0.0;
                double g = 0.0;
                double b = 0.0;
                for (const AxisSample &sample : yTable.samples[y]) {
                    const auto *sourceLine = reinterpret_cast<const QRgb *>(
                        horizontalReadBits + (sample.index - yTable.first) * horizontalReadStride);
                    const QRgb pixel = sourceLine[x];
                    a += sample.weight * qAlpha(pixel);
                    r += sample.weight * qRed(pixel);
                    g += sample.weight * qGreen(pixel);
                    b += sample.weight * qBlue(pixel);
                }
                targetLine[x] = premultipliedPixel(r, g, b, a);
            }
        }
    });
    return target;
}

}
