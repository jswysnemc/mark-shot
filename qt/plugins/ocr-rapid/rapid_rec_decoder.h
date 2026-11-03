#pragma once

#include "rapid_onnx_session.h"
#include "rapid_rec_result.h"

#include <QStringList>

namespace markshot::ocr_rapid {

/**
 * 【OCR】【字符解码】在概率张量有效期间提取自有文字、置信度和跨度，不复制完整概率数组。
 * @param output 借用的连续浮点数据，只在调用期间读取。
 * @param count 可读取的浮点元素数量。
 * @param outputShape 模型输出形状 [1,T,C]。
 * @param dictionary 字符字典，索引 0 保留为空白类别。
 * @param result 自有结果输出，不保留 output 的指针。
 * @param error 失败原因输出，可为空。
 * @return 形状有效并完成解码时为 true，空白文本也视为成功。
 */
bool decodeRapidRecognition(const float *output, qsizetype count, const TensorShape &outputShape,
                            const QStringList &dictionary, RapidRecResult *result, QString *error);

}  // namespace markshot::ocr_rapid
