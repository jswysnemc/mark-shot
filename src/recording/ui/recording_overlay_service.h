#pragma once

#include <QObject>
#include <QPointer>

class QTimer;

namespace markshot::recording {

struct RecordingStatus;

namespace ui {

class RecordingOverlayWindow;

/**
 * 【录制】【覆盖层】按录制会话状态管理覆盖层的生命周期。
 *
 * 录制开始时创建覆盖层，结束时销毁，并周期性刷新控制条上的计时。
 */
class RecordingOverlayService final : public QObject {
    Q_OBJECT

public:
    /**
     * 读取覆盖层服务单例。
     * @return 服务实例。
     */
    static RecordingOverlayService &instance();

    /**
     * 开始跟随录制会话状态。
     * @return 无返回值。
     */
    void attach();

private:
    /**
     * 创建覆盖层服务。
     * @param parent 父对象。
     */
    explicit RecordingOverlayService(QObject *parent = nullptr);

    /**
     * 处理录制状态变化。
     * @return 无返回值。
     */
    void handleStatusChanged();

    /**
     * 按当前录制状态创建覆盖层。
     * @param status 录制状态。
     * @return 无返回值。
     */
    void createOverlay(const RecordingStatus &status);

    /**
     * 销毁覆盖层。
     * @return 无返回值。
     */
    void destroyOverlay();

    /**
     * 刷新控制条显示。
     * @return 无返回值。
     */
    void refreshOverlay();

    QPointer<RecordingOverlayWindow> m_overlay;
    QTimer *m_refreshTimer = nullptr;
    bool m_attached = false;
};

}  // namespace ui
}  // namespace markshot::recording
