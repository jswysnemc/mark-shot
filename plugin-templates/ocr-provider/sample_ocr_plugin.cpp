#include "sample_ocr_plugin.h"

QString SampleOcrPlugin::providerId() const
{
    return QStringLiteral("sample-ocr");
}

QString SampleOcrPlugin::displayName() const
{
    return QStringLiteral("Sample OCR");
}

bool SampleOcrPlugin::isAvailable(QString *error) const
{
    Q_UNUSED(error)
    return true;
}

bool SampleOcrPlugin::recognize(const QImage &image,
                                QVector<markshot::plugin::OcrToken> *tokens,
                                QString *error)
{
    if (!tokens) {
        if (error) {
            *error = QStringLiteral("Output token container is null");
        }
        return false;
    }
    tokens->clear();
    if (image.isNull()) {
        if (error) {
            *error = QStringLiteral("Input image is empty");
        }
        return false;
    }

    // 1. 在这里调用真实 OCR 引擎，并把结果转换为 OcrToken
    return true;
}
