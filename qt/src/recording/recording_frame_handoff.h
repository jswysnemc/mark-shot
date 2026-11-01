#pragma once

#include "recording/recording_frame_sample.h"

#include <QObject>

#include <functional>
#include <memory>

namespace markshot::recording {

/**
 * 【录制】【帧交接】最多保留一个最新待交付帧和一个唤醒通知，另允许当前回调持有一帧。
 * 发布句柄可以安全晚于会话释放；预算不包含生产者、编码器或接收方主动保留的帧。
 */
class RecordingFrameHandoff final : public QObject {
    Q_OBJECT

public:
    using FramePublisher = std::function<void(RecordingFrameSample)>;
    using ErrorPublisher = std::function<void(QString)>;

    /** 创建属于 parent 的交接器，所有控制方法必须在所属线程调用。 */
    explicit RecordingFrameHandoff(QObject *parent = nullptr);
    /** 关闭交接器并使已分发给后台的发布句柄失效。 */
    ~RecordingFrameHandoff() override;

    /** 返回可跨线程复制和调用的帧发布句柄；输入样本通过共享所有权交接，不复制像素。 */
    FramePublisher publisher() const;
    /** 返回可跨线程调用的错误发布句柄；输入错误非空时终止本会话的帧交接。 */
    ErrorPublisher errorPublisher() const;
    /** 按 enabled 启停帧接收；禁用时释放待交接帧，但不撤销错误或关闭状态，无返回值。 */
    void setEnabled(bool enabled);
    /** 永久关闭当前会话，立即释放待交接载荷，后续发布不产生回调，无返回值。 */
    void close();

signals:
    /** 在所属线程交付 sample；接收方若跨线程连接，必须自行约束那一段队列。 */
    void frameReady(const markshot::recording::RecordingFrameSample &sample);
    /** 在所属线程交付本会话第一个非空 error，无返回值。 */
    void failed(const QString &error);

private:
    struct State;
    std::shared_ptr<State> m_state;
};

}  // namespace markshot::recording
