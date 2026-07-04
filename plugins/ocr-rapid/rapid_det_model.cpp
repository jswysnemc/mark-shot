#include "rapid_det_model.h"

#include <algorithm>
#include <cmath>

namespace markshot::ocr_rapid {
namespace {

constexpr int kMaxSideLength = 960;
constexpr float kBinaryThreshold = 0.3f;
constexpr float kBoxScoreThreshold = 0.5f;
constexpr float kUnclipRatio = 1.6f;
constexpr int kMinBoxSize = 3;

/**
 * 计算限制最长边并对齐 32 像素的缩放尺寸。
 * @param size 原始尺寸。
 * @return 模型输入尺寸。
 */
QSize detInputSize(const QSize &size)
{
    qreal ratio = 1.0;
    const int maxSide = std::max(size.width(), size.height());
    if (maxSide > kMaxSideLength) {
        ratio = static_cast<qreal>(kMaxSideLength) / maxSide;
    }
    const auto align32 = [](qreal value) {
        return std::max(32, static_cast<int>(std::round(value / 32.0)) * 32);
    };
    return {align32(size.width() * ratio), align32(size.height() * ratio)};
}

/**
 * 把图像归一化为 NCHW 浮点输入。
 * @param image 已缩放到输入尺寸的 ARGB32 图像。
 * @param input 输出浮点数据。
 * @return 无返回值。
 */
void fillDetInput(const QImage &image, QVector<float> *input)
{
    // 与 rapidocr 一致：BGR 通道顺序，按位置应用 ImageNet mean/std
    constexpr float kMean[] = {0.485f, 0.456f, 0.406f};
    constexpr float kStd[] = {0.229f, 0.224f, 0.225f};
    const int width = image.width();
    const int height = image.height();
    const qsizetype plane = static_cast<qsizetype>(width) * height;
    input->resize(plane * 3);
    float *blue = input->data();
    float *green = blue + plane;
    float *red = green + plane;
    for (int y = 0; y < height; ++y) {
        const QRgb *row = reinterpret_cast<const QRgb *>(image.constScanLine(y));
        for (int x = 0; x < width; ++x) {
            const qsizetype offset = static_cast<qsizetype>(y) * width + x;
            blue[offset] = (qBlue(row[x]) / 255.0f - kMean[0]) / kStd[0];
            green[offset] = (qGreen(row[x]) / 255.0f - kMean[1]) / kStd[1];
            red[offset] = (qRed(row[x]) / 255.0f - kMean[2]) / kStd[2];
        }
    }
}

struct Component {
    int minX = 0;
    int minY = 0;
    int maxX = 0;
    int maxY = 0;
    double scoreSum = 0.0;
    int pixelCount = 0;
};

/**
 * 用连通域标记从概率图提取文本区域包围盒。
 * @param probabilities 概率图数据。
 * @param width 概率图宽度。
 * @param height 概率图高度。
 * @return 连通域列表。
 */
QVector<Component> connectedComponents(const QVector<float> &probabilities, int width, int height)
{
    QVector<quint8> visited(static_cast<qsizetype>(width) * height, 0);
    QVector<Component> components;
    QVector<int> stack;

    for (int start = 0; start < width * height; ++start) {
        if (visited[start] || probabilities[start] < kBinaryThreshold) {
            continue;
        }

        // 1. 深度优先洪泛收集当前连通域
        Component component{start % width, start / width, start % width, start / width, 0.0, 0};
        stack.clear();
        stack.append(start);
        visited[start] = 1;
        while (!stack.isEmpty()) {
            const int index = stack.takeLast();
            const int x = index % width;
            const int y = index / width;
            component.minX = std::min(component.minX, x);
            component.maxX = std::max(component.maxX, x);
            component.minY = std::min(component.minY, y);
            component.maxY = std::max(component.maxY, y);
            component.scoreSum += probabilities[index];
            ++component.pixelCount;

            const int neighbors[4] = {index - 1, index + 1, index - width, index + width};
            const bool valid[4] = {x > 0, x < width - 1, y > 0, y < height - 1};
            for (int i = 0; i < 4; ++i) {
                if (valid[i] && !visited[neighbors[i]]
                    && probabilities[neighbors[i]] >= kBinaryThreshold) {
                    visited[neighbors[i]] = 1;
                    stack.append(neighbors[i]);
                }
            }
        }
        components.append(component);
    }
    return components;
}

}  // namespace

bool RapidDetModel::load(const QString &modelPath, QString *error)
{
    return m_session.load(modelPath, error);
}

bool RapidDetModel::detect(const QImage &image, QVector<QRect> *boxes, QString *error)
{
    boxes->clear();
    if (image.isNull()) {
        if (error) {
            *error = QStringLiteral("detection input image is empty");
        }
        return false;
    }

    // 1. 缩放并归一化输入
    const QSize inputSize = detInputSize(image.size());
    const QImage scaled = image.scaled(inputSize, Qt::IgnoreAspectRatio, Qt::SmoothTransformation)
                              .convertToFormat(QImage::Format_ARGB32);
    QVector<float> input;
    fillDetInput(scaled, &input);

    // 2. 推理得到概率图 [1,1,H,W]
    QVector<float> output;
    TensorShape outputShape;
    if (!m_session.run(input,
                       {{1, 3, inputSize.height(), inputSize.width()}},
                       &output,
                       &outputShape,
                       error)) {
        return false;
    }
    if (outputShape.dims.size() < 2) {
        if (error) {
            *error = QStringLiteral("unexpected detection output shape");
        }
        return false;
    }
    const int mapHeight = static_cast<int>(outputShape.dims.at(outputShape.dims.size() - 2));
    const int mapWidth = static_cast<int>(outputShape.dims.at(outputShape.dims.size() - 1));
    if (mapHeight <= 0 || mapWidth <= 0
        || static_cast<qsizetype>(mapHeight) * mapWidth > output.size()) {
        if (error) {
            *error = QStringLiteral("detection output size mismatch");
        }
        return false;
    }

    // 3. 连通域提取包围盒，按均值分数过滤后 unclip 外扩映射回原图
    const qreal scaleX = static_cast<qreal>(image.width()) / mapWidth;
    const qreal scaleY = static_cast<qreal>(image.height()) / mapHeight;
    const QVector<Component> components = connectedComponents(output, mapWidth, mapHeight);
    for (const Component &component : components) {
        const int boxWidth = component.maxX - component.minX + 1;
        const int boxHeight = component.maxY - component.minY + 1;
        if (boxWidth < kMinBoxSize || boxHeight < kMinBoxSize || component.pixelCount <= 0) {
            continue;
        }
        if (component.scoreSum / component.pixelCount < kBoxScoreThreshold) {
            continue;
        }

        // unclip：按 DB 论文近似，矩形外扩 area*ratio/perimeter
        const qreal delta = (static_cast<qreal>(boxWidth) * boxHeight * kUnclipRatio)
            / (2.0 * (boxWidth + boxHeight));
        const QRect mapped(
            static_cast<int>(std::floor((component.minX - delta) * scaleX)),
            static_cast<int>(std::floor((component.minY - delta) * scaleY)),
            static_cast<int>(std::ceil((boxWidth + delta * 2) * scaleX)),
            static_cast<int>(std::ceil((boxHeight + delta * 2) * scaleY)));
        const QRect clipped = mapped.intersected(QRect(QPoint(0, 0), image.size()));
        if (!clipped.isEmpty()) {
            boxes->append(clipped);
        }
    }
    return true;
}

}  // namespace markshot::ocr_rapid
