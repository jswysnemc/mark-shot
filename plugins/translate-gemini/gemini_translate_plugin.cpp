#include "gemini_translate_plugin.h"

#include "gemini_translate_config.h"
#include "gemini_translate_request.h"

#include <QHash>

namespace markshot::translate_gemini {

QString GeminiTranslatePlugin::providerId() const
{
    return QStringLiteral("gemini");
}

QString GeminiTranslatePlugin::displayName() const
{
    return QStringLiteral("Google Gemini");
}

bool GeminiTranslatePlugin::isAvailable(QString *error) const
{
    return validateGeminiTranslateConfig(readGeminiTranslateConfig(), error);
}

bool GeminiTranslatePlugin::translate(const QVector<markshot::plugin::TranslateSegment> &segments,
                                      const QString &targetLanguage,
                                      QVector<markshot::plugin::TranslateSegment> *translations,
                                      QString *error)
{
    if (!translations) {
        if (error) {
            *error = QStringLiteral("translation output target is missing");
        }
        return false;
    }
    translations->clear();
    if (segments.isEmpty()) {
        return true;
    }

    const GeminiTranslateConfig config = readGeminiTranslateConfig();
    if (!validateGeminiTranslateConfig(config, error)) {
        return false;
    }

    const QByteArray payload = buildGeminiPayload(config, segments, targetLanguage);
    QByteArray responseBody;
    if (!sendGeminiRequest(config, payload, &responseBody, error)) {
        return false;
    }

    QHash<int, QString> translatedById;
    if (!parseGeminiResponse(responseBody, &translatedById, error)) {
        return false;
    }

    for (const auto &segment : segments) {
        const QString text = translatedById.value(segment.id).trimmed();
        translations->append({segment.id, text.isEmpty() ? segment.text : text});
    }
    return true;
}

}  // namespace markshot::translate_gemini
