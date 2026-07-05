#include "rapid_ocr_plugin.h"

#include "rapid_model_paths.h"

#include <QMutexLocker>

#include <algorithm>

namespace markshot::ocr_rapid {
namespace {

/**
 * 按行聚类并排序检测框，行内自左向右。
 * @param boxes 检测框列表。
 * @return 排序后的检测框。
 */
QVector<QRect> sortBoxesInReadingOrder(QVector<QRect> boxes)
{
    // 1. 先按中心 Y 再按 X 粗排
    std::sort(boxes.begin(), boxes.end(), [](const QRect &left, const QRect &right) {
        const int leftCenter = left.center().y();
        const int rightCenter = right.center().y();
        if (leftCenter != rightCenter) {
            return leftCenter < rightCenter;
        }
        return left.x() < right.x();
    });

    // 2. 按平均行高的 0.65 倍聚类为行，行内按 X 排序
    QVector<QVector<QRect>> lines;
    for (const QRect &box : boxes) {
        bool placed = false;
        for (QVector<QRect> &line : lines) {
            double centerSum = 0.0;
            double heightSum = 0.0;
            for (const QRect &item : line) {
                centerSum += item.center().y();
                heightSum += item.height();
            }
            const double lineCenter = centerSum / line.size();
            const double threshold =
                std::max(6.0, (heightSum / line.size() + box.height()) / 2.0 * 0.65);
            if (std::abs(box.center().y() - lineCenter) <= threshold) {
                line.append(box);
                placed = true;
                break;
            }
        }
        if (!placed) {
            lines.append({box});
        }
    }

    QVector<QRect> ordered;
    for (QVector<QRect> &line : lines) {
        std::sort(line.begin(), line.end(), [](const QRect &left, const QRect &right) {
            return left.x() < right.x();
        });
        ordered.append(line);
    }
    return ordered;
}

}  // namespace

QString RapidOcrPlugin::providerId() const
{
    return QStringLiteral("rapid-onnx");
}

QString RapidOcrPlugin::displayName() const
{
    return QStringLiteral("PP-OCR (ONNX Runtime)");
}

bool RapidOcrPlugin::isAvailable(QString *error) const
{
    const RapidModelPaths paths = locateRapidModels();
    if (paths.isComplete()) {
        return true;
    }
    if (error) {
        *error = QStringLiteral("PP-OCR models not found in: %1")
                     .arg(rapidModelSearchDirs().join(QStringLiteral(", ")));
    }
    return false;
}

bool RapidOcrPlugin::ensureLoaded(QString *error)
{
    if (m_loaded) {
        return true;
    }
    if (m_loadFailed) {
        if (error) {
            *error = m_loadError;
        }
        return false;
    }

    const RapidModelPaths paths = locateRapidModels();
    if (!paths.isComplete()) {
        m_loadFailed = true;
        m_loadError = QStringLiteral("PP-OCR models not found");
        if (error) {
            *error = m_loadError;
        }
        return false;
    }

    QString loadError;
    if (!m_detModel.load(paths.detModel, &loadError)
        || !m_recModel.load(paths.recModel, paths.recDictionary, &loadError)) {
        m_loadFailed = true;
        m_loadError = loadError;
        if (error) {
            *error = loadError;
        }
        return false;
    }
    m_loaded = true;
    return true;
}

bool RapidOcrPlugin::recognize(const QImage &image,
                               QVector<markshot::plugin::OcrToken> *tokens,
                               QString *error)
{
    tokens->clear();
    QMutexLocker locker(&m_mutex);
    if (!ensureLoaded(error)) {
        return false;
    }

    const QImage source = image.convertToFormat(QImage::Format_ARGB32);

    // 1. 检测文本行区域
    QVector<QRect> boxes;
    if (!m_detModel.detect(source, &boxes, error)) {
        return false;
    }
    if (boxes.isEmpty()) {
        return true;
    }

    // 2. 按阅读顺序逐行识别；截图场景文字直立，省略方向分类
    const QVector<QRect> ordered = sortBoxesInReadingOrder(boxes);
    int line = 0;
    int previousCenterY = -1;
    for (const QRect &box : ordered) {
        RapidRecResult recResult;
        QString recError;
        if (!m_recModel.recognize(source.copy(box), &recResult, &recError)) {
            continue;
        }
        if (recResult.text.isEmpty()) {
            continue;
        }

        // 行号按检测框中心 Y 与行高判断换行
        if (previousCenterY >= 0
            && std::abs(box.center().y() - previousCenterY) > box.height() * 0.65) {
            ++line;
        }
        previousCenterY = box.center().y();

        markshot::plugin::OcrToken token;
        token.text = recResult.text;
        token.box = QRectF(box);
        token.line = line;
        token.index = 0;
        token.confidence = recResult.confidence;
        tokens->append(token);
    }

    // 3. 同行多框时补齐行内序号
    int index = 0;
    int currentLine = -1;
    for (markshot::plugin::OcrToken &token : *tokens) {
        if (token.line != currentLine) {
            currentLine = token.line;
            index = 0;
        }
        token.index = index++;
    }
    return true;
}

}  // namespace markshot::ocr_rapid
