#include "recording/recording_capture_memory_policy.h"
#include "recording/recording_frame_payload.h"

#include <QtTest/QtTest>

using namespace markshot::recording;

class RecordingCaptureMemoryPolicyTest final : public QObject {
    Q_OBJECT
private slots:
    /** 校验常用尺寸、对齐跨度以及倒序行，无参数和返回值。 */
    void calculatesValidatedPixelBytes()
    {
        QCOMPARE(recordingBgraFrameBytes(QSize(3840, 2160)), qsizetype(33177600));
        QCOMPARE(recordingBgraFrameBytes(QSize(10, 20), 64), qsizetype(1280));
        QCOMPARE(recordingBgraFrameBytes(QSize(10, 20), -64), qsizetype(1280));
        QCOMPARE(recordingBgraFrameBytes({}, 4), qsizetype(0));
        QCOMPARE(recordingBgraFrameBytes(QSize(10, 20), 39), qsizetype(0));
        QCOMPARE(recordingBgraFrameBytes(QSize(10, 20), std::numeric_limits<int>::min()), qsizetype(0));
        QCOMPARE(recordingBgraFrameBytes(QSize(std::numeric_limits<int>::max(), 1)), qsizetype(0));
        QCOMPARE(recordingBgraFrameBytes(QSize(4096, std::numeric_limits<int>::max())), qsizetype(0));
        RecordingRawBgraFrame invalid;
        invalid.size = QSize(10, 20);
        invalid.stride = std::numeric_limits<int>::min();
        QVERIFY(!invalid.isValid());
    }

    /** 默认预算允许三帧 4K；8K 和降分辨率过渡保留必要的两帧空间，无参数和返回值。 */
    void keepsProgressWithoutUnboundedExpansion()
    {
        constexpr quint64 budget = 96 * 1024 * 1024;
        const auto frame = static_cast<quint64>(recordingBgraFrameBytes(QSize(3840, 2160)));
        QCOMPARE(recordingEffectiveCaptureBudget(frame, 0, budget), budget);
        QCOMPARE(recordingEffectiveCaptureBudget(frame * 4, 0, budget), frame * 8);
        QCOMPARE(recordingEffectiveCaptureBudget(1024, frame * 4, budget), frame * 8);
        QCOMPARE(recordingEffectiveCaptureBudget(1024, 0, budget), budget);
        QCOMPARE(recordingEffectiveCaptureBudget(std::numeric_limits<quint64>::max(), 0, budget),
                 std::numeric_limits<quint64>::max());
    }
};

QTEST_APPLESS_MAIN(RecordingCaptureMemoryPolicyTest)
#include "recording_capture_memory_policy_test.moc"
