#include "recording/recording_video_encoder_options.h"

#include <QtTest/QtTest>

class RecordingVideoEncoderOptionsTest final : public QObject {
    Q_OBJECT

private slots:
    /**
     * 【录制】【软件编码回退】验证候选列表包含 FFmpeg 原生 mpeg4 编码器。
     * @return 无返回值。
     */
    void includesNativeMpeg4Fallback()
    {
        markshot::recording::RecordingOptions options;
        const QVector<markshot::recording::RecordingVideoEncoderOptions> candidates =
            markshot::recording::recordingVideoEncoderCandidates(options, 30);

        QStringList candidateIds;
        for (const markshot::recording::RecordingVideoEncoderOptions &candidate : candidates) {
            candidateIds.append(candidate.id);
        }

        const int x264Index = candidateIds.indexOf(QStringLiteral("libx264"));
        const int mpeg4Index = candidateIds.indexOf(QStringLiteral("mpeg4"));
        QVERIFY(x264Index >= 0);
        QVERIFY(mpeg4Index > x264Index);
        QVERIFY(!candidates.at(mpeg4Index).hardware);
    }
};

QTEST_APPLESS_MAIN(RecordingVideoEncoderOptionsTest)

#include "recording_video_encoder_options_test.moc"
