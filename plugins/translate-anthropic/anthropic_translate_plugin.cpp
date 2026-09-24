#include "anthropic_translate_plugin.h"

#include "anthropic_translate_config.h"
#include "anthropic_translate_request.h"

#include <QHash>

namespace markshot::translate_anthropic {

QString AnthropicTranslatePlugin::providerId() const
{
    return QStringLiteral("anthropic");
}

QString AnthropicTranslatePlugin::displayName() const
{
    return QStringLiteral("Anthropic Claude");
}

bool AnthropicTranslatePlugin::isAvailable(QString *error) const
{
    return validateAnthropicTranslateConfig(readAnthropicTranslateConfig(), error);
}

bool AnthropicTranslatePlugin::translate(const QVector<markshot::plugin::TranslateSegment> &segments,
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

    const AnthropicTranslateConfig config = readAnthropicTranslateConfig();
    if (!validateAnthropicTranslateConfig(config, error)) {
        return false;
    }

    const QByteArray payload = buildAnthropicPayload(config, segments, targetLanguage);
    QByteArray responseBody;
    if (!sendAnthropicRequest(config, payload, &responseBody, error)) {
        return false;
    }

    QHash<int, QString> translatedById;
    if (!parseAnthropicResponse(responseBody, &translatedById, error)) {
        return false;
    }

    for (const auto &segment : segments) {
        const QString text = translatedById.value(segment.id).trimmed();
        translations->append({segment.id, text.isEmpty() ? segment.text : text});
    }
    return true;
}

}  // namespace markshot::translate_anthropic
