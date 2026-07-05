#include "zxing_code_scan_plugin.h"

#include <QtTest/QtTest>

#include <QImage>

#include <ZXing/BarcodeFormat.h>
#include <ZXing/CreateBarcode.h>
#include <ZXing/ImageView.h>
#include <ZXing/WriteBarcode.h>

using namespace markshot::code_scan_zxing;

namespace {

/**
 * 渲染二维码测试图像。
 * @param text 二维码内容。
 * @return 黑白二维码图像。
 */
QImage renderQrImage(const QString &text)
{
    const ZXing::Barcode barcode =
        ZXing::CreateBarcodeFromText(text.toStdString(), ZXing::CreatorOptions(ZXing::BarcodeFormat::QRCode));
    const ZXing::Image image = ZXing::WriteBarcodeToImage(barcode, ZXing::WriterOptions().scale(8).addQuietZones(true));
    return QImage(image.data(),
                  image.width(),
                  image.height(),
                  image.rowStride(),
                  QImage::Format_Grayscale8)
        .copy()
        .convertToFormat(QImage::Format_ARGB32);
}

}  // namespace

class CodeScanZxingPluginTest : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证 zxing-cpp 插件能识别二维码。
     * @return 无返回值。
     */
    void scansQrCode()
    {
        ZxingCodeScanPlugin plugin;
        QString error;
        QVERIFY(plugin.isAvailable(&error));

        QVector<markshot::plugin::CodeScanResult> results;
        QVERIFY2(plugin.scan(renderQrImage(QStringLiteral("mark-shot-plugin")), &results, &error),
                 qPrintable(error));
        QVERIFY(!results.isEmpty());
        QCOMPARE(results.first().text, QStringLiteral("mark-shot-plugin"));
        QVERIFY(!results.first().points.isEmpty());
    }
};

QTEST_MAIN(CodeScanZxingPluginTest)
#include "code_scan_zxing_plugin_test.moc"
