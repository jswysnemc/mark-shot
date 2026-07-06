#include "sample_translate_plugin.h"

QString SampleTranslatePlugin::providerId() const
{
    return QStringLiteral("sample-translate");
}

QString SampleTranslatePlugin::displayName() const
{
    return QStringLiteral("Sample Translate");
}

bool SampleTranslatePlugin::isAvailable(QString *error) const
{
    Q_UNUSED(error)
    return true;
}

bool SampleTranslatePlugin::translate(const QVector<markshot::plugin::TranslateSegment> &segments,
                                      const QString &targetLanguage,
                                      QVector<markshot::plugin::TranslateSegment> *translations,
                                      QString *error)
{
    Q_UNUSED(targetLanguage)
    if (!translations) {
        if (error) {
            *error = QStringLiteral("Output translation container is null");
        }
        return false;
    }
    translations->clear();

    // 1. 在这里调用真实翻译模型或 API，并保持 segment id 不变
    for (const markshot::plugin::TranslateSegment &segment : segments) {
        translations->append(segment);
    }
    return true;
}
