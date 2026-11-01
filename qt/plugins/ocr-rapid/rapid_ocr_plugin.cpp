#include "rapid_ocr_plugin.h"

#include "rapid_model_paths.h"
#include "rapid_ocr_engine.h"
#include "rapid_onnx_runtime.h"

#include <QMutexLocker>

namespace markshot::ocr_rapid {

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
    if (error) error->clear();
    const RapidModelPaths paths = locateRapidModels();
    if (!paths.isComplete()) {
        if (error) {
            *error = QStringLiteral("PP-OCR models not found in: %1")
                         .arg(rapidModelSearchDirs().join(QStringLiteral(", ")));
        }
        return false;
    }
    // 1. 【OCR】【可用性】只检查 API 兼容性，不创建模型，也不等待正在进行的识别
    RapidOnnxRuntime runtime;
    return runtime.load(error);
}

bool RapidOcrPlugin::recognize(const QImage &image,
                               QVector<markshot::plugin::OcrToken> *tokens,
                               QString *error)
{
    if (error) error->clear();
    if (!tokens) {
        if (error) *error = QStringLiteral("OCR token output is missing");
        return false;
    }
    tokens->clear();
    QMutexLocker locker(&m_mutex);
    const RapidModelPaths paths = locateRapidModels();
    if (!paths.isComplete()) {
        if (error) *error = QStringLiteral("PP-OCR models not found");
        return false;
    }
    RapidOnnxRuntime runtime;
    if (!runtime.load(error)) return false;
    // 1. 【OCR】【识别寿命】原生对象与线程先结束，局部运行库句柄最后释放；插件本体继续保留
    return runtime.run([&](const OrtApi &api) {
        return recognizeRapidImage(api, paths, image, tokens, error);
    }, error);
}

}  // namespace markshot::ocr_rapid
