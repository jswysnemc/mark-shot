#include "providers/translate/translate_plugin_task.h"

#include "markshot/translate_provider_plugin.h"
#include "providers/translate/translate_segments.h"

#include <QtConcurrent/QtConcurrentRun>

#include <utility>

namespace markshot::providers {

TranslatePluginTask::TranslatePluginTask(markshot::plugin::TranslateProviderPlugin *plugin,
                                         QByteArray inputJson,
                                         QString targetLanguage,
                                         QObject *parent)
    : ProviderTask(plugin ? plugin->providerId() : QStringLiteral("plugin"), parent)
    , m_plugin(plugin)
    , m_inputJson(std::move(inputJson))
    , m_targetLanguage(std::move(targetLanguage))
{
    m_timeoutTimer.setSingleShot(true);
    connect(&m_timeoutTimer, &QTimer::timeout, this, [this] {
        m_watcher.disconnect(this);
        emitFinished({false, TaskError::Timeout, {}, {}, {}});
    });
    connect(&m_watcher, &QFutureWatcher<TaskResult>::finished, this, [this] {
        m_timeoutTimer.stop();
        emitFinished(m_watcher.result());
    });
}

void TranslatePluginTask::start(int timeoutMs)
{
    if (!m_plugin) {
        emitFinished({false, TaskError::StartFailed, {}, QByteArrayLiteral("translate plugin is missing"), {}});
        return;
    }
    if (timeoutMs > 0) {
        m_timeoutTimer.start(timeoutMs);
    }

    markshot::plugin::TranslateProviderPlugin *plugin = m_plugin;
    const QByteArray inputJson = m_inputJson;
    const QString explicitLanguage = m_targetLanguage;
    m_watcher.setFuture(QtConcurrent::run([plugin, inputJson, explicitLanguage]() -> TaskResult {
        // 1. 输入 JSON 转分段，交给插件同步翻译
        QString inputLanguage;
        const QVector<TranslateSourceSegment> segments =
            translateSegmentsFromInputJson(inputJson, &inputLanguage);
        if (segments.isEmpty()) {
            return {false, TaskError::Failed, {}, QByteArrayLiteral("no source text"), {}};
        }
        const QString targetLanguage = !explicitLanguage.trimmed().isEmpty()
            ? explicitLanguage
            : (inputLanguage.isEmpty() ? QStringLiteral("Simplified Chinese") : inputLanguage);

        QVector<markshot::plugin::TranslateSegment> pluginSegments;
        pluginSegments.reserve(segments.size());
        for (const TranslateSourceSegment &segment : segments) {
            pluginSegments.append({segment.id, segment.text});
        }

        QVector<markshot::plugin::TranslateSegment> translated;
        QString error;
        if (!plugin->translate(pluginSegments, targetLanguage, &translated, &error)) {
            return {false, TaskError::Failed, {}, error.toUtf8(), {}};
        }

        // 2. 译文映射回标准 tokens JSON 契约
        QHash<int, QString> translations;
        for (const markshot::plugin::TranslateSegment &segment : translated) {
            translations.insert(segment.id, segment.text);
        }
        return {true,
                TaskError::None,
                translateTokensJson(segments, translations, plugin->providerId()),
                {},
                {}};
    }));
}

void TranslatePluginTask::cancel()
{
    m_timeoutTimer.stop();
    m_watcher.disconnect(this);
}

}  // namespace markshot::providers
