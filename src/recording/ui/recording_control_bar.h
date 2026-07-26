#pragma once

#include <QWidget>

class QLabel;
class QToolButton;

namespace markshot::recording::ui {

/**
 * 【录制】【控制条】录制进行中的悬浮控制条。
 *
 * 显示录制状态指示、已录时长，并提供暂停与停止入口。
 */
class RecordingControlBar final : public QWidget {
    Q_OBJECT

public:
    /**
     * 创建录制控制条。
     * @param parent 父控件。
     */
    explicit RecordingControlBar(QWidget *parent = nullptr);

    /**
     * 更新控制条显示内容。
     * @param elapsedMs 已录制毫秒数。
     * @param paused 是否处于暂停状态。
     * @return 无返回值。
     */
    void updateStatus(qint64 elapsedMs, bool paused);

signals:
    void pauseToggleRequested();
    void stopRequested();

protected:
    /**
     * 绘制控制条背景。
     * @param event 绘制事件。
     * @return 无返回值。
     */
    void paintEvent(QPaintEvent *event) override;

private:
    /**
     * 构建控制条内部控件。
     * @return 无返回值。
     */
    void buildLayout();

    /**
     * 按暂停状态刷新按钮图标与提示。
     * @return 无返回值。
     */
    void refreshPauseButton();

    QLabel *m_indicator = nullptr;
    QLabel *m_elapsed = nullptr;
    QToolButton *m_pause = nullptr;
    QToolButton *m_stop = nullptr;
    bool m_paused = false;
};

}  // namespace markshot::recording::ui
