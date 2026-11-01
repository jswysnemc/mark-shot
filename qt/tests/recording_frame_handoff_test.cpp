#include "recording/recording_frame_handoff.h"

#include <QtTest/QtTest>

#include <atomic>
#include <array>
#include <chrono>
#include <thread>

using namespace markshot::recording;

namespace {
/** 为指定 sequence 创建共享 BGRA 测试帧，返回带毫秒时间戳的样本。 */
RecordingFrameSample makeFrame(qint64 sequence)
{
    RecordingFrameSample sample;
    sample.bgra.size = QSize(32, 24);
    sample.bgra.stride = 32 * 4;
    sample.bgra.bytes = QByteArray(32 * 24 * 4, static_cast<char>(sequence));
    sample.sequence = sequence;
    sample.timestampMs = sequence * 10;
    return sample;
}

struct FrameLifetimes {
    std::atomic<int> live{0};
    std::atomic<int> peak{0};
};

struct TrackedFrame {
    std::array<char, 32 * 24 * 4> pixels{};
    std::shared_ptr<FrameLifetimes> counts;

    /** 用 sharedCounts 统计当前载荷数量和峰值，像素由该对象单独持有。 */
    explicit TrackedFrame(std::shared_ptr<FrameLifetimes> sharedCounts)
        : counts(std::move(sharedCounts))
    {
        const int live = ++counts->live;
        int peak = counts->peak;
        while (peak < live && !counts->peak.compare_exchange_weak(peak, live)) {}
    }
    /** 释放载荷时减少在途计数。 */
    ~TrackedFrame() { --counts->live; }
};

/** 按 sequence 创建映射帧，以 counts 统计共享所有权，不通过复制像素维持生命周期。 */
RecordingFrameSample trackedFrame(qint64 sequence, const std::shared_ptr<FrameLifetimes> &counts)
{
    const auto owner = std::make_shared<TrackedFrame>(counts);
    auto sample = makeFrame(sequence);
    sample.bgra.bytes.clear();
    sample.bgra.mappedOwner = owner;
    sample.bgra.mappedData = owner->pixels.data();
    sample.bgra.mappedSize = owner->pixels.size();
    return sample;
}

class WakeCounter final : public QObject {
public:
    int calls = 0;
protected:
    /** 观察 object 上的 event，统计元调用但不拦截事件，返回 false。 */
    bool eventFilter(QObject *object, QEvent *event) override
    {
        Q_UNUSED(object)
        calls += event->type() == QEvent::MetaCall;
        return false;
    }
};
}  // namespace

class RecordingFrameHandoffTest final : public QObject {
    Q_OBJECT

private slots:
    /** 主线程不处理事件时只交付最新帧，不重放历史积压，无参数和返回值。 */
    void blockedReceiverKeepsOnlyLatestFrame()
    {
        RecordingFrameHandoff handoff;
        WakeCounter wakes;
        handoff.installEventFilter(&wakes);
        int delivered = 0;
        qint64 lastSequence = 0;
        connect(&handoff, &RecordingFrameHandoff::frameReady, this,
                [&](const RecordingFrameSample &sample) {
                    ++delivered;
                    lastSequence = sample.sequence;
                    QCOMPARE(QThread::currentThread(), thread());
                });
        std::thread producer([publish = handoff.publisher()] {
            for (int i = 1; i <= 24; ++i) {
                publish(makeFrame(i));
            }
        });
        producer.join();
        QCOMPARE(delivered, 0);
        QCoreApplication::sendPostedEvents(&handoff, QEvent::MetaCall);
        QCOMPARE(delivered, 1);
        QCOMPARE(lastSequence, 24);
        QCOMPARE(wakes.calls, 1);
    }

    /** 覆盖主线程常见的短暂停顿和一秒阻塞，无参数和返回值。 */
    void blockedReceiverBoundsPayloadLifetime_data()
    {
        QTest::addColumn<int>("blockedMs");
        for (int duration : {50, 200, 1000}) {
            QTest::newRow(qPrintable(QString::number(duration))) << duration;
        }
    }

    /** 阻塞期间只保留一个待交付载荷，关闭立即释放它，无参数和返回值。 */
    void blockedReceiverBoundsPayloadLifetime()
    {
        QFETCH(int, blockedMs);
        RecordingFrameHandoff handoff;
        const auto counts = std::make_shared<FrameLifetimes>();
        std::atomic<bool> stop{false};
        std::thread producer([&, publish = handoff.publisher()] {
            for (int index = 1; !stop; ++index) {
                publish(trackedFrame(index, counts));
                std::this_thread::sleep_for(std::chrono::milliseconds(1));
            }
        });
        std::this_thread::sleep_for(std::chrono::milliseconds(blockedMs));
        stop = true;
        producer.join();
        QCOMPARE(counts->live.load(), 1);
        QVERIFY(counts->peak <= 2);
        handoff.close();
        QCOMPARE(counts->live.load(), 0);
        QCoreApplication::sendPostedEvents(&handoff, QEvent::MetaCall);
        QCOMPARE(counts->live.load(), 0);
    }

    /** 接收回调中继续发布只产生下一次唤醒，交付不丢失、不重入且不复制映射，无参数和返回值。 */
    void reentrantPublishingSchedulesAnotherWake()
    {
        RecordingFrameHandoff handoff;
        const auto counts = std::make_shared<FrameLifetimes>();
        const auto publish = handoff.publisher();
        auto first = trackedFrame(1, counts);
        const char *pixels = first.bgra.constData();
        QList<qint64> sequences;
        connect(&handoff, &RecordingFrameHandoff::frameReady, this,
                [&](const RecordingFrameSample &sample) {
                    sequences.append(sample.sequence);
                    if (sample.sequence == 1) {
                        QVERIFY(sample.bgra.constData() == pixels);
                        publish(trackedFrame(2, counts));
                        publish(trackedFrame(3, counts));
                        QCOMPARE(sequences.size(), 1);
                    }
                });
        publish(std::move(first));
        QTRY_COMPARE(sequences.size(), 2);
        QCOMPARE(sequences, (QList<qint64>{1, 3}));
        QCOMPARE(counts->live.load(), 0);
        QVERIFY(counts->peak <= 3);
    }

    /** 背压清空待交付载荷，恢复后不重放过期帧，关闭不可恢复，无参数和返回值。 */
    void disableAndCloseDiscardPendingFrames()
    {
        RecordingFrameHandoff handoff;
        const auto counts = std::make_shared<FrameLifetimes>();
        const auto publish = handoff.publisher();
        QList<qint64> delivered;
        connect(&handoff, &RecordingFrameHandoff::frameReady, this,
                [&](const RecordingFrameSample &sample) { delivered.append(sample.sequence); });
        publish(trackedFrame(1, counts));
        handoff.setEnabled(false);
        QCOMPARE(counts->live.load(), 0);
        publish(trackedFrame(2, counts));
        QCOMPARE(counts->live.load(), 0);
        QCoreApplication::sendPostedEvents(&handoff, QEvent::MetaCall);
        QVERIFY(delivered.isEmpty());
        handoff.setEnabled(true);
        publish(trackedFrame(3, counts));
        QCoreApplication::sendPostedEvents(&handoff, QEvent::MetaCall);
        QCOMPARE(delivered, QList<qint64>{3});
        publish(trackedFrame(4, counts));
        handoff.close();
        handoff.setEnabled(true);
        publish(trackedFrame(5, counts));
        QCoreApplication::sendPostedEvents(&handoff, QEvent::MetaCall);
        QCOMPARE(delivered, QList<qint64>{3});
        QCOMPARE(counts->live.load(), 0);
    }

    /** 首个错误替代未交付帧，仅通知一次且不能通过解除背压复活，无参数和返回值。 */
    void failureIsTerminalAndCoalesced()
    {
        RecordingFrameHandoff handoff;
        WakeCounter wakes;
        handoff.installEventFilter(&wakes);
        const auto counts = std::make_shared<FrameLifetimes>();
        int delivered = 0;
        connect(&handoff, &RecordingFrameHandoff::frameReady, this,
                [&](const RecordingFrameSample &) { ++delivered; });
        QSignalSpy errors(&handoff, &RecordingFrameHandoff::failed);
        const auto publish = handoff.publisher();
        const auto fail = handoff.errorPublisher();
        fail({});
        publish(trackedFrame(1, counts));
        fail(QStringLiteral("first error"));
        QCOMPARE(counts->live.load(), 0);
        fail(QStringLiteral("second error"));
        handoff.setEnabled(true);
        publish(trackedFrame(2, counts));
        QCoreApplication::sendPostedEvents(&handoff, QEvent::MetaCall);
        QCOMPARE(delivered, 0);
        QCOMPARE(errors.size(), 1);
        QCOMPARE(errors.first().first().toString(), QStringLiteral("first error"));
        QCOMPARE(wakes.calls, 1);
        QCOMPARE(counts->live.load(), 0);
    }

    /** 并发到达的旧帧和无效帧不能覆盖较新的采集时间，无参数和返回值。 */
    void staleSamplesCannotReplaceNewerFrames()
    {
        RecordingFrameHandoff handoff;
        QList<qint64> sequences;
        connect(&handoff, &RecordingFrameHandoff::frameReady, this,
                [&](const RecordingFrameSample &sample) { sequences.append(sample.sequence); });
        const auto publish = handoff.publisher();
        publish(makeFrame(20));
        publish(makeFrame(5));
        publish({});
        QCoreApplication::sendPostedEvents(&handoff, QEvent::MetaCall);
        publish(makeFrame(19));
        auto newer = makeFrame(21);
        newer.timestampMs = 200;
        publish(newer);
        publish(makeFrame(20));
        QCoreApplication::sendPostedEvents(&handoff, QEvent::MetaCall);
        QCOMPARE(sequences, (QList<qint64>{20, 21}));
    }

    /** 多个发布线程竞争时按采集时间选择最新帧，最多排入一次唤醒，无参数和返回值。 */
    void concurrentPublishersKeepLatestCaptureTimestamp()
    {
        RecordingFrameHandoff handoff;
        qint64 delivered = 0;
        int calls = 0;
        connect(&handoff, &RecordingFrameHandoff::frameReady, this,
                [&](const RecordingFrameSample &sample) { delivered = sample.sequence; ++calls; });
        std::atomic<qint64> sequence{0};
        std::array<std::thread, 4> producers;
        for (auto &producer : producers) {
            producer = std::thread([&, publish = handoff.publisher()] {
                for (int index = 0; index < 1000; ++index) {
                    publish(makeFrame(++sequence));
                }
            });
        }
        for (auto &producer : producers) {
            producer.join();
        }
        QCoreApplication::sendPostedEvents(&handoff, QEvent::MetaCall);
        QCOMPARE(delivered, 4000);
        QCOMPARE(calls, 1);
    }

    /** 图像载荷也按共享所有权交接，不隐式转换成另一份像素，无参数和返回值。 */
    void imagePayloadIsSharedWithoutConversion()
    {
        RecordingFrameHandoff handoff;
        RecordingFrameSample sample;
        sample.image = QImage(32, 24, QImage::Format_ARGB32);
        sample.image.fill(Qt::red);
        const uchar *pixels = sample.image.constBits();
        bool delivered = false;
        connect(&handoff, &RecordingFrameHandoff::frameReady, this,
                [&](const RecordingFrameSample &received) {
                    QVERIFY(received.image.constBits() == pixels);
                    delivered = true;
                });
        handoff.publisher()(std::move(sample));
        QCoreApplication::sendPostedEvents(&handoff, QEvent::MetaCall);
        QVERIFY(delivered);
    }

    /** 主线程销毁会话时，后台发布句柄不会再访问接收对象，无参数和返回值。 */
    void publisherCanOutliveReceiverDuringConcurrentDestruction()
    {
        auto handoff = std::make_unique<RecordingFrameHandoff>();
        const auto publish = handoff->publisher();
        const auto fail = handoff->errorPublisher();
        const auto counts = std::make_shared<FrameLifetimes>();
        std::atomic<bool> started{false};
        std::thread producer([&] {
            for (int index = 1; index <= 10000; ++index) {
                publish(trackedFrame(index, counts));
                started = true;
            }
            fail(QStringLiteral("late failure"));
        });
        while (!started) { std::this_thread::yield(); }
        handoff.reset();
        producer.join();
        publish(trackedFrame(10001, counts));
        fail(QStringLiteral("after destruction"));
        QCOMPARE(counts->live.load(), 0);
        QVERIFY(counts->peak <= 2);
    }

    /** 接收方可以在回调内关闭并销毁交接器，剩余发布句柄仍安全，无参数和返回值。 */
    void receiverMayDestroyHandoffInsideCallback()
    {
        auto handoff = std::make_unique<RecordingFrameHandoff>();
        const auto publish = handoff->publisher();
        int delivered = 0;
        connect(handoff.get(), &RecordingFrameHandoff::frameReady, this,
                [&](const RecordingFrameSample &) {
                    ++delivered;
                    handoff.reset();
                });
        publish(makeFrame(1));
        QCoreApplication::sendPostedEvents(handoff.get(), QEvent::MetaCall);
        QCOMPARE(delivered, 1);
        publish(makeFrame(2));
    }
};

QTEST_GUILESS_MAIN(RecordingFrameHandoffTest)
#include "recording_frame_handoff_test.moc"
