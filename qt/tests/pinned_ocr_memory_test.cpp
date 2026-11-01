#include "app_config_store.h"
#include "memory/image_allocation_policy.h"
#include "pinned_window/pinned_image_window.h"
#include "providers/ocr/ocr_task_executor.h"
#include "providers/provider_plugin_paths.h"
#include "rapid_model_paths.h"
#include "support/isolated_config.h"
#include "support/process_memory_sample.h"

#include <QApplication>
#include <QFile>
#include <QJsonDocument>
#include <QPointer>
#include <QThreadPool>
#include <QTimer>
#include <QtTest>

class PinnedOcrMemoryTest final : public QObject {
    Q_OBJECT
    using Window = markshot::shot::PinnedImageWindow;
    QJsonArray samples;

    /** 【贴图测试】【合成图片】生成 seed 对应的独立文字图像，不读取用户内容，返回像素。 */
    QImage image(int seed) const
    {
        QImage result(1920, 1080, QImage::Format_RGB32);
        result.fill(Qt::white);
        QPainter painter(&result);
        QFont font = painter.font();
        font.setPixelSize(48);
        painter.setFont(font);
        painter.setPen(Qt::black);
        painter.drawText(QPoint(40, 100), QStringLiteral("Memory test %1 HELLO 123").arg(seed));
        return result;
    }

    /** 【贴图测试】【窗口创建】创建 seed 对应的真实贴图并显示，返回受保护窗口指针。 */
    QPointer<Window> openWindow(int seed)
    {
        QPointer<Window> window = new Window(image(seed), QPoint(40 + seed * 3, 40 + seed * 3));
        window->show();
        return window;
    }

    /** 【贴图测试】【内存记录】记录 phase 的 RSS 与内核高水位，不强制清空线程池或工作集，无返回值。 */
    void checkpoint(const QString &phase)
    {
        auto sample = markshot::test::processMemorySample().json();
        sample.insert(QStringLiteral("phase"), phase);
        samples.append(sample);
    }

private slots:
    /** 【贴图测试】【环境准备】检查真实模型并将测试插件装入隔离目录，无参数和返回值。 */
    void initTestCase()
    {
        if (!markshot::ocr_rapid::locateRapidModels().isComplete()) {
            if (qEnvironmentVariableIsSet("MARK_SHOT_TEST_REQUIRE_OCR_MODELS")) {
                QFAIL("Required OCR models are missing");
            }
            QSKIP("PP-OCR models are not available on this machine");
        }
        const QString plugins = markshot::providers::userPluginDirectory();
        QVERIFY(QDir().mkpath(plugins));
        const QString source = QString::fromUtf8(MARK_SHOT_TEST_OCR_PLUGIN_PATH);
        QVERIFY(QFile::copy(source, QDir(plugins).filePath(QFileInfo(source).fileName())));
        // 1. 【贴图测试】【私有运行库】真实注册器扫描同目录依赖，不能给非插件库添加永久加载标记
        const QString runtime = qEnvironmentVariable("MARK_SHOT_TEST_PRIVATE_RUNTIME");
        if (!runtime.isEmpty()) {
            const QString destination = QDir(plugins).filePath(QFileInfo(runtime).fileName());
            QVERIFY(QFile::copy(runtime, destination));
            qputenv("MARK_SHOT_RAPID_ONNX_RUNTIME", destination.toUtf8());
        }
        QVERIFY(markshot::writeAppConfigRoot({
            {QStringLiteral("ocr"), QJsonObject{{QStringLiteral("enabled"), true},
                                               {QStringLiteral("provider"), QStringLiteral("plugin:rapid-onnx")}}},
            {QStringLiteral("pinnedWindow"), QJsonObject{{QStringLiteral("autoOcr"), false},
                {QStringLiteral("alwaysOnTop"), false}, {QStringLiteral("textSelectionCopyEnabled"), true}}},
            {QStringLiteral("translation"), QJsonObject{{QStringLiteral("autoAfterOcr"), false}}}
        }));
        checkpoint(QStringLiteral("before"));
    }

    /** 【贴图测试】【多窗口】所有可见窗口均完成异步识别，文字仍可命中，事件循环继续处理计时器，无参数和返回值。 */
    void recognizesEveryVisibleWindow()
    {
        int ticks = 0;
        QTimer timer;
        timer.setInterval(10);
        connect(&timer, &QTimer::timeout, this, [&] { ++ticks; });
        timer.start();
        QVector<QPointer<Window>> windows;
        for (int i = 0; i < 4; ++i) windows.append(openWindow(i));
        checkpoint(QStringLiteral("four-open"));
        for (const auto &window : windows) {
            QTRY_VERIFY_WITH_TIMEOUT(window && !window->m_ocrTask && !window->m_ocrTokens.isEmpty(), 15000);
            QVERIFY(window->m_config.textSelectionCopyEnabled);
            QVERIFY(window->tokenAt(window->m_ocrTokens.first().imageRect.center()).has_value());
            QVERIFY(window->cursor().shape() != Qt::BusyCursor);
        }
        QVERIFY(ticks > 0);
        checkpoint(QStringLiteral("four-recognized"));
        for (const auto &window : windows) window->close();
        for (const auto &window : windows) QTRY_VERIFY_WITH_TIMEOUT(window.isNull(), 3000);
        checkpoint(QStringLiteral("four-closed"));
    }

    /** 【贴图测试】【快速关闭】已启动后台识别的窗口可关闭，随后打开的窗口仍能取得完整结果，无参数和返回值。 */
    void closesPendingWindowsWithoutBlockingLaterOcr()
    {
        QVector<QPointer<Window>> closed;
        for (int i = 0; i < 5; ++i) closed.append(openWindow(i));
        for (const auto &window : closed) {
            QTRY_VERIFY_WITH_TIMEOUT(window->m_textSelectionOcrAttempted, 3000);
        }
        for (const auto &window : closed) window->close();
        auto live = openWindow(8);
        QTRY_VERIFY_WITH_TIMEOUT(live && !live->m_ocrTask && !live->m_ocrTokens.isEmpty(), 15000);
        for (const auto &window : closed) QVERIFY(window.isNull());
        checkpoint(QStringLiteral("rapid-close-next-recognized"));
        live->close();
        QTRY_VERIFY_WITH_TIMEOUT(live.isNull(), 3000);
    }

    /** 【贴图测试】【重复开关】连续八轮真实识别后关闭窗口，采样并检查稳定内存不持续增长，无参数和返回值。 */
    void repeatedOpenCloseReleasesRuntime()
    {
        quint64 first = 0, last = 0;
        for (int i = 0; i < 8; ++i) {
            auto window = openWindow(i);
            QTRY_VERIFY_WITH_TIMEOUT(window && !window->m_ocrTask && !window->m_ocrTokens.isEmpty(), 15000);
            window->close();
            QTRY_VERIFY_WITH_TIMEOUT(window.isNull(), 3000);
            QTRY_COMPARE_WITH_TIMEOUT(markshot::providers::ocrPluginTaskThreadPool()->activeThreadCount(), 0, 3000);
            QTest::qWait(30);
            checkpoint(QStringLiteral("repeat-%1-closed").arg(i));
            last = markshot::test::processMemorySample().resident;
            if (i == 0) first = last;
#ifdef Q_OS_LINUX
            QFile maps(QStringLiteral("/proc/self/maps"));
            QVERIFY(maps.open(QIODevice::ReadOnly));
            QVERIFY(!maps.readAll().contains("libonnxruntime.so"));
#endif
        }
#ifdef Q_OS_LINUX
        QVERIFY2(last <= first + 8 * 1024 * 1024, "Closed-window memory grows across repeated OCR");
#endif
    }

    /** 【贴图测试】【清理】关闭失败用例遗留的测试窗口并等待现有任务自然完成，无参数和返回值。 */
    void cleanup()
    {
        for (QWidget *widget : QApplication::topLevelWidgets()) widget->close();
        QTRY_COMPARE_WITH_TIMEOUT(markshot::providers::ocrPluginTaskThreadPool()->activeThreadCount(), 0, 15000);
    }

    /** 【贴图测试】【结果输出】按可选环境变量保存采样 JSON，无参数和返回值。 */
    void cleanupTestCase()
    {
        const QString path = qEnvironmentVariable("MARK_SHOT_PIN_OCR_REPORT");
        if (path.isEmpty()) return;
        QFile file(path);
        QVERIFY(file.open(QIODevice::WriteOnly));
        QVERIFY(file.write(QJsonDocument(samples).toJson()) > 0);
    }
};

/** 【贴图测试】【独立进程】以 argc/argv 执行隔离配置的离屏测试，返回测试状态。 */
int main(int argc, char **argv)
{
    markshot::memory::configureImageAllocationPolicy();
    const bool wayland = qEnvironmentVariableIsSet("MARK_SHOT_PIN_TEST_WAYLAND");
    const QByteArray runtime = qgetenv("MARK_SHOT_RECORDING_TEST_RUNTIME");
    // 1. 【贴图测试】【桌面隔离】仅接受现有隔离会话启动器创建的绝对 socket，拒绝使用用户桌面
    if (wayland && (runtime.isEmpty() || !qgetenv("WAYLAND_DISPLAY").startsWith(runtime + '/'))) {
        return 1;
    }
    QTemporaryDir isolated;
    if (!markshot::test::isolateConfig(isolated)) return 1;
    if (wayland) {
        qputenv("XDG_RUNTIME_DIR", runtime);
        qputenv("QT_QPA_PLATFORM", "wayland");
    } else {
        qputenv("QT_QPA_PLATFORM", "offscreen");
        qputenv("XDG_SESSION_TYPE", "offscreen");
        qputenv("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent-mark-shot-test-bus");
        qunsetenv("WAYLAND_DISPLAY");
    }
    for (const char *key : {"DISPLAY", "WAYLAND_SOCKET", "NIRI_SOCKET", "SWAYSOCK",
                            "MARK_SHOT_OCR_COMMAND", "MARK_SHOT_PINNED_AUTO_OCR", "MARK_SHOT_OCR_DISABLED",
                            "MARK_SHOT_TRANSLATION_AUTO_AFTER_OCR", "MARK_SHOT_OCR_PYTHON"}) {
        qunsetenv(key);
    }
    QApplication app(argc, argv);
    app.setQuitOnLastWindowClosed(false);
    app.setStyle(QStringLiteral("Fusion"));
    PinnedOcrMemoryTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "pinned_ocr_memory_test.moc"
