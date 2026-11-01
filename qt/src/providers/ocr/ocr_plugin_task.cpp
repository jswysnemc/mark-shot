#include "providers/ocr/ocr_plugin_task.h"

#include "markshot/ocr_provider_plugin.h"
#include "memory/process_memory_trim.h"
#include "providers/ocr/ocr_task_executor.h"

#include <QImage>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QtConcurrent/QtConcurrentRun>

#include <utility>

namespace markshot::providers {
namespace {

/** 【OCR】【工作区释放】在图片和推理临时对象析构后归还空闲页，不保留已解码图片。 */
struct TrimAfterJob final {
    /** 归还本次任务已释放的堆页，无参数和返回值。 */
    ~TrimAfterJob() { markshot::memory::trimProcessMemoryWithLog(); }
};

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
        // 1. 【OCR】【任务超时】尚未开始的任务不再解码图片，同步推理中的任务仅丢弃结果
        cancel();
        emitFinished({false, TaskError::Timeout, {}, {}, {}});
    });
    connect(&m_watcher, &QFutureWatcher<TaskResult>::finished, this, [this] {
        if (m_cancelled->load(std::memory_order_relaxed)) return;
        m_timeoutTimer.stop();
        emitFinished(m_watcher.result());
    });
}

OcrPluginTask::~OcrPluginTask()
{
    cancel();
}

void OcrPluginTask::start(int timeoutMs)
{
    if (m_started || m_cancelled->load(std::memory_order_relaxed)) return;
    m_started = true;
    if (!m_plugin) {
        emitFinished({false, TaskError::StartFailed, {}, QByteArrayLiteral("ocr plugin is missing"), {}});
        return;
    }
    if (timeoutMs > 0) {
        m_timeoutTimer.start(timeoutMs);
    }

    markshot::plugin::OcrProviderPlugin *plugin = m_plugin;
    const QString imagePath = m_imagePath;
    const auto cancelled = m_cancelled;
    m_watcher.setFuture(QtConcurrent::run(ocrPluginTaskThreadPool(), [plugin, imagePath, cancelled]() -> TaskResult {
        // 1. 【OCR】【任务准入】取消状态不依赖窗口寿命，排队期间只持有路径而非整张图片
        if (cancelled->load(std::memory_order_relaxed)) return {};
        TrimAfterJob trim;
        const QImage image(imagePath);
        if (cancelled->load(std::memory_order_relaxed)) return {};
        if (image.isNull()) {
            return {false, TaskError::Failed, {}, QByteArrayLiteral("cannot load image for ocr plugin"), {}};
        }
        QVector<markshot::plugin::OcrToken> tokens;
        QString error;
        const bool recognized = plugin->recognize(image, &tokens, &error);
        // 2. 【OCR】【结果交付】同步插件不能强行中止，完成后再次核对接收方是否仍需要结果
        if (cancelled->load(std::memory_order_relaxed)) return {};
        if (!recognized) {
            return {false, TaskError::Failed, {}, error.toUtf8(), {}};
        }
        // 3. 转换为与 helper 一致的 JSON 契约
        return {true, TaskError::None, tokensToJson(plugin->providerId(), tokens), {}, {}};
    }));
}

void OcrPluginTask::cancel()
{
    m_cancelled->store(true, std::memory_order_relaxed);
    m_timeoutTimer.stop();
    m_watcher.disconnect(this);
}

}  // namespace markshot::providers
