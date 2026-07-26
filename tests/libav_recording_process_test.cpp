#include "recording/libav_recording_process.h"
#include "recording/libav_audio_encoder.h"
#include "recording/libav_error.h"
#include "recording/libav_gif_recording_process.h"
#include "recording/recording_encoder_probe.h"

#include <QtTest/QtTest>

#include <QFileInfo>
#include <QColor>
#include <QFile>
#include <QImage>
#include <QImageReader>
#include <QTemporaryDir>

#include <cstdlib>

#include <mutex>

extern "C" {
#include <libavformat/avformat.h>
}

class LibavRecordingProcessTest : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证库内 FFmpeg writer 可以生成基础视频文件。
     * @return 无返回值。
     */
    void writesSmallSoftwareEncodedVideo()
    {
        QTemporaryDir directory;
        QVERIFY(directory.isValid());

        markshot::recording::RecordingOptions options;
        options.mode = markshot::recording::RecordingMode::Video;
        options.includeAudio = false;
        options.outputPath = directory.filePath(QStringLiteral("sample.mp4"));

        markshot::recording::RecordingVideoEncoderOptions encoder;
        encoder.id = QStringLiteral("mpeg4");
        encoder.label = QStringLiteral("mpeg4");
        encoder.hardware = false;

        QString error;
        markshot::recording::LibavRecordingProcess process;
        QVERIFY2(process.start(options, encoder, QSize(32, 24), 10, &error),
                 qPrintable(error));

        for (int i = 0; i < 4; ++i) {
            QImage image(32, 24, QImage::Format_ARGB32);
            image.fill(QColor(40 + i * 20, 80, 120).rgba());

            markshot::recording::RecordingFrameSample sample;
            sample.image = image;
            sample.timestampMs = i * 100;
            sample.sequence = i + 1;
            QVERIFY2(process.writeFrame(sample, &error), qPrintable(error));
        }

        QVERIFY2(process.finish(&error), qPrintable(error));
        const QFileInfo output(options.outputPath);
        QVERIFY(output.exists());
        QVERIFY(output.size() > 0);
    }

    /**
     * 验证补帧接口可复用上一帧数据生成视频。
     * @return 无返回值。
     */
    void writesRepeatFrames()
    {
        QTemporaryDir directory;
        QVERIFY(directory.isValid());

        markshot::recording::RecordingOptions options;
        options.mode = markshot::recording::RecordingMode::Video;
        options.includeAudio = false;
        options.outputPath = directory.filePath(QStringLiteral("repeat.mp4"));

        markshot::recording::RecordingVideoEncoderOptions encoder;
        encoder.id = QStringLiteral("mpeg4");
        encoder.label = QStringLiteral("mpeg4");
        encoder.hardware = false;

        QString error;
        markshot::recording::LibavRecordingProcess process;
        QVERIFY2(process.start(options, encoder, QSize(32, 24), 10, &error),
                 qPrintable(error));

        // 无前置帧时补帧必须失败
        QVERIFY(!process.writeRepeatFrame(&error));

        QImage image(32, 24, QImage::Format_ARGB32);
        image.fill(QColor(60, 90, 130).rgba());
        markshot::recording::RecordingFrameSample sample;
        sample.image = image;
        sample.timestampMs = 0;
        sample.sequence = 1;
        QVERIFY2(process.writeFrame(sample, &error), qPrintable(error));
        QVERIFY2(process.writeRepeatFrame(&error), qPrintable(error));
        QVERIFY2(process.writeRepeatFrame(&error), qPrintable(error));

        QVERIFY2(process.finish(&error), qPrintable(error));
        const QFileInfo output(options.outputPath);
        QVERIFY(output.exists());
        QVERIFY(output.size() > 0);
    }

    /**
     * 验证 VAAPI 硬件编码在有渲染节点的机器上能完整写出视频。
     * @return 无返回值。
     */
    void writesVaapiEncodedVideoWhenAvailable()
    {
        if (!markshot::recording::recordingRenderNodeAvailable()
            || !markshot::recording::recordingEncoderImplementationAvailable(
                QStringLiteral("h264_vaapi"))) {
            QSKIP("VAAPI render node or encoder is unavailable");
        }

        QTemporaryDir directory;
        QVERIFY(directory.isValid());

        markshot::recording::RecordingOptions options;
        options.mode = markshot::recording::RecordingMode::Video;
        options.includeAudio = false;
        options.outputPath = directory.filePath(QStringLiteral("vaapi.mp4"));

        markshot::recording::RecordingVideoEncoderOptions encoder;
        encoder.id = QStringLiteral("h264_vaapi");
        encoder.label = QStringLiteral("h264_vaapi");
        encoder.hardware = true;

        QString error;
        markshot::recording::LibavRecordingProcess process;
        if (!process.start(options, encoder, QSize(320, 240), 15, &error)) {
            // 容器内或受限环境下打不开 VAAPI 设备时跳过，运行时会回退软件编码
            QSKIP(qPrintable(QStringLiteral("VAAPI encoder cannot be opened: %1").arg(error)));
        }

        for (int i = 0; i < 6; ++i) {
            QImage image(320, 240, QImage::Format_ARGB32);
            image.fill(QColor(30 + i * 15, 90, 160).rgba());

            markshot::recording::RecordingFrameSample sample;
            sample.image = image;
            sample.timestampMs = i * 66;
            sample.sequence = i + 1;
            QVERIFY2(process.writeFrame(sample, &error), qPrintable(error));
        }
        QVERIFY2(process.writeRepeatFrame(&error), qPrintable(error));
        QVERIFY2(process.finish(&error), qPrintable(error));

        const QFileInfo output(options.outputPath);
        QVERIFY(output.exists());
        QVERIFY(output.size() > 0);
    }

    /**
     * 验证库内音频编码器可以写入 AAC 音频流。
     * @return 无返回值。
     */
    void writesSmallAudioStream()
    {
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        const QByteArray outputPath = QFile::encodeName(
            directory.filePath(QStringLiteral("audio.m4a")));

        AVFormatContext *formatContext = nullptr;
        int result = avformat_alloc_output_context2(&formatContext,
                                                    nullptr,
                                                    nullptr,
                                                    outputPath.constData());
        QVERIFY2(result >= 0 && formatContext, qPrintable(markshot::recording::libavErrorText(result)));
        result = avio_open(&formatContext->pb, outputPath.constData(), AVIO_FLAG_WRITE);
        QVERIFY2(result >= 0, qPrintable(markshot::recording::libavErrorText(result)));

        QString error;
        std::mutex writeMutex;
        markshot::recording::LibavAudioEncoder encoder;
        QVERIFY2(encoder.open(formatContext, 48000, &error), qPrintable(error));
        result = avformat_write_header(formatContext, nullptr);
        QVERIFY2(result >= 0, qPrintable(markshot::recording::libavErrorText(result)));

        for (int i = 0; i < 4; ++i) {
            markshot::recording::AudioCaptureSample sample;
            sample.pcm = QByteArray(encoder.frameBytes(), '\0');
            sample.sequence = i + 1;
            QVERIFY2(encoder.encode(sample, writeMutex, &error), qPrintable(error));
        }
        QVERIFY2(encoder.flush(writeMutex, &error), qPrintable(error));
        result = av_write_trailer(formatContext);
        QVERIFY2(result >= 0, qPrintable(markshot::recording::libavErrorText(result)));

        encoder.close();
        avio_closep(&formatContext->pb);
        avformat_free_context(formatContext);

        const QFileInfo output(QString::fromLocal8Bit(outputPath));
        QVERIFY(output.exists());
        QVERIFY(output.size() > 0);
    }

    /**
     * 验证库内 FFmpeg writer 可以生成基础 GIF 文件。
     * @return 无返回值。
     */
    void writesSmallGif()
    {
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        const QString outputPath = directory.filePath(QStringLiteral("sample.gif"));

        QString error;
        markshot::recording::LibavGifRecordingProcess process;
        QVERIFY2(process.start(outputPath, QSize(32, 24), 8, &error), qPrintable(error));
        for (int i = 0; i < 4; ++i) {
            QImage image(32, 24, QImage::Format_ARGB32);
            image.fill(QColor(120, 40 + i * 20, 80).rgba());

            markshot::recording::RecordingFrameSample sample;
            sample.image = image;
            sample.timestampMs = i * 125;
            sample.sequence = i + 1;
            QVERIFY2(process.writeFrame(sample, &error), qPrintable(error));
        }
        QVERIFY2(process.finish(&error), qPrintable(error));

        const QFileInfo output(outputPath);
        QVERIFY(output.exists());
        QVERIFY(output.size() > 0);
    }

    /**
     * 验证 GIF 量化后的色彩保真度，逐帧调色板应显著优于固定调色板。
     * @return 无返回值。
     */
    void writesGifWithAccuratePalette()
    {
#ifndef HAVE_LIBAVFILTER
        QSKIP("libavfilter is not available, GIF falls back to a fixed palette");
#else
        if (!QImageReader::supportedImageFormats().contains(QByteArrayLiteral("gif"))) {
            QSKIP("Qt GIF image plugin is unavailable");
        }

        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        const QString outputPath = directory.filePath(QStringLiteral("gradient.gif"));

        // 1. 生成平滑渐变，固定调色板在这类画面上误差最明显
        QImage source(64, 64, QImage::Format_ARGB32);
        for (int y = 0; y < source.height(); ++y) {
            for (int x = 0; x < source.width(); ++x) {
                source.setPixel(x, y, qRgb(x * 4, y * 4, 128));
            }
        }

        QString error;
        markshot::recording::LibavGifRecordingProcess process;
        QVERIFY2(process.start(outputPath, source.size(), 10, &error), qPrintable(error));
        markshot::recording::RecordingFrameSample sample;
        sample.image = source;
        sample.timestampMs = 0;
        sample.sequence = 1;
        QVERIFY2(process.writeFrame(sample, &error), qPrintable(error));
        QVERIFY2(process.finish(&error), qPrintable(error));

        // 2. 读回首帧并统计与原图的平均通道误差
        QImage decoded(outputPath);
        QVERIFY(!decoded.isNull());
        QCOMPARE(decoded.size(), source.size());
        decoded = decoded.convertToFormat(QImage::Format_ARGB32);

        qint64 totalError = 0;
        for (int y = 0; y < source.height(); ++y) {
            for (int x = 0; x < source.width(); ++x) {
                const QRgb expected = source.pixel(x, y);
                const QRgb actual = decoded.pixel(x, y);
                totalError += std::abs(qRed(expected) - qRed(actual));
                totalError += std::abs(qGreen(expected) - qGreen(actual));
                totalError += std::abs(qBlue(expected) - qBlue(actual));
            }
        }
        const double averageError =
            static_cast<double>(totalError) / (source.width() * source.height() * 3);
        qInfo("GIF average channel error: %.2f", averageError);
        // 3-3-2 固定调色板在该渐变上的平均误差在 20 以上，逐帧调色板应远低于此
        QVERIFY2(averageError < 10.0,
                 qPrintable(QStringLiteral("GIF palette error is too high: %1").arg(averageError)));
#endif
    }
};

QTEST_APPLESS_MAIN(LibavRecordingProcessTest)
#include "libav_recording_process_test.moc"
