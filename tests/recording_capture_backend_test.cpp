#include "recording/recording_capture_backend.h"

#include <QtTest/QtTest>

class RecordingCaptureBackendTest : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证自动模式优先使用无 Portal 录制路径。
     * @return 无返回值。
     */
    void autoBackendKeepsPreferredFallbackOrder()
    {
        const QVector<markshot::recording::RecordingCaptureBackend> order =
            markshot::recording::recordingCaptureBackendOrder(
                markshot::recording::RecordingCaptureBackend::Auto);

        QCOMPARE(order,
                 QVector<markshot::recording::RecordingCaptureBackend>({
                     markshot::recording::RecordingCaptureBackend::Wlroots,
                     markshot::recording::RecordingCaptureBackend::PipeWire,
                     markshot::recording::RecordingCaptureBackend::WindowsWgc,
                     markshot::recording::RecordingCaptureBackend::Polling,
                 }));
    }

    /**
     * 验证 PipeWire 失败时可以回退到 CPU 可读采集路径。
     * @return 无返回值。
     */
    void pipeWireBackendFallsBackAfterUnmappableFrames()
    {
        const QVector<markshot::recording::RecordingCaptureBackend> order =
            markshot::recording::recordingCaptureBackendOrder(
                markshot::recording::RecordingCaptureBackend::PipeWire);

        QCOMPARE(order,
                 QVector<markshot::recording::RecordingCaptureBackend>({
                     markshot::recording::RecordingCaptureBackend::PipeWire,
                     markshot::recording::RecordingCaptureBackend::Wlroots,
                     markshot::recording::RecordingCaptureBackend::WindowsWgc,
                     markshot::recording::RecordingCaptureBackend::Polling,
                 }));
    }

    /**
     * 验证 wlroots 后端不可用时仍可回退到轮询采集。
     * @return 无返回值。
     */
    void wlrootsBackendFallsBackToPolling()
    {
        const QVector<markshot::recording::RecordingCaptureBackend> order =
            markshot::recording::recordingCaptureBackendOrder(
                markshot::recording::RecordingCaptureBackend::Wlroots);

        QCOMPARE(order,
                 QVector<markshot::recording::RecordingCaptureBackend>({
                     markshot::recording::RecordingCaptureBackend::Wlroots,
                     markshot::recording::RecordingCaptureBackend::WindowsWgc,
                     markshot::recording::RecordingCaptureBackend::Polling,
                 }));
    }

    /**
     * 验证 Windows WGC 后端不可用时仍可回退到轮询采集。
     * @return 无返回值。
     */
    void windowsWgcBackendFallsBackToPolling()
    {
        const QVector<markshot::recording::RecordingCaptureBackend> order =
            markshot::recording::recordingCaptureBackendOrder(
                markshot::recording::RecordingCaptureBackend::WindowsWgc);

        QCOMPARE(order,
                 QVector<markshot::recording::RecordingCaptureBackend>({
                     markshot::recording::RecordingCaptureBackend::WindowsWgc,
                     markshot::recording::RecordingCaptureBackend::Polling,
                 }));
    }

    /**
     * 验证轮询后端保持严格单后端路径。
     * @return 无返回值。
     */
    void pollingBackendRemainsStrict()
    {
        const QVector<markshot::recording::RecordingCaptureBackend> order =
            markshot::recording::recordingCaptureBackendOrder(
                markshot::recording::RecordingCaptureBackend::Polling);

        QCOMPARE(order,
                 QVector<markshot::recording::RecordingCaptureBackend>({
                     markshot::recording::RecordingCaptureBackend::Polling,
                 }));
    }
};

QTEST_APPLESS_MAIN(RecordingCaptureBackendTest)
#include "recording_capture_backend_test.moc"
