#pragma once

#include <QByteArray>

#include <array>

namespace markshot::recording {

/**
 * 录制帧 BGRA 缓冲池。
 *
 * 采集回调每帧需要一块与帧等大的连续缓冲，直接新建 QByteArray 会造成
 * 高频大块分配。池内保留固定数量的槽位，下游释放引用后槽位自动可复用。
 * 仅限单个采集线程使用，不做加锁。
 */
class RecordingBgraBufferPool final {
public:
    /**
     * 获取一块指定字节数的可写缓冲。
     * @param byteCount 需要的字节数。
     * @return 池内槽位引用，写入完成后按值拷出即共享数据；引用在下次
     *         acquire 前有效。
     */
    QByteArray &acquire(qsizetype byteCount);

private:
    static constexpr int kSlotCount = 4;

    std::array<QByteArray, kSlotCount> m_slots;
    int m_cursor = 0;
};

}  // namespace markshot::recording
