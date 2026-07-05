#include "providers/ocr/ocr_plugin_task.h"

#include "markshot/ocr_provider_plugin.h"

#include <QImage>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QtConcurrent/QtConcurrentRun>

#include <utility>

namespace markshot::providers {
namespace {

/**
 * 把插件识别结果转换为标准 tokens JSON。
 * @param backend provider 标识。
 * @param tokens 插件输出 token。
 * @return {backend, tokens, errors} JSON 字节。
 */
QByteArray tokensToJson(const QString &backend, const QVector<markshot::plugin::OcrToken> &tokens)
{
    QJsonArray tokenArray;
    for (const markshot::plugin::OcrToken &token : tokens) {
        QJsonObject object;
        object.insert(QStringLiteral("text"), token.text);
        object.insert(QStringLiteral("box"),
                      QJsonArray{token.box.x(), token.box.y(), token.box.width(), token.box.height()});
        object.insert(QStringLiteral("line"), token.line);
        object.insert(QStringLiteral("index"), token.index);
        object.insert(QStringLiteral("confidence"), token.confidence);
        tokenArray.append(object);
    }
    QJsonObject root;
    root.insert(QStringLiteral("backend"), backend);
    root.insert(QStringLiteral("tokens"), tokenArray);
    root.insert(QStringLiteral("errors"), QJsonArray());
    return QJsonDocument(root).toJson(QJsonDocument::Compact);
}

}  // namespace

OcrPluginTask::OcrPluginTask(markshot::plugin::OcrProviderPlugin *plugin,
                             QString imagePath,
                             QObject *parent)
    : ProviderTask(plugin ? plugin->providerId() : QStringLiteral("plugin"), parent)
    , m_plugin(plugin)
    , m_imagePath(std::move(imagePath))
{
    m_timeoutTimer.setSingleShot(true);
    connect(&m_timeoutTimer, &QTimer::timeout, this, [this] {
        // 插件线程无法安全中断，超时后丢弃后续结果
        m_watcher.disconnect(this);
        emitFinished({false, TaskError::Timeout, {}, {}, {}});
    });
    connect(&m_watcher, &QFutureWatcher<TaskResult>::finished, this, [this] {
        m_timeoutTimer.stop();
        emitFinished(m_watcher.result());
    });
}

void OcrPluginTask::start(int timeoutMs)
{
    if (!m_plugin) {
        emitFinished({false, TaskError::StartFailed, {}, QByteArrayLiteral("ocr plugin is missing"), {}});
        return;
    }
    if (timeoutMs > 0) {
        m_timeoutTimer.start(timeoutMs);
    }

    markshot::plugin::OcrProviderPlugin *plugin = m_plugin;
    const QString imagePath = m_imagePath;
    m_watcher.setFuture(QtConcurrent::run([plugin, imagePath]() -> TaskResult {
        // 1. 工作线程加载图像并调用插件同步识别接口
        const QImage image(imagePath);
        if (image.isNull()) {
            return {false, TaskError::Failed, {}, QByteArrayLiteral("cannot load image for ocr plugin"), {}};
        }
        QVector<markshot::plugin::OcrToken> tokens;
        QString error;
        if (!plugin->recognize(image, &tokens, &error)) {
            return {false, TaskError::Failed, {}, error.toUtf8(), {}};
        }
        // 2. 转换为与 helper 一致的 JSON 契约
        return {true, TaskError::None, tokensToJson(plugin->providerId(), tokens), {}, {}};
    }));
}

void OcrPluginTask::cancel()
{
    m_timeoutTimer.stop();
    m_watcher.disconnect(this);
}

}  // namespace markshot::providers
