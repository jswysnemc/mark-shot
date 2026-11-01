#pragma once

#include "markshot/ocr_provider_plugin.h"
#include "rapid_model_paths.h"

struct OrtApi;

namespace markshot::ocr_rapid {

/** 【OCR】【识别流程】使用 api 和 paths 检测并识别 image，写入自有 tokens/error；所有原生对象在返回前释放，返回是否成功。 */
bool recognizeRapidImage(const OrtApi &api, const RapidModelPaths &paths, const QImage &image,
                         QVector<markshot::plugin::OcrToken> *tokens, QString *error);

}  // namespace markshot::ocr_rapid
