#include "providers/code_scan/code_scan_tasks.h"

#include "markshot/code_scan_provider_plugin.h"

#include <QImage>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QtConcurrent/QtConcurrentRun>

#include <utility>

#ifdef HAVE_ZXING_CPP
#include <ZXing/BarcodeFormat.h>
#include <ZXing/ReadBarcode.h>
#endif

namespace markshot::providers {
namespace {

/**
 * 把扫码结果序列化为标准 results JSON。
 * @param backend provider 标识。
 * @param results 扫码结果。
 * @return {backend, results, errors} JSON 字节。
 */
QByteArray resultsToJson(const QString &backend,
                         const QVector<markshot::plugin::CodeScanResult> &results)
{
    QJsonArray resultArray;
    for (const markshot::plugin::CodeScanResult &result : results) {
        QJsonObject object;
        object.insert(QStringLiteral("format"), result.format);
        object.insert(QStringLiteral("text"), result.text);
        QJsonArray points;
        for (const QPointF &point : result.points) {
            points.append(QJsonArray{point.x(), point.y()});
        }
        object.insert(QStringLiteral("points"), points);
        resultArray.append(object);
    }
    QJsonObject root;
    root.insert(QStringLiteral("backend"), backend);
    root.insert(QStringLiteral("results"), resultArray);
    root.insert(QStringLiteral("errors"), QJsonArray());
    return QJsonDocument(root).toJson(QJsonDocument::Compact);
}

#ifdef HAVE_ZXING_CPP

/**
 * 用 zxing-cpp 扫描图像。
 * @param imagePath 输入图像路径。
 * @return 任务结果。
 */
TaskResult scanWithZxing(const QString &imagePath)
{
    // 1. 灰度化后交给 zxing，Lum 格式在各版本间行为最稳定
    const QImage image = QImage(imagePath).convertToFormat(QImage::Format_Grayscale8);
    if (image.isNull()) {
        return {false, TaskError::Failed, {}, QByteArrayLiteral("cannot load image for code scan"), {}};
    }

    ZXing::ImageView view(image.constBits(),
                          image.width(),
                          image.height(),
                          ZXing::ImageFormat::Lum,
                          static_cast<int>(image.bytesPerLine()));
    ZXing::ReaderOptions options;
    options.setTryHarder(true);
    options.setTryRotate(true);
    const auto barcodes = ZXing::ReadBarcodes(view, options);

    QVector<markshot::plugin::CodeScanResult> results;
    for (const auto &barcode : barcodes) {
        markshot::plugin::CodeScanResult result;
        result.format = QString::fromStdString(ZXing::ToString(barcode.format()));
        result.text = QString::fromStdString(barcode.text());
        if (result.text.trimmed().isEmpty()) {
            continue;
        }
        const auto &position = barcode.position();
        for (const auto &point : {position.topLeft(), position.topRight(),
                                  position.bottomRight(), position.bottomLeft()}) {
            result.points.append(QPointF(point.x, point.y));
        }
        results.append(result);
    }
    return {true, TaskError::None, resultsToJson(QStringLiteral("zxing-cpp"), results), {}, {}};
}

#endif

}  // namespace

CodeScanZxingTask::CodeScanZxingTask(QString imagePath, QObject *parent)
    : ProviderTask(QStringLiteral("zxing-cpp"), parent)
    , m_imagePath(std::move(imagePath))
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

bool CodeScanZxingTask::available()
{
#ifdef HAVE_ZXING_CPP
    return true;
#else
    return false;
#endif
}

void CodeScanZxingTask::start(int timeoutMs)
{
#ifndef HAVE_ZXING_CPP
    Q_UNUSED(timeoutMs)
    emitFinished({false, TaskError::StartFailed, {}, QByteArrayLiteral("zxing-cpp is not built in"), {}});
#else
    if (timeoutMs > 0) {
        m_timeoutTimer.start(timeoutMs);
    }
    const QString imagePath = m_imagePath;
    m_watcher.setFuture(QtConcurrent::run([imagePath] { return scanWithZxing(imagePath); }));
#endif
}

void CodeScanZxingTask::cancel()
{
    m_timeoutTimer.stop();
    m_watcher.disconnect(this);
}

CodeScanPluginTask::CodeScanPluginTask(markshot::plugin::CodeScanProviderPlugin *plugin,
                                       QString imagePath,
                                       QObject *parent)
    : ProviderTask(plugin ? plugin->providerId() : QStringLiteral("plugin"), parent)
    , m_plugin(plugin)
    , m_imagePath(std::move(imagePath))
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

void CodeScanPluginTask::start(int timeoutMs)
{
    if (!m_plugin) {
        emitFinished({false, TaskError::StartFailed, {}, QByteArrayLiteral("code scan plugin is missing"), {}});
        return;
    }
    if (timeoutMs > 0) {
        m_timeoutTimer.start(timeoutMs);
    }

    markshot::plugin::CodeScanProviderPlugin *plugin = m_plugin;
    const QString imagePath = m_imagePath;
    m_watcher.setFuture(QtConcurrent::run([plugin, imagePath]() -> TaskResult {
        const QImage image(imagePath);
        if (image.isNull()) {
            return {false, TaskError::Failed, {}, QByteArrayLiteral("cannot load image for code scan"), {}};
        }
        QVector<markshot::plugin::CodeScanResult> results;
        QString error;
        if (!plugin->scan(image, &results, &error)) {
            return {false, TaskError::Failed, {}, error.toUtf8(), {}};
        }
        return {true, TaskError::None, resultsToJson(plugin->providerId(), results), {}, {}};
    }));
}

void CodeScanPluginTask::cancel()
{
    m_timeoutTimer.stop();
    m_watcher.disconnect(this);
}

}  // namespace markshot::providers
