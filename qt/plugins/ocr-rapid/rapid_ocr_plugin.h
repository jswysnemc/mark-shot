#pragma once

#include "markshot/ocr_provider_plugin.h"

#include <QMutex>
#include <QObject>

namespace markshot::ocr_rapid {

/**
 * PP-OCR ONNX 推理插件。
 *
 * 以外部库插件形态提供 rapidocr 等价的识别能力，模型文件复用
 * mark-shot 模型目录或旧 rapidocr venv 已下载的模型。
 */
class RapidOcrPlugin final : public QObject, public markshot::plugin::OcrProviderPlugin {
    Q_OBJECT
    Q_PLUGIN_METADATA(IID MARK_SHOT_OCR_PROVIDER_PLUGIN_IID FILE "metadata.json")
    Q_INTERFACES(markshot::plugin::OcrProviderPlugin)

public:
    QString providerId() const override;
    QString displayName() const override;
    bool isAvailable(QString *error) const override;
    bool recognize(const QImage &image,
                   QVector<markshot::plugin::OcrToken> *tokens,
                   QString *error) override;

private:
    QMutex m_mutex;
};

}  // namespace markshot::ocr_rapid
