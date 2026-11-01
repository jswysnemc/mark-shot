#include "rapid_image_input.h"

#include <QColorSpace>
#include <QtTest>

using markshot::ocr_rapid::rapidImageInput;

class RapidImageInputTest final : public QObject {
    Q_OBJECT
private slots:
    /** 【OCR测试】【共享寿命】结果保留输入像素，源图写入或析构后仍有效，无参数和返回值。 */
    void sharesOpaquePixelsAndOwnsTheirLifetime()
    {
        QImage result;
        const QRgb original = qRgb(24, 57, 102);
        {
            QImage source(320, 180, QImage::Format_RGB32);
            source.fill(original);
            source.setDevicePixelRatio(2.0);
            source.setColorSpace(QColorSpace::SRgb);
            result = rapidImageInput(source);
            QCOMPARE(result.format(), QImage::Format_ARGB32);
            QCOMPARE(result.constBits(), source.constBits());
            QCOMPARE(result.devicePixelRatio(), 2.0);
            QCOMPARE(result.colorSpace(), source.colorSpace());
            source.fill(Qt::red);
            QCOMPARE(result.pixel(0, 0), original);
        }
        QCOMPARE(result.pixel(319, 179), original);
        QImage snapshot = result;
        result.fill(Qt::green);
        QCOMPARE(snapshot.pixel(319, 179), original);
    }

    /** 【OCR测试】【只读输入】修改转换结果必须与原始像素分离，无参数和返回值。 */
    void detachesBeforeWritingToResult()
    {
        QImage source(80, 64, QImage::Format_RGB32);
        source.fill(Qt::blue);
        QImage result = rapidImageInput(source);
        QCOMPARE(result.constBits(), source.constBits());
        result.setPixel(0, 0, qRgb(255, 0, 0));
        QCOMPARE(source.pixel(0, 0), qRgb(0, 0, 255));
        QVERIFY(result.constBits() != source.constBits());
    }

    /** 【OCR测试】【格式回退】准备 RGB、透明及预乘格式，无参数和返回值。 */
    void preservesPixelsAndScaling_data()
    {
        QTest::addColumn<int>("format");
        QTest::newRow("rgb32") << int(QImage::Format_RGB32);
        QTest::newRow("argb32") << int(QImage::Format_ARGB32);
        QTest::newRow("premultiplied") << int(QImage::Format_ARGB32_Premultiplied);
        QTest::newRow("rgb888") << int(QImage::Format_RGB888);
        QTest::newRow("rgba8888") << int(QImage::Format_RGBA8888);
    }

    /** 【OCR测试】【像素语义】完整像素和大小两种平滑缩放必须与原转换一致，无参数和返回值。 */
    void preservesPixelsAndScaling()
    {
        QFETCH(int, format);
        QImage pattern(961, 543, QImage::Format_ARGB32);
        for (int y = 0; y < pattern.height(); ++y) {
            QRgb *row = reinterpret_cast<QRgb *>(pattern.scanLine(y));
            for (int x = 0; x < pattern.width(); ++x) {
                row[x] = qRgba((x * 37 + y) % 256, (y * 71) % 256, (x + y * 13) % 256,
                              (x * 19 + y * 11) % 256);
            }
        }
        const QImage source = pattern.convertToFormat(QImage::Format(format));
        const QImage expected = source.convertToFormat(QImage::Format_ARGB32);
        const QImage actual = rapidImageInput(source);
        QCOMPARE(actual, expected);
        for (const QSize size : {QSize(416, 256), QSize(1200, 720)}) {
            QCOMPARE(actual.scaled(size, Qt::IgnoreAspectRatio, Qt::SmoothTransformation),
                     expected.scaled(size, Qt::IgnoreAspectRatio, Qt::SmoothTransformation));
        }
    }

    /** 【OCR测试】【缓冲归还】外部像素最后一份共享释放后才清理，行填充不参与透明检查，无参数和返回值。 */
    void preservesPaddedBufferAndCleanup()
    {
        int released = 0;
        struct Buffer {
            int *released;
            alignas(QRgb) uchar pixels[96] = {};
        };
        auto *buffer = new Buffer{&released};
        for (int y = 0; y < 3; ++y) {
            auto *row = reinterpret_cast<QRgb *>(buffer->pixels + y * 32);
            for (int x = 0; x < 5; ++x) row[x] = qRgb(23, y, x);
        }
        QImage source(buffer->pixels, 5, 3, 32, QImage::Format_RGB32, [](void *value) {
            auto *owned = static_cast<Buffer *>(value);
            ++*owned->released;
            delete owned;
        }, buffer);
        QImage result = rapidImageInput(source);
        QCOMPARE(result.constBits(), source.constBits());
        QCOMPARE(result.bytesPerLine(), 32);
        source = {};
        QCOMPARE(released, 0);
        QCOMPARE(result.pixel(4, 2), qRgb(23, 2, 4));
        result = {};
        QCOMPARE(released, 1);
    }

    /** 【OCR测试】【异常透明位】RGB32 中非不透明 alpha 必须沿用原转换归一化，无参数和返回值。 */
    void normalizesInvalidRgbAlpha()
    {
        QImage source(20, 10, QImage::Format_RGB32);
        source.fill(Qt::white);
        reinterpret_cast<QRgb *>(source.scanLine(4))[5] = qRgba(23, 56, 78, 0);
        const QImage expected = source.convertToFormat(QImage::Format_ARGB32);
        const QImage result = rapidImageInput(source);
        QCOMPARE(result, expected);
        QVERIFY(result.constBits() != source.constBits());
        QCOMPARE(result.pixel(5, 4), qRgb(23, 56, 78));
    }

    /** 【OCR测试】【空图像】空输入不创建像素所有者，无参数和返回值。 */
    void acceptsNullInput()
    {
        QVERIFY(rapidImageInput({}).isNull());
    }
};

QTEST_GUILESS_MAIN(RapidImageInputTest)
#include "rapid_image_input_test.moc"
