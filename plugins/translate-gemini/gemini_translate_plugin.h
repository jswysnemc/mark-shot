#pragma once

#include "markshot/translate_provider_plugin.h"

#include <QObject>

namespace markshot::translate_gemini {

/**
 * Google Gemini 翻译 provider 插件。
 */
class GeminiTranslatePlugin final : public QObject, public markshot::plugin::TranslateProviderPlugin {
    Q_OBJECT
    Q_PLUGIN_METADATA(IID MARK_SHOT_TRANSLATE_PROVIDER_PLUGIN_IID FILE "metadata.json")
    Q_INTERFACES(markshot::plugin::TranslateProviderPlugin)

public:
    QString providerId() const override;
    QString displayName() const override;
    bool isAvailable(QString *error) const override;
    bool translate(const QVector<markshot::plugin::TranslateSegment> &segments,
                   const QString &targetLanguage,
                   QVector<markshot::plugin::TranslateSegment> *translations,
                   QString *error) override;
};

}  // namespace markshot::translate_gemini
