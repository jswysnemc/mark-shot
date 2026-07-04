#pragma once

#include "rapid_onnx_session.h"

#include <QImage>
#include <QString>
#include <QStringList>

namespace markshot::ocr_rapid {

struct RapidRecResult {
    QString text;
    qreal confidence = 0.0;
};

/**
 * PP-OCR CTC 文本识别模型。
 *
 * 输入单行文本图像，输出识别文本与平均置信度。
 */
class RapidRecModel final {
public:
    /**
     * 加载识别模型与字符字典。
     * @param modelPath 模型文件路径。
     * @param dictionaryPath 字典文件路径。
     * @param error 输出错误信息。
     * @return 加载成功时返回 true。
     */
    bool load(const QString &modelPath, const QString &dictionaryPath, QString *error);

    /**
     * 识别单行文本图像。
     * @param lineImage 文本行图像。
     * @param result 输出识别结果。
     * @param error 输出错误信息。
     * @return 识别成功时返回 true（无文本时 text 为空）。
     */
    bool recognize(const QImage &lineImage, RapidRecResult *result, QString *error);

private:
    RapidOnnxSession m_session;
    QStringList m_characters;
};

}  // namespace markshot::ocr_rapid
