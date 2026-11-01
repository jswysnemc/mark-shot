#include "recording/recording_async_writer.h"
#include "recording/recording_bgra_buffer_pool.h"
#include "recording/recording_quality_options.h"
#include "support/process_memory_sample.h"

#include <QCommandLineParser>
#include <QCoreApplication>
#include <QElapsedTimer>
#include <QEventLoop>
#include <QFileInfo>
#include <QJsonDocument>
#include <QTemporaryDir>
#include <QTextStream>
#include <QThread>

#include <algorithm>
#include <cmath>
#include <functional>

using namespace markshot::recording;

namespace {

/** 在 timeoutMs 内处理事件直到 ready 成立，返回是否成功，避免堵塞写出完成通知。 */
bool waitUntil(const std::function<bool()> &ready, int timeoutMs)
{
    QElapsedTimer timer;
    timer.start();
    while (!ready() && timer.elapsed() < timeoutMs) {
        QCoreApplication::processEvents(QEventLoop::AllEvents, 5);
        QThread::msleep(1);
    }
    return ready();
}

/** 用 pool 生成尺寸为 size、编号为 index、帧率为 fps 的固定图案帧，返回共享 BGRA 载荷。 */
RecordingFrameSample createFrame(RecordingBgraBufferPool &pool, QSize size, int index, int fps)
{
    QByteArray *slot = nullptr;
    if (!waitUntil([&] {
            return slot || (slot = pool.tryAcquire(recordingBgraFrameBytes(size))) != nullptr;
        }, 30000)) {
        return {};
    }
    QByteArray &buffer = *slot;
    auto *pixels = reinterpret_cast<QRgb *>(buffer.data());
    for (int y = 0; y < size.height(); ++y) {
        for (int x = 0; x < size.width(); ++x) {
            pixels[y * size.width() + x] = qRgb((x / 16 + index * 3) & 255,
                                                (y / 16 + index) & 255, ((x + y) / 64) & 255);
        }
    }
    RecordingFrameSample sample;
    sample.bgra.bytes = buffer;
    sample.bgra.size = size;
    sample.bgra.stride = size.width() * 4;
    sample.sequence = index + 1;
    sample.timestampMs = static_cast<qint64>(index) * 1000 / fps;
    return sample;
}

/** 按 size、fps、count 和 quality 执行异步编码，realtime 控制实时供帧，写入 output 并返回测量报告。 */
QJsonObject measureRecording(QSize size, int fps, int count, RecordingQuality quality,
                             const QString &output, bool realtime)
{
    using markshot::test::processMemorySample;
    QJsonObject report{{QStringLiteral("before"), processMemorySample().json()}};
    QString failure;
    bool finished = false;
    bool succeeded = false;
    int submitted = 0;
    int lateFrames = 0;
    qint64 maximumLagMs = 0;
    QElapsedTimer timer;
    timer.start();
    {
        RecordingBgraBufferPool pool;
        RecordingOptions options;
        options.mode = RecordingMode::Video;
        options.quality = quality;
        options.outputPath = output;
        RecordingAsyncWriter writer(options);
        bool pressured = false;
        QObject::connect(&writer, &RecordingAsyncWriter::backpressureChanged,
                         &writer, [&pressured](bool active) { pressured = active; });
        QObject::connect(&writer, &RecordingAsyncWriter::failed,
                         &writer, [&failure](const QString &error) { failure = error; });
        QObject::connect(&writer, &RecordingAsyncWriter::finished,
                         &writer, [&](bool ok, const QString &error) {
            finished = true;
            succeeded = ok;
            if (!ok) failure = error;
        });
        if (writer.start(size, fps, &failure)) {
            report.insert(QStringLiteral("started"), processMemorySample().json());
            QElapsedTimer inputTimer;
            inputTimer.start();
            for (int index = 0; index < count && failure.isEmpty(); ++index) {
                // 1. 【录制测量】【帧队列】遵守生产背压，不把测试主动丢帧当成优化收益
                const qint64 dueMs = static_cast<qint64>(index) * 1000 / fps;
                if (!waitUntil([&] {
                        return (!pressured && (!realtime || inputTimer.elapsed() >= dueMs))
                            || !failure.isEmpty();
                    }, 30000)) {
                    failure = QStringLiteral("Recording queue did not drain");
                    break;
                }
                if (!failure.isEmpty()) break;
                const auto sample = createFrame(pool, size, index, fps);
                if (!writer.writeFrame(sample, &failure)) break;
                ++submitted;
                if (realtime) {
                    const qint64 lag = inputTimer.elapsed() - static_cast<qint64>(index + 1) * 1000 / fps;
                    maximumLagMs = std::max(maximumLagMs, lag);
                    lateFrames += lag > 0 ? 1 : 0;
                }
                QCoreApplication::processEvents(QEventLoop::AllEvents, 1);
            }
            report.insert(QStringLiteral("input_elapsed_ms"), inputTimer.elapsed());
            report.insert(QStringLiteral("frames_submitted"), processMemorySample().json());
            report.insert(QStringLiteral("pool_retained_bytes"), pool.retainedCapacity());
            if (failure.isEmpty() && writer.finish(&failure)) {
                if (!waitUntil([&] { return finished || !failure.isEmpty(); }, 30000)) {
                    failure = QStringLiteral("Recording did not finish");
                }
            }
        }
        report.insert(QStringLiteral("finished_with_pool"), processMemorySample().json());
    }
    report.insert(QStringLiteral("released"), processMemorySample().json());
    // 2. 【录制测量】【峰值统计】使用全部快照的最大值，避免单次系统计数取整掩盖先前峰值
    qint64 peak = 0;
    for (const QJsonValue &value : report) {
        const QJsonObject sample = value.toObject();
        peak = std::max({peak, sample.value(QStringLiteral("resident_bytes")).toInteger(),
                         sample.value(QStringLiteral("peak_resident_bytes")).toInteger()});
    }
    report.insert(QStringLiteral("peak_resident_bytes"), peak);
    report.insert(QStringLiteral("elapsed_ms"), timer.elapsed());
    report.insert(QStringLiteral("realtime"), realtime);
    report.insert(QStringLiteral("late_frames"), lateFrames);
    report.insert(QStringLiteral("maximum_submission_lag_ms"), maximumLagMs);
    report.insert(QStringLiteral("submitted_frames"), submitted);
    report.insert(QStringLiteral("output_bytes"), QFileInfo(output).size());
    report.insert(QStringLiteral("error"), failure);
    report.insert(QStringLiteral("passed"), succeeded && failure.isEmpty() && submitted == count);
    return report;
}

}  // namespace

/** 解析 argc/argv 后测量录制，输出 JSON；编码失败或峰值超过预算时返回非零。 */
int main(int argc, char **argv)
{
    QCoreApplication app(argc, argv);
    QCommandLineParser parser;
    parser.setApplicationDescription(QStringLiteral("Measure real recording buffers and encoder memory without desktop capture"));
    parser.addHelpOption();
    parser.addOptions({
        {{QStringLiteral("width")}, QStringLiteral("Frame width"), QStringLiteral("pixels"), QStringLiteral("1920")},
        {{QStringLiteral("height")}, QStringLiteral("Frame height"), QStringLiteral("pixels"), QStringLiteral("1080")},
        {{QStringLiteral("fps")}, QStringLiteral("Video frame rate"), QStringLiteral("fps"), QStringLiteral("30")},
        {{QStringLiteral("frames")}, QStringLiteral("Number of input frames"), QStringLiteral("count"), QStringLiteral("48")},
        {{QStringLiteral("quality")}, QStringLiteral("efficient, balanced or high"), QStringLiteral("profile"), QStringLiteral("balanced")},
        {{QStringLiteral("output")}, QStringLiteral("Keep the MP4 at this path"), QStringLiteral("path")},
        {{QStringLiteral("realtime")}, QStringLiteral("Pace input frames at the requested frame rate")},
        {{QStringLiteral("max-peak-mib")}, QStringLiteral("Fail above the peak resident memory budget"), QStringLiteral("MiB")},
    });
    parser.process(app);
    const QSize size(parser.value(QStringLiteral("width")).toInt(), parser.value(QStringLiteral("height")).toInt());
    const int fps = parser.value(QStringLiteral("fps")).toInt();
    const int count = parser.value(QStringLiteral("frames")).toInt();
    const QString quality = parser.value(QStringLiteral("quality"));
    if (size.width() < 2 || size.height() < 2 || size.width() > 8192 || size.height() > 8192
        || fps < 1 || fps > 240 || count < 1 || count > 3600
        || !QStringList{QStringLiteral("efficient"), QStringLiteral("balanced"), QStringLiteral("high")}.contains(quality)) {
        parser.showHelp(2);
    }
    QTemporaryDir directory;
    if (!directory.isValid()) return 2;
    qputenv("MARK_SHOT_RECORDING_SW_ENCODER", "1");
    const QString output = parser.isSet(QStringLiteral("output"))
        ? parser.value(QStringLiteral("output")) : directory.filePath(QStringLiteral("sample.mp4"));
    QJsonObject result = measureRecording(size, fps, count, recordingQualityFromName(quality), output,
                                          parser.isSet(QStringLiteral("realtime")));
    result.insert(QStringLiteral("width"), size.width());
    result.insert(QStringLiteral("height"), size.height());
    result.insert(QStringLiteral("fps"), fps);
    result.insert(QStringLiteral("quality"), quality);
    result.insert(QStringLiteral("ideal_threads"), QThread::idealThreadCount());
#ifdef Q_OS_WIN
    result.insert(QStringLiteral("private_metric"), QStringLiteral("commit"));
#else
    result.insert(QStringLiteral("private_metric"), QStringLiteral("resident"));
#endif
    if (parser.isSet(QStringLiteral("max-peak-mib"))) {
        bool valid = false;
        const double budget = parser.value(QStringLiteral("max-peak-mib")).toDouble(&valid) * 1024 * 1024;
        const double peak = result.value(QStringLiteral("peak_resident_bytes")).toDouble();
        if (!valid || !std::isfinite(budget) || budget <= 0 || peak <= 0 || peak > budget) {
            result.insert(QStringLiteral("passed"), false);
        }
    }
    QTextStream(stdout) << QJsonDocument(result).toJson(QJsonDocument::Indented);
    return result.value(QStringLiteral("passed")).toBool() ? 0 : 1;
}
