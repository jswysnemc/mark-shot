#pragma once

#include "rapid_rec_model.h"

#include <QRectF>
#include <QString>
#include <QVector>

namespace markshot::ocr_rapid {

struct RapidOcrWordSegment {
    QString text;
    QRectF box;
    qreal confidence = 0.0;
};

/**
 * 根据 CTC 字符跨度生成词级别 OCR 片段。
 * @param result 单行识别结果，包含字符文本、时间步跨度和置信度。
 * @param lineBox 单行检测框，坐标为原图像像素坐标。
 * @return 词级别片段列表，空列表表示无法可靠切分。
 */
QVector<RapidOcrWordSegment> buildRapidOcrWordSegments(const RapidRecResult &result,
                                                       const QRectF &lineBox);

}  // namespace markshot::ocr_rapid
