#include "rapid_model_paths.h"
#include "markshot/ocr_provider_plugin.h"
#include "marketplace/plugin_installer.h"
#include "providers/provider_runtime_dependencies.h"

#include <QtTest/QtTest>

#include <QFont>
#include <QFontMetrics>
#include <QDir>
#include <QFile>
#include <QScopeGuard>
#include <QImage>
#include <QPainter>
#include <QPluginLoader>
#include <QTemporaryDir>

using namespace markshot::ocr_rapid;

namespace {

/**
 * 渲染一张黑字白底的文本图像。
 * @param text 需要渲染的文本。
 * @return 渲染后的图像。
 */
QImage renderTextImage(const QString &text)
{
    QImage image(480, 120, QImage::Format_ARGB32);
    image.fill(Qt::white);
    QPainter painter(&image);
    painter.setPen(Qt::black);
    QFont font = painter.font();
    font.setPixelSize(42);
    font.setBold(true);
    painter.setFont(font);
    painter.drawText(image.rect(), Qt::AlignCenter, text);
    painter.end();
    return image;
}

}  // namespace

class OcrRapidPluginTest : public QObject {
    Q_OBJECT

private:
    QTemporaryDir m_pluginDirectory;
    QPluginLoader m_loader;
    markshot::plugin::OcrProviderPlugin *m_plugin = nullptr;

private slots:
    /**
     * 【OCR】【插件加载】安装生产动态库并验证插件导出接口和运行时依赖
     * @return 无返回值，加载失败时立即报告具体错误
     */
    void initTestCase()
    {
        const QString overridePath = qEnvironmentVariable("MARK_SHOT_TEST_OCR_PLUGIN_PATH");
        const QString sourcePath = overridePath.isEmpty()
            ? QString::fromUtf8(MARK_SHOT_TEST_OCR_PLUGIN_PATH) : overridePath;
        // 1. 【OCR】【插件加载】使用市场安装器复制到独立目录，验证可分发的真实库文件
        QVERIFY(m_pluginDirectory.isValid());
        const auto installed = markshot::marketplace::installPluginAsset(
            {sourcePath, QString(), m_pluginDirectory.path(), QString()});
        QVERIFY2(installed.success, qPrintable(installed.error));
        // 2. 【OCR】【插件加载】从安装结果加载，不预先链接插件或 ONNX Runtime
        m_loader.setFileName(installed.installedPath);
        m_loader.setLoadHints(QLibrary::ResolveAllSymbolsHint);
        QObject *instance = markshot::providers::loadProviderPluginWithRuntimeFallback(&m_loader);
        QVERIFY2(instance, qPrintable(m_loader.errorString()));
        m_plugin = qobject_cast<markshot::plugin::OcrProviderPlugin *>(instance);
        QVERIFY2(m_plugin, "OCR provider interface is missing");
        QCOMPARE(m_plugin->providerId(), QStringLiteral("rapid-onnx"));
        if (qEnvironmentVariableIsSet("MARK_SHOT_TEST_REQUIRE_OCR_MODELS")) {
            QVERIFY2(locateRapidModels().isComplete(), "Required OCR test models are missing");
        }
    }

    /**
     * 【OCR】【插件清理】卸载动态库，使 Windows 也能删除测试安装目录
     * @return 无返回值
     */
    void cleanupTestCase()
    {
        m_plugin = nullptr;
        if (m_loader.isLoaded()) {
            QVERIFY2(m_loader.unload(), qPrintable(m_loader.errorString()));
        }
    }

    /**
     * 【OCR】【模型补齐】模型缺失时识别失败，补齐后同一插件实例能够重新加载
     * @return 无返回值
     */
    void recoversAfterModelsBecomeAvailable()
    {
        const auto models = locateRapidModels();
        if (!models.isComplete()) {
            QSKIP("PP-OCR models are not available on this machine");
        }
        const QByteArray oldDet = qgetenv("MARK_SHOT_RAPID_DET_MODEL");
        const QByteArray oldRec = qgetenv("MARK_SHOT_RAPID_REC_MODEL");
        const QByteArray oldDict = qgetenv("MARK_SHOT_RAPID_REC_DICT");
        const auto restore = qScopeGuard([&] {
            for (const auto &item : {qMakePair("MARK_SHOT_RAPID_DET_MODEL", oldDet),
                                     qMakePair("MARK_SHOT_RAPID_REC_MODEL", oldRec),
                                     qMakePair("MARK_SHOT_RAPID_REC_DICT", oldDict)}) {
                if (item.second.isNull()) qunsetenv(item.first);
                else qputenv(item.first, item.second);
            }
        });
        QTemporaryDir downloaded;
        QVERIFY(downloaded.isValid());
        const QString det = downloaded.filePath(QStringLiteral("ch_PP-OCRv5_det_mobile.onnx"));
        const QString rec = downloaded.filePath(QStringLiteral("ch_PP-OCRv5_rec_mobile.onnx"));
        const QString dict = downloaded.filePath(QStringLiteral("ppocrv5_dict.txt"));
        qputenv("MARK_SHOT_RAPID_DET_MODEL", det.toUtf8());
        qputenv("MARK_SHOT_RAPID_REC_MODEL", rec.toUtf8());
        qputenv("MARK_SHOT_RAPID_REC_DICT", dict.toUtf8());
        QString error;
        QVector<markshot::plugin::OcrToken> tokens;
        QVERIFY(!m_plugin->isAvailable(&error));
        QVERIFY(!m_plugin->recognize(renderTextImage(QStringLiteral("HELLO 123")), &tokens, &error));
        QVERIFY(QFile::copy(models.detModel, det));
        QVERIFY(QFile::copy(models.recModel, rec));
        QVERIFY(QFile::copy(models.recDictionary, dict));
        QVERIFY2(m_plugin->isAvailable(&error), qPrintable(error));
        QVERIFY(error.isEmpty());
        QVERIFY2(m_plugin->recognize(renderTextImage(QStringLiteral("HELLO 123")), &tokens, &error), qPrintable(error));
        QVERIFY(!tokens.isEmpty());
    }

    /**
     * 验证插件可用真实 PP-OCR 模型识别渲染文本。
     * @return 无返回值。
     */
    void recognizesRenderedText()
    {
        if (!locateRapidModels().isComplete()) {
            QSKIP("PP-OCR models are not available on this machine");
        }

        QString error;
        QVERIFY2(m_plugin->isAvailable(&error), qPrintable(error));

        QVector<markshot::plugin::OcrToken> tokens;
        QVERIFY2(m_plugin->recognize(renderTextImage(QStringLiteral("HELLO 123")), &tokens, &error),
                 qPrintable(error));
        QVERIFY(!tokens.isEmpty());

        QString combined;
        for (const markshot::plugin::OcrToken &token : tokens) {
            combined += token.text;
            combined += QLatin1Char(' ');
        }
        QVERIFY2(combined.contains(QStringLiteral("HELLO"), Qt::CaseInsensitive),
                 qPrintable(QStringLiteral("recognized: %1").arg(combined)));
        QVERIFY2(combined.contains(QStringLiteral("123")),
                 qPrintable(QStringLiteral("recognized: %1").arg(combined)));
    }

    /**
     * 验证中文渲染文本识别。
     * @return 无返回值。
     */
    void recognizesChineseText()
    {
        if (!locateRapidModels().isComplete()) {
            QSKIP("PP-OCR models are not available on this machine");
        }
        if (!QFontMetrics(QGuiApplication::font()).inFontUcs4(0x4F60)) {
            QSKIP("The test environment has no Chinese font");
        }

        QString error;
        QVector<markshot::plugin::OcrToken> tokens;
        QVERIFY2(m_plugin->recognize(renderTextImage(QStringLiteral("你好世界")), &tokens, &error),
                 qPrintable(error));

        QString combined;
        for (const markshot::plugin::OcrToken &token : tokens) {
            combined += token.text;
        }
        QVERIFY2(combined.contains(QStringLiteral("你好")),
                 qPrintable(QStringLiteral("recognized: %1").arg(combined)));
    }

    /** 【OCR测试】【运行库回收】保留插件实例并完成识别后不得继续映射 ONNX，无参数和返回值。 */
    void releasesRuntimeAfterRecognition()
    {
#ifdef Q_OS_LINUX
        if (!locateRapidModels().isComplete()) {
            QSKIP("PP-OCR models are not available on this machine");
        }
        QString error;
        QVector<markshot::plugin::OcrToken> tokens;
        QVERIFY2(m_plugin->recognize(renderTextImage(QStringLiteral("HELLO 123")), &tokens, &error),
                 qPrintable(error));
        QVERIFY(!tokens.isEmpty());
        QFile maps(QStringLiteral("/proc/self/maps"));
        QVERIFY(maps.open(QIODevice::ReadOnly));
        QVERIFY2(!maps.readAll().contains("libonnxruntime.so"),
                 "Completed OCR still retains the ONNX Runtime mapping");
        QVERIFY(m_loader.isLoaded());
#else
        QSKIP("This regression checks Linux mappings; other platforms use lifetime tests");
#endif
    }

    /** 【OCR测试】【损坏资源】覆盖检测、识别及字典加载失败，无参数和返回值。 */
    void recoversFromDamagedResources_data()
    {
        QTest::addColumn<QByteArray>("variable");
        QTest::newRow("detector") << QByteArray("MARK_SHOT_RAPID_DET_MODEL");
        QTest::newRow("recognizer") << QByteArray("MARK_SHOT_RAPID_REC_MODEL");
        QTest::newRow("dictionary") << QByteArray("MARK_SHOT_RAPID_REC_DICT");
    }

    /** 【OCR测试】【损坏资源】失败后的原生资源释放不能阻断同一插件下一次正常识别，无参数和返回值。 */
    void recoversFromDamagedResources()
    {
        QFETCH(QByteArray, variable);
        if (!locateRapidModels().isComplete()) QSKIP("PP-OCR models are not available on this machine");
        const QByteArray previous = qgetenv(variable.constData());
        const auto restore = [&] {
            if (previous.isNull()) qunsetenv(variable.constData());
            else qputenv(variable.constData(), previous);
        };
        const auto guard = qScopeGuard(restore);
        QTemporaryDir damaged;
        QVERIFY(damaged.isValid());
        const QString path = damaged.filePath(QStringLiteral("empty-model"));
        QFile file(path);
        QVERIFY(file.open(QIODevice::WriteOnly));
        file.close();
        qputenv(variable.constData(), path.toUtf8());
        QString error;
        QVector<markshot::plugin::OcrToken> tokens;
        QVERIFY(!m_plugin->recognize(renderTextImage(QStringLiteral("HELLO 123")), &tokens, &error));
        QVERIFY(!error.isEmpty());
        QVERIFY(tokens.isEmpty());
        restore();
        QVERIFY2(m_plugin->recognize(renderTextImage(QStringLiteral("HELLO 123")), &tokens, &error),
                 qPrintable(error));
        QVERIFY(!tokens.isEmpty());
    }

    /** 【OCR测试】【运行库恢复】缺失显式运行库时不可用，恢复配置后同一插件正常识别，无参数和返回值。 */
    void recoversAfterRuntimeBecomesAvailable()
    {
        if (!locateRapidModels().isComplete()) QSKIP("PP-OCR models are not available on this machine");
        const QByteArray previous = qgetenv("MARK_SHOT_RAPID_ONNX_RUNTIME");
        const auto restore = [&] {
            if (previous.isNull()) qunsetenv("MARK_SHOT_RAPID_ONNX_RUNTIME");
            else qputenv("MARK_SHOT_RAPID_ONNX_RUNTIME", previous);
        };
        const auto guard = qScopeGuard(restore);
        qputenv("MARK_SHOT_RAPID_ONNX_RUNTIME", m_pluginDirectory.filePath(QStringLiteral("missing-runtime")).toUtf8());
        QString error;
        QVERIFY(!m_plugin->isAvailable(&error));
        QVERIFY(!error.isEmpty());
        QVector<markshot::plugin::OcrToken> tokens;
        QVERIFY(!m_plugin->recognize(renderTextImage(QStringLiteral("HELLO 123")), &tokens, &error));
        restore();
        QVERIFY2(m_plugin->isAvailable(&error), qPrintable(error));
        QVERIFY2(m_plugin->recognize(renderTextImage(QStringLiteral("HELLO 123")), &tokens, &error),
                 qPrintable(error));
        QVERIFY(!tokens.isEmpty());
    }
};

QTEST_MAIN(OcrRapidPluginTest)
#include "ocr_rapid_plugin_test.moc"
