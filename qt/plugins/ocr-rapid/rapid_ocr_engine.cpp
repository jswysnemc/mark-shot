#include "rapid_ocr_engine.h"

#include "rapid_det_model.h"
#include "rapid_rec_model.h"
#include "rapid_image_input.h"
#include "rapid_ocr_word_segments.h"

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

bool recognizeRapidImage(const OrtApi &api, const RapidModelPaths &paths, const QImage &image,
                               QVector<markshot::plugin::OcrToken> *tokens,
                               QString *error)
{
    tokens->clear();
    RapidDetModel detModel(api);
    RapidRecModel recModel(api);
    // 1. 【OCR】【模型工作区】模型均为本次线程的局部对象，不跨运行库寿命保留
    if (!paths.isComplete()) {
        if (error) *error = QStringLiteral("PP-OCR models not found");
        return false;
    }
    if (!detModel.load(paths.detModel, error)) {
        return false;
    }

    const QImage source = rapidImageInput(image);

    // 2. 【OCR】【文本检测】取得文本框后立即释放检测模型；空白画面无需加载识别模型
    QVector<QRect> boxes;
    if (!detModel.detect(source, &boxes, error)) {
        return false;
    }
    detModel.release();
    if (boxes.isEmpty()) {
        return true;
    }

    // 3. 【OCR】【文字识别】此时才加载识别模型，保持原有逐行解码和字符区域规则
    if (!recModel.load(paths.recModel, paths.recDictionary, error)) {
        return false;
    }
    const QVector<QRect> ordered = sortBoxesInReadingOrder(boxes);
    int line = 0;
    int previousCenterY = -1;
    for (const QRect &box : ordered) {
        RapidRecResult recResult;
        QString recError;
        if (!recModel.recognize(source.copy(box), &recResult, &recError)) {
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

        const QVector<RapidOcrWordSegment> segments =
            buildRapidOcrWordSegments(recResult, QRectF(box));
        if (segments.isEmpty()) {
            // 识别模型没有提供可用字符跨度时，保留旧版整行 token 行为
            markshot::plugin::OcrToken token;
            token.text = recResult.text;
            token.box = QRectF(box);
            token.line = line;
            token.index = 0;
            token.confidence = recResult.confidence;
            tokens->append(token);
            continue;
        }

        for (const RapidOcrWordSegment &segment : segments) {
            markshot::plugin::OcrToken token;
            token.text = segment.text;
            token.box = segment.box;
            token.line = line;
            token.index = 0;
            token.confidence = segment.confidence;
            tokens->append(token);
        }
    }

    // 4. 【OCR】【文字识别】同行多框时补齐行内序号
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
