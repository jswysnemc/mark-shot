#include "recording/recording_bgra_buffer_pool.h"

#include <QtTest/QtTest>

#include <cstring>

class RecordingBgraBufferPoolTest : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证缓冲池返回指定大小的可写缓冲。
     * @return 无返回值。
     */
    void acquiresRequestedSize()
    {
        markshot::recording::RecordingBgraBufferPool pool;
        QByteArray &buffer = pool.acquire(64);
        QCOMPARE(buffer.size(), qsizetype(64));
        std::memset(buffer.data(), 0x5a, 64);
        QCOMPARE(buffer.at(63), char(0x5a));
    }

    /**
     * 验证下游释放引用后槽位内存被复用。
     * @return 无返回值。
     */
    void reusesReleasedSlot()
    {
        markshot::recording::RecordingBgraBufferPool pool;
        const char *firstData = nullptr;
        {
            QByteArray &buffer = pool.acquire(128);
            firstData = buffer.constData();
        }
        // 未被下游持有的槽位应在后续 acquire 中复用同一块内存
        bool reused = false;
        for (int i = 0; i < 4; ++i) {
            QByteArray &buffer = pool.acquire(128);
            if (buffer.constData() == firstData) {
                reused = true;
                break;
            }
        }
        QVERIFY(reused);
    }

    /**
     * 验证被下游持有的缓冲不会被后续 acquire 覆写。
     * @return 无返回值。
     */
    void doesNotOverwriteHeldBuffers()
    {
        markshot::recording::RecordingBgraBufferPool pool;
        QByteArray &slot = pool.acquire(16);
        std::memset(slot.data(), 0x11, 16);
        const QByteArray held = slot;

        // 连续获取超过槽位数量的缓冲并写入不同内容
        for (int i = 0; i < 8; ++i) {
            QByteArray &buffer = pool.acquire(16);
            std::memset(buffer.data(), 0x22 + i, 16);
        }
        QCOMPARE(held.at(0), char(0x11));
        QCOMPARE(held.at(15), char(0x11));
    }
};

QTEST_APPLESS_MAIN(RecordingBgraBufferPoolTest)
#include "recording_bgra_buffer_pool_test.moc"
