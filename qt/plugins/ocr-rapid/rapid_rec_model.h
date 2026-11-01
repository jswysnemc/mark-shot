#pragma once

#include "rapid_onnx_session.h"

#include <QImage>
#include <QString>
#include <QStringList>
#include <QVector>

namespace markshot::ocr_rapid {

struct RapidRecCharacter {
    QString text;
    int startStep = 0;
    int endStep = 0;
    qreal confidence = 0.0;
};

struct RapidRecResult {
    QString text;
    qreal confidence = 0.0;
    QVector<RapidRecCharacter> characters;
    int stepCount = 0;
};

/**
 * PP-OCR CTC 文本识别模型。
 *
 * 输入单行文本图像，输出识别文本与平均置信度。
 */
class RapidRecModel final {
public:
    /** 【OCR】【识别模型】将模型会话绑定到本次运行库 api，不持有跨任务的原生状态。 */
    explicit RapidRecModel(const OrtApi &api) : m_session(api) {}

    /**
     * 加载识别模型与字符字典。
     * @param modelPath 模型文件路径。
     * @param dictionaryPath 字典文件路径。
     * @param error 输出错误信息。
     * @return 加载成功时返回 true。
     */
    bool load(const QString &modelPath, const QString &dictionaryPath, QString *error);

    /**
     * 释放识别会话与字典。
     * @return 无返回值。
     */
    void release();

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
