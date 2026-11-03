#include "gui_memory_scenarios.h"
#include "memory/image_allocation_policy.h"
#include "support/isolated_config.h"
#include "support/process_memory_sample.h"

#include <QApplication>
#include <QCommandLineParser>
#include <QElapsedTimer>
#include <QJsonArray>
#include <QJsonDocument>
#include <QScreen>
#include <QTemporaryDir>
#include <QTextStream>
#include <QWidget>

#include <algorithm>
#include <cmath>

#ifdef __GLIBC__
#if __GLIBC_PREREQ(2, 33)
#define MARK_SHOT_HAVE_MALLINFO2
#include <malloc.h>
#endif
#endif

/** 在隔离配置下执行 argc/argv 指定的 GUI 场景，输出内存 JSON；场景失败或超过预算时返回非零。 */
int main(int argc, char **argv)
{
    markshot::memory::configureImageAllocationPolicy();
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 2;
    if (qEnvironmentVariable("QT_QPA_PLATFORM").startsWith(QStringLiteral("offscreen"))) {
        qputenv("XDG_SESSION_TYPE", "offscreen");
        for (const char *name : {"WAYLAND_DISPLAY", "DISPLAY", "NIRI_SOCKET", "SWAYSOCK"}) qunsetenv(name);
    }
    QApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("mark-shot-gui-memory-benchmark"));
    app.setStyle(QStringLiteral("Fusion"));
    app.setQuitOnLastWindowClosed(false);
    QCommandLineParser parser;
    parser.setApplicationDescription(QStringLiteral("Measure real GUI workflows without desktop capture or working-set trimming"));
    parser.addHelpOption();
    parser.addOptions({
        {{QStringLiteral("scenario")}, QStringLiteral("capture, annotate, edit, font-panel, pin, pin-edit, pin-edit-hidpi, settings or cycle"), QStringLiteral("name"), QStringLiteral("capture")},
        {{QStringLiteral("width")}, QStringLiteral("Source image width"), QStringLiteral("pixels"), QStringLiteral("3840")},
        {{QStringLiteral("height")}, QStringLiteral("Source image height"), QStringLiteral("pixels"), QStringLiteral("2160")},
        {{QStringLiteral("strokes")}, QStringLiteral("Number of annotation strokes"), QStringLiteral("count"), QStringLiteral("20")},
        {{QStringLiteral("zoom-passes")}, QStringLiteral("Consecutive zoom levels before restoring the image"), QStringLiteral("count"), QStringLiteral("1")},
        {{QStringLiteral("iterations")}, QStringLiteral("Repeat the selected workflow"), QStringLiteral("count"), QStringLiteral("1")},
        {{QStringLiteral("max-peak-mib")}, QStringLiteral("Fail above this process peak resident budget"), QStringLiteral("MiB")},
    });
    parser.process(app);
    const QSize size(parser.value(QStringLiteral("width")).toInt(), parser.value(QStringLiteral("height")).toInt());
    const int strokes = parser.value(QStringLiteral("strokes")).toInt();
    const int zoomPasses = parser.value(QStringLiteral("zoom-passes")).toInt();
    const int iterations = parser.value(QStringLiteral("iterations")).toInt();
    const QString scenario = parser.value(QStringLiteral("scenario"));
    const QStringList scenarios{QStringLiteral("capture"), QStringLiteral("annotate"), QStringLiteral("edit"), QStringLiteral("font-panel"),
                                QStringLiteral("pin"), QStringLiteral("pin-edit"), QStringLiteral("pin-edit-hidpi"),
                                QStringLiteral("settings")};
    if (size.width() < 320 || size.height() < 240 || size.width() > 8192 || size.height() > 8192
        || strokes < 1 || strokes > 200 || zoomPasses < 1 || zoomPasses > 20 || iterations < 1 || iterations > 20
        || (!scenarios.contains(scenario) && scenario != QStringLiteral("cycle"))) parser.showHelp(2);
    const qreal dpr = app.primaryScreen()->devicePixelRatio();
    const QSize viewport(qRound(size.width() / dpr), qRound(size.height() / dpr));
    QJsonArray samples;
    quint64 peak = 0;
    QString failure;
    QElapsedTimer timer;
    timer.start();
    const auto checkpoint = [&](const QString &phase) {
        const auto memory = markshot::test::processMemorySample();
        QJsonObject sample = memory.json();
        sample.insert(QStringLiteral("phase"), phase);
        sample.insert(QStringLiteral("elapsed_ms"), timer.elapsed());
        sample.insert(QStringLiteral("widgets"), QApplication::allWidgets().size());
        // 1. 【GUI测量】【场景核对】记录实际可见窗口像素数，防止插件或屏幕差异改变对照工作量
        qint64 windowPixels = 0;
        for (const QWidget *window : QApplication::topLevelWidgets()) {
            if (!window->isVisible()) continue;
            const qreal scale = window->devicePixelRatioF();
            windowPixels += qRound64(window->width() * scale) * qRound64(window->height() * scale);
        }
        sample.insert(QStringLiteral("visible_window_pixels"), windowPixels);
#ifdef MARK_SHOT_HAVE_MALLINFO2
        // 1. 【GUI测量】【分配器】区分仍在使用的堆与已释放但尚未归还系统的空闲块
        const auto heap = mallinfo2();
        sample.insert(QStringLiteral("heap_in_use_bytes"), static_cast<qint64>(heap.uordblks + heap.hblkhd));
        sample.insert(QStringLiteral("heap_free_bytes"), static_cast<qint64>(heap.fordblks));
#endif
        samples.append(sample);
        peak = std::max({peak, memory.resident, memory.peakResident});
    };
    checkpoint(QStringLiteral("before"));
    bool passed = true;
    for (int iteration = 0; iteration < iterations && passed; ++iteration) {
        for (const QString &name : (scenario == QStringLiteral("cycle") ? scenarios : QStringList{scenario})) {
            passed = markshot::test::runGuiMemoryScenario(name, size, viewport, strokes, zoomPasses,
                [&](const QString &phase) { checkpoint(QStringLiteral("%1:%2:%3").arg(iteration).arg(name, phase)); },
                &failure);
            if (!passed) break;
        }
    }
    checkpoint(QStringLiteral("released"));
    if (parser.isSet(QStringLiteral("max-peak-mib"))) {
        bool valid = false;
        const double budget = parser.value(QStringLiteral("max-peak-mib")).toDouble(&valid) * 1024 * 1024;
        passed &= valid && std::isfinite(budget) && budget > 0 && peak > 0 && peak <= budget;
    }
    QJsonObject result{{QStringLiteral("scenario"), scenario}, {QStringLiteral("width"), size.width()},
                        {QStringLiteral("runtime_directory"), QCoreApplication::applicationDirPath()},
                        {QStringLiteral("height"), size.height()}, {QStringLiteral("dpr"), dpr},
                        {QStringLiteral("iterations"), iterations}, {QStringLiteral("strokes"), strokes},
                        {QStringLiteral("zoom_passes"), zoomPasses},
                        {QStringLiteral("peak_resident_bytes"), static_cast<qint64>(peak)},
                        {QStringLiteral("elapsed_ms"), timer.elapsed()}, {QStringLiteral("samples"), samples},
                        {QStringLiteral("error"), failure}, {QStringLiteral("passed"), passed}};
#if defined(Q_OS_LINUX) && defined(__GLIBC__)
    result.insert(QStringLiteral("allocation_policy"), QStringLiteral("production-glibc-mmap-1mib"));
#else
    result.insert(QStringLiteral("allocation_policy"), QStringLiteral("platform-default"));
#endif
#ifdef Q_OS_WIN
    result.insert(QStringLiteral("private_metric"), QStringLiteral("commit"));
#else
    result.insert(QStringLiteral("private_metric"), QStringLiteral("resident"));
#endif
    QTextStream(stdout) << QJsonDocument(result).toJson(QJsonDocument::Indented);
    return passed ? 0 : 1;
}
