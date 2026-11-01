#include "recording/gif_recording_writer.h"

#include <QFile>
#include <QImageReader>
#include <QTemporaryDir>
#include <QtTest/QtTest>

#include <cstring>

using namespace markshot::recording;

namespace {

/** 【录制测试】【GIF图案】按尺寸 size 和编号 index 生成渐变、细线及透明输入，返回图像。 */
QImage pattern(QSize size, int index)
{
    QImage image(size, QImage::Format_ARGB32);
    for (int y = 0; y < size.height(); ++y) {
        auto *row = reinterpret_cast<QRgb *>(image.scanLine(y));
        for (int x = 0; x < size.width(); ++x) {
            row[x] = x == index % size.width() || y == index % size.height()
                ? qRgba(255, 255, 255, 255)
                : qRgba((x * 13 + index * 41) & 255, (y * 17 + index * 19) & 255,
                        ((x + y) * 7 + index * 53) & 255, (x * 9 + y * 11) & 255);
        }
    }
    return image;
}

/** 【录制测试】【GIF载荷】把 image 按 layout 转为图像或带填充、倒置的原生样本，返回样本。 */
RecordingFrameSample sampleFor(const QImage &image, int layout)
{
    RecordingFrameSample sample;
    if (layout == 0) {
        sample.image = image;
        return sample;
    }
    sample.bgra.size = image.size();
    sample.bgra.stride = image.width() * 4 + 16;
    sample.bgra.yInverted = layout == 2;
    auto storage = std::make_shared<QByteArray>(sample.bgra.stride * image.height(), '\x7f');
    for (int y = 0; y < image.height(); ++y) {
        const int target = sample.bgra.yInverted ? image.height() - 1 - y : y;
        std::memcpy(storage->data() + target * sample.bgra.stride,
                    image.constScanLine(y), image.width() * 4);
    }
    if (layout == 1) {
        sample.bgra.bytes = *storage;
    } else {
        sample.bgra.mappedOwner = storage;
        sample.bgra.mappedData = storage->constData();
        sample.bgra.mappedSize = storage->size();
    }
    return sample;
}

}  // namespace

class RecordingGifRepeatTest final : public QObject {
    Q_OBJECT

private slots:
    /** 【录制测试】【GIF回读】缺少 GIF 插件时显式跳过，无参数和返回值。 */
    void initTestCase()
    {
        if (!QImageReader::supportedImageFormats().contains("gif")) {
            QSKIP("Qt GIF plugin is required to compare decoded frames and delays");
        }
    }

    /** 【录制测试】【GIF复用】列举连续变化、心跳、静态和不同载荷，无参数和返回值。 */
    void preservesDecodedFramesAndTiming_data()
    {
        QTest::addColumn<int>("layout");
        QTest::addColumn<int>("groupSize");
        QTest::addColumn<int>("fps");
        for (int layout = 0; layout < 3; ++layout) {
            for (int group : {1, 4, 24}) {
                for (int fps : {12, 30}) {
                    QTest::newRow(qPrintable(QStringLiteral("layout-%1-group-%2-fps-%3")
                                                .arg(layout).arg(group).arg(fps)))
                        << layout << group << fps;
                }
            }
        }
    }

    /** 【录制测试】【GIF复用】对照未知序号路径，逐帧检查 RGBA、帧延迟和完整文件，无参数和返回值。 */
    void preservesDecodedFramesAndTiming()
    {
        QFETCH(int, layout);
        QFETCH(int, groupSize);
        QFETCH(int, fps);
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        QString error;
        const QSize size(64, 48);
        const QStringList paths{directory.filePath("fresh.gif"), directory.filePath("cached.gif")};
        for (int cached = 0; cached < 2; ++cached) {
            RecordingOptions options;
            options.outputPath = paths[cached];
            GifRecordingWriter writer(options);
            QVERIFY2(writer.start(size, fps, &error), qPrintable(error));
            RecordingFrameSample sample;
            for (int i = 0; i < 24; ++i) {
                if (i % groupSize == 0) {
                    sample = sampleFor(pattern(size, i / groupSize), layout);
                }
                sample.sequence = cached ? i / groupSize + 1 : 0;
                // 1. 【录制测试】【GIF时间轴】混合正常帧间隔与静止心跳，保留长间隔而非固定延迟
                sample.timestampMs = static_cast<qint64>(i) * 1000 / fps + (i / 8) * 400;
                QVERIFY2(writer.writeFrame(sample, &error), qPrintable(error));
            }
            QVERIFY2(writer.finish(&error), qPrintable(error));
        }
        QImageReader fresh(paths[0]);
        QImageReader cached(paths[1]);
        int totalDelay = 0;
        for (int i = 0; i < 24; ++i) {
            const QImage expected = fresh.read().convertToFormat(QImage::Format_ARGB32);
            const QImage actual = cached.read().convertToFormat(QImage::Format_ARGB32);
            QVERIFY2(!expected.isNull(), qPrintable(fresh.errorString()));
            QVERIFY2(!actual.isNull(), qPrintable(cached.errorString()));
            QCOMPARE(actual, expected);
            QCOMPARE(cached.nextImageDelay(), fresh.nextImageDelay());
            totalDelay += cached.nextImageDelay();
        }
        QVERIFY(!fresh.canRead());
        QVERIFY(!cached.canRead());
        QVERIFY(totalDelay >= 23000 / fps + 800);
        QFile freshFile(paths[0]);
        QFile cachedFile(paths[1]);
        QVERIFY(freshFile.open(QIODevice::ReadOnly));
        QVERIFY(cachedFile.open(QIODevice::ReadOnly));
        QCOMPARE(cachedFile.readAll(), freshFile.readAll());
    }

    /** 【录制测试】【GIF会话】重启及取消不能跨会话复用相同序号，空样本仍失败，无参数和返回值。 */
    void clearsCacheBetweenSessions()
    {
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        LibavGifRecordingProcess process;
        QString error;
        QList<QImage> decoded;
        for (int session = 0; session < 3; ++session) {
            const QSize size(32 + session * 8, 24);
            const QString path = directory.filePath(QString::number(session) + ".gif");
            QVERIFY2(process.start(path, size, 10, &error), qPrintable(error));
            RecordingFrameSample sample;
            sample.image = QImage(size, QImage::Format_ARGB32);
            sample.image.fill(session == 0 ? Qt::red : Qt::blue);
            sample.sequence = 1;
            QVERIFY2(process.writeFrame(sample, &error), qPrintable(error));
            RecordingFrameSample empty;
            empty.sequence = 1;
            QVERIFY(!process.writeFrame(empty, &error));
            sample.timestampMs = 400;
            QVERIFY2(process.writeFrame(sample, &error), qPrintable(error));
            if (session == 1) {
                process.cancel();
                continue;
            }
            QVERIFY2(process.finish(&error), qPrintable(error));
            decoded.append(QImage(path));
            QVERIFY(!decoded.last().isNull());
            QCOMPARE(decoded.last().size(), size);
        }
        QVERIFY(decoded[0].pixelColor(0, 0).red() > decoded[0].pixelColor(0, 0).blue());
        QVERIFY(decoded[1].pixelColor(0, 0).blue() > decoded[1].pixelColor(0, 0).red());
    }
};

QTEST_APPLESS_MAIN(RecordingGifRepeatTest)
#include "recording_gif_repeat_test.moc"
