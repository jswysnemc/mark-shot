#pragma once

#include "recording/recording_capture_memory_policy.h"

#include <QByteArray>

#include <array>

namespace markshot::recording {

/**
 * 录制帧 BGRA 缓冲池。
 *
 * 采集回调每帧需要一块与帧等大的连续缓冲，直接新建 QByteArray 会造成
 * 高频大块分配。池内最多保留固定数量的槽位，下游释放引用后槽位自动可复用。
 * 优先复用最近使用的槽位，持续闲置的槽位延迟释放，避免一直保留积压时的峰值容量。
 * 仅限单个采集线程使用，其他线程维护时必须与采集共用外层锁。
 */
class RecordingBgraBufferPool final {
public:
    /** 创建像素池，budgetBytes 为容量预算；默认值读取采集内存配置。 */
    explicit RecordingBgraBufferPool(quint64 budgetBytes = recordingCaptureMemoryBudget());
    RecordingBgraBufferPool(const RecordingBgraBufferPool &) = delete;
    RecordingBgraBufferPool &operator=(const RecordingBgraBufferPool &) = delete;

    /**
     * 获取一块指定字节数的可写缓冲。
     * @param byteCount 需要的字节数。
     * @param nowMs 单调毫秒时间，可注入确定性测试时钟。
     * @return 可写槽位指针；预算或槽位耗尽、参数非法时返回空。指针只在下次池操作前有效。
     */
    QByteArray *tryAcquire(qsizetype byteCount, qint64 nowMs = recordingCaptureClockMs());

    /** 返回池当前引用的总容量（字节），含下游共享槽位；无参数，不等于进程驻留内存。 */
    qsizetype retainedCapacity() const;

    /** 依据 nowMs 回收连续空闲两秒的未共享槽位，不释放借出中的像素，无返回值。 */
    void reclaimIdle(qint64 nowMs = recordingCaptureClockMs());

    /** 停止采集后归还池引用；外部持有的 QByteArray 仍有效，无参数和返回值。 */
    void reset();

private:
    static constexpr int kSlotCount = 4;

    quint64 m_budgetBytes;
    std::array<QByteArray, kSlotCount> m_slots;
    std::array<qint64, kSlotCount> m_idleSince{{-1, -1, -1, -1}};
    int m_cursor = kSlotCount - 1;
};

}  // namespace markshot::recording
