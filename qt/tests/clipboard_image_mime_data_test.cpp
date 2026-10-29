#include "clipboard_image_mime_data.h"

#include <QBuffer>
#include <QImage>
#include <QtEndian>
#include <QtTest/QtTest>

namespace {

/**
 * 生成带透明像素的测试 PNG。
 * @return PNG 字节。
 */
QByteArray samplePng()
{
    QImage image(7, 5, QImage::Format_ARGB32);
    image.fill(QColor(20, 120, 220));
    image.setPixelColor(0, 0, QColor(255, 0, 0, 0));
    QByteArray png;
    QBuffer buffer(&png);
    buffer.open(QIODevice::WriteOnly);
    image.save(&buffer, "PNG");
    return png;
}

}  // namespace

class ClipboardImageMimeDataTest : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证只声明 PNG 与 BMP，且 PNG 在前、原样返回。
     * @return 无返回值。
     */
    void offersPngAndBmp()
    {
        const QByteArray png = samplePng();
        markshot::ClipboardImageMimeData data(png);
        QCOMPARE(data.formats(), (QStringList{QStringLiteral("image/png"), QStringLiteral("image/bmp")}));
        QVERIFY(data.hasFormat(QStringLiteral("image/bmp")));
        QVERIFY(!data.hasFormat(QStringLiteral("application/x-qt-image")));
        QCOMPARE(data.data(QStringLiteral("image/png")), png);
        QVERIFY(data.data(QStringLiteral("image/jpeg")).isEmpty());
    }

    /**
     * 验证 BMP 为 24 位且尺寸与像素保持一致。
     * @return 无返回值。
     */
    void convertsToTwentyFourBitBmp()
    {
        markshot::ClipboardImageMimeData data(samplePng());
        const QByteArray bmp = data.data(QStringLiteral("image/bmp"));
        QVERIFY(bmp.startsWith("BM"));
        QVERIFY(bmp.size() > 30);
        // 1. BITMAPINFOHEADER 的 biBitCount 位于文件偏移 28
        QCOMPARE(qFromLittleEndian<quint16>(reinterpret_cast<const uchar *>(bmp.constData()) + 28), quint16(24));
        QImage decoded;
        QVERIFY(decoded.loadFromData(bmp, "BMP"));
        QCOMPARE(decoded.size(), QSize(7, 5));
        QCOMPARE(decoded.pixelColor(3, 3), QColor(20, 120, 220));
    }

    /**
     * 验证损坏的 PNG 不会产生 BMP。
     * @return 无返回值。
     */
    void rejectsInvalidPng()
    {
        QVERIFY(markshot::bmpFromPng(QByteArrayLiteral("not a png")).isEmpty());
    }
};

QTEST_MAIN(ClipboardImageMimeDataTest)

#include "clipboard_image_mime_data_test.moc"
