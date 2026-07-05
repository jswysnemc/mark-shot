#pragma once

#include "markshot/ocr_provider_plugin.h"
#include "rapid_det_model.h"
#include "rapid_rec_model.h"

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
    /**
     * 首次识别时加载模型，线程安全。
     * @param error 输出错误信息。
     * @return 模型就绪时返回 true。
     */
    bool ensureLoaded(QString *error);

    QMutex m_mutex;
    RapidDetModel m_detModel;
    RapidRecModel m_recModel;
    bool m_loaded = false;
    bool m_loadFailed = false;
    QString m_loadError;
};

}  // namespace markshot::ocr_rapid
