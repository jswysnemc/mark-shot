#include "recording/recording_bgra_buffer_pool.h"

#include <QtTest/QtTest>

#include <array>
#include <limits>

using namespace markshot::recording;

class RecordingBgraBufferPoolTest : public QObject {
    Q_OBJECT

private slots:
    /** 借出全部槽位后拒绝继续扩容，归还一个槽位后原地复用，无参数和返回值。 */
    void refusesAllocationWhileEverySlotIsHeld()
    {
        RecordingBgraBufferPool pool;
        std::array<QByteArray, 4> held;
        for (int index = 0; index < 4; ++index) {
            auto *slot = pool.tryAcquire(4096, 0);
            QVERIFY(slot);
            slot->fill(char(index + 1));
            held[index] = *slot;
        }
        const auto capacity = pool.retainedCapacity();
        QVERIFY(!pool.tryAcquire(4096, 10000));
        QCOMPARE(pool.retainedCapacity(), capacity);
        for (int index = 0; index < 4; ++index) {
            QCOMPARE(held[index], QByteArray(4096, char(index + 1)));
        }
        const char *released = held[2].constData();
        held[2] = {};
        auto *slot = pool.tryAcquire(4096, 10001);
        QVERIFY(slot);
        QVERIFY(slot->constData() == released);
    }

    /** 按实际字节而非固定四槽计数准入，预算内仍允许两帧前进，无参数和返回值。 */
    void enforcesByteBudget()
    {
        RecordingBgraBufferPool pool(3 * 4096);
        std::array<QByteArray, 3> held;
        for (auto &frame : held) {
            auto *slot = pool.tryAcquire(4096, 0);
            QVERIFY(slot);
            frame = *slot;
        }
        QCOMPARE(pool.retainedCapacity(), qsizetype(3 * 4096));
        QVERIFY(!pool.tryAcquire(4096, 0));
        RecordingBgraBufferPool smallBudget(1);
        const QByteArray first = *smallBudget.tryAcquire(4096, 0);
        const QByteArray second = *smallBudget.tryAcquire(4096, 0);
        QVERIFY(!smallBudget.tryAcquire(4096, 0));
        QCOMPARE(smallBudget.retainedCapacity(), qsizetype(8192));
    }

    /** 不依赖后续帧到达也能回收空闲槽位，但不会释放仍被借出的内存，无参数和返回值。 */
    void reclaimsOnlyContinuouslyIdleBuffers()
    {
        RecordingBgraBufferPool pool;
        std::array<QByteArray, 4> held;
        for (auto &frame : held) frame = *pool.tryAcquire(4096, 0);
        held[0] = {};
        held[1] = {};
        pool.reclaimIdle(100);
        pool.reclaimIdle(100 + kRecordingCaptureIdleMs - 1);
        QCOMPARE(pool.retainedCapacity(), qsizetype(4 * 4096));
        pool.reclaimIdle(100 + kRecordingCaptureIdleMs);
        QCOMPARE(pool.retainedCapacity(), qsizetype(2 * 4096));
        held = {};
        pool.reclaimIdle(100000);
        QCOMPARE(pool.retainedCapacity(), qsizetype(2 * 4096));
        pool.reclaimIdle(100000 + kRecordingCaptureIdleMs);
        QCOMPARE(pool.retainedCapacity(), qsizetype(0));
    }

    /** 大帧尚未全部归还时先等待，归还一帧后允许缩小分辨率且不覆盖旧快照，无参数和返回值。 */
    void resolutionChangesDoNotOverwriteOrDeadlock()
    {
        RecordingBgraBufferPool pool(4096);
        QByteArray largeA = *pool.tryAcquire(8192, 0);
        QByteArray largeB = *pool.tryAcquire(8192, 0);
        QVERIFY(!pool.tryAcquire(1024, 1));
        largeA = {};
        auto *small = pool.tryAcquire(1024, 2);
        QVERIFY(small);
        QCOMPARE(small->size(), qsizetype(1024));
        QCOMPARE(pool.retainedCapacity(), qsizetype(8192 + 1024));
        const QByteArray smallHeld = *small;
        largeB = {};
        auto *next = pool.tryAcquire(1024, 3);
        QVERIFY(next);
        QCOMPARE(pool.retainedCapacity(), qsizetype(2048));
    }

    /** 拒绝非法尺寸，停止后释放池引用而不使外部 QByteArray 失效，无参数和返回值。 */
    void validatesSizeAndResetPreservesSnapshots()
    {
        RecordingBgraBufferPool pool;
        QVERIFY(!pool.tryAcquire(0));
        QVERIFY(!pool.tryAcquire(-1));
        QVERIFY(!pool.tryAcquire(std::numeric_limits<qsizetype>::max()));
        auto *slot = pool.tryAcquire(64);
        QVERIFY(slot);
        slot->fill('x');
        const QByteArray held = *slot;
        pool.reset();
        QCOMPARE(pool.retainedCapacity(), qsizetype(0));
        QCOMPARE(held, QByteArray(64, 'x'));
    }
};

QTEST_APPLESS_MAIN(RecordingBgraBufferPoolTest)
#include "recording_bgra_buffer_pool_test.moc"
