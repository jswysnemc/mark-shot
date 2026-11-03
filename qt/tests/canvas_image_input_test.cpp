#include "canvas/canvas_image_input.h"

#include <QColorSpace>
#include <QPainter>
#include <QtTest/QtTest>

class CanvasImageInputTest final : public QObject {
    Q_OBJECT

private slots:
    /** 【画布测试】【图像格式】为 Qt 测试框架提供可共享格式、独占状态和 DPR，无参数和返回值。 */
    void preservesPixelsAndMetadata_data()
    {
        QTest::addColumn<int>("format");
        QTest::addColumn<bool>("shared");
        QTest::addColumn<qreal>("dpr");
        for (int format : {QImage::Format_RGB32, QImage::Format_ARGB32_Premultiplied}) {
            for (bool shared : {false, true}) {
                for (qreal dpr : {1.0, 1.5, 2.0}) {
                    const QByteArray name = QByteArray::number(format) + '-' + QByteArray::number(shared)
                        + '-' + QByteArray::number(dpr);
                    QTest::newRow(name.constData()) << format << shared << dpr;
                }
            }
        }
    }

    /** 【画布测试】【元数据隔离】所有可共享输入复用像素并保留非 DPR 元数据，无参数和返回值。 */
    void preservesPixelsAndMetadata()
    {
        QFETCH(int, format);
        QFETCH(bool, shared);
        QFETCH(qreal, dpr);
        QImage input(64, 48, static_cast<QImage::Format>(format));
        input.fill(QColor(60, 90, 120, format == QImage::Format_RGB32 ? 255 : 128));
        input.setDevicePixelRatio(dpr);
        input.setColorSpace(QColorSpace(QColorSpace::DisplayP3));
        input.setDotsPerMeterX(4200);
        input.setDotsPerMeterY(4300);
        input.setOffset(QPoint(10, -20));
        input.setText(QStringLiteral("Description"), QStringLiteral("fixture"));
        input.setText(QStringLiteral("Author"), QStringLiteral("test"));
        const auto *pixels = input.constBits();
        const QRgb expected = input.pixel(0, 0);
        QImage retained = shared ? input : QImage();
        const QImage result = markshot::canvas::canvasImageInput(std::move(input));
        QCOMPARE(result.constBits(), pixels);
        QCOMPARE(result.format(), static_cast<QImage::Format>(format));
        QCOMPARE(result.pixel(0, 0), expected);
        QCOMPARE(result.devicePixelRatio(), 1.0);
        QCOMPARE(result.colorSpace(), QColorSpace(QColorSpace::DisplayP3));
        QCOMPARE(result.dotsPerMeterX(), 4200);
        QCOMPARE(result.dotsPerMeterY(), 4300);
        QCOMPARE(result.offset(), QPoint(10, -20));
        QCOMPARE(result.text(QStringLiteral("Description")), QStringLiteral("fixture"));
        QCOMPARE(result.text(QStringLiteral("Author")), QStringLiteral("test"));
        if (shared) {
            QCOMPARE(retained.devicePixelRatio(), dpr);
            retained.fill(Qt::black);
            QCOMPARE(result.pixel(0, 0), expected);
        }
    }

    /** 【画布测试】【缓冲区寿命】带行填充的外部只读原图由最后一个视图释放，且写入时隔离，无参数和返回值。 */
    void readonlyStrideLifetimeAndCopyOnWrite()
    {
        int releases = 0;
        QByteArray bytes(32 * 2, char(0));
        const QRgb expected = qPremultiply(qRgba(40, 80, 120, 128));
        for (int y = 0; y < 2; ++y) {
            auto *row = reinterpret_cast<QRgb *>(bytes.data() + y * 32);
            for (int x = 0; x < 3; ++x) row[x] = expected;
        }
        QImage source(reinterpret_cast<const uchar *>(bytes.constData()), 3, 2, 32,
                      QImage::Format_ARGB32_Premultiplied,
                      [](void *counter) { ++*static_cast<int *>(counter); }, &releases);
        source.setDevicePixelRatio(2.0);
        QImage first = markshot::canvas::canvasImageInput(source);
        QImage second = markshot::canvas::canvasImageInput(source);
        QCOMPARE(first.constBits(), source.constBits());
        QCOMPARE(first.bytesPerLine(), qsizetype(32));
        QCOMPARE(second.constBits(), source.constBits());
        source = {};
        QCOMPARE(releases, 0);
        {
            QPainter painter(&first);
            painter.setCompositionMode(QPainter::CompositionMode_Source);
            painter.fillRect(first.rect(), Qt::blue);
        }
        QCOMPARE(first.pixelColor(0, 0), QColor(Qt::blue));
        QCOMPARE(second.pixel(0, 0), expected);
        QCOMPARE(second.pixel(2, 1), expected);
        QCOMPARE(releases, 0);
        QImage retained = second;
        second = {};
        QCOMPARE(releases, 0);
        retained = {};
        QCOMPARE(releases, 1);
        first = {};
        QCOMPARE(releases, 1);
    }

    /** 【画布测试】【空图】空输入不分配视图，返回空图，无参数和返回值。 */
    void emptyInput()
    {
        QVERIFY(markshot::canvas::canvasImageInput({}).isNull());
    }
};

QTEST_APPLESS_MAIN(CanvasImageInputTest)
#include "canvas_image_input_test.moc"
