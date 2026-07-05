#include "rapid_model_paths.h"
#include "rapid_ocr_plugin.h"

#include <QtTest/QtTest>

#include <QFont>
#include <QImage>
#include <QPainter>

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

private slots:
    /**
     * 验证插件可用真实 PP-OCR 模型识别渲染文本。
     * @return 无返回值。
     */
    void recognizesRenderedText()
    {
        if (!locateRapidModels().isComplete()) {
            QSKIP("PP-OCR models are not available on this machine");
        }

        RapidOcrPlugin plugin;
        QString error;
        QVERIFY2(plugin.isAvailable(&error), qPrintable(error));

        QVector<markshot::plugin::OcrToken> tokens;
        QVERIFY2(plugin.recognize(renderTextImage(QStringLiteral("HELLO 123")), &tokens, &error),
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

        RapidOcrPlugin plugin;
        QString error;
        QVector<markshot::plugin::OcrToken> tokens;
        QVERIFY2(plugin.recognize(renderTextImage(QStringLiteral("你好世界")), &tokens, &error),
                 qPrintable(error));

        QString combined;
        for (const markshot::plugin::OcrToken &token : tokens) {
            combined += token.text;
        }
        QVERIFY2(combined.contains(QStringLiteral("你好")),
                 qPrintable(QStringLiteral("recognized: %1").arg(combined)));
    }
};

QTEST_MAIN(OcrRapidPluginTest)
#include "ocr_rapid_plugin_test.moc"
