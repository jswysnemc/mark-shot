#include "recording/libav_recording_process.h"
#include "recording/libav_audio_encoder.h"
#include "recording/libav_error.h"
#include "recording/libav_gif_recording_process.h"

#include <QtTest/QtTest>

#include <QFileInfo>
#include <QColor>
#include <QFile>
#include <QImage>
#include <QTemporaryDir>

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
        encoder.id = QStringLiteral("libx264");
        encoder.label = QStringLiteral("libx264");
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
};

QTEST_APPLESS_MAIN(LibavRecordingProcessTest)
#include "libav_recording_process_test.moc"
