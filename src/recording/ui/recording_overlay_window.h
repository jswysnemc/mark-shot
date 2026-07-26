#pragma once

#include <QRect>
#include <QWidget>

class QScreen;

namespace markshot::recording::ui {

class RecordingControlBar;

/**
 * 【录制】【覆盖层】录制进行中的屏幕覆盖窗口。
 *
 * 覆盖窗口铺满目标屏幕，只在录制区域边框与控制条位置保留可见与可点击区域，
 * 其余部分完全透传，不影响录制期间的正常操作。
 */
class RecordingOverlayWindow final : public QWidget {
    Q_OBJECT

public:
    /**
     * 创建录制覆盖窗口。
     * @param screen 目标屏幕。
     * @param regionRect 录制区域矩形，使用屏幕坐标。
     * @param recordsWholeScreen 是否为整屏录制。
     * @param parent 父控件。
     */
    RecordingOverlayWindow(QScreen *screen,
                           const QRect &regionRect,
                           bool recordsWholeScreen,
                           QWidget *parent = nullptr);

    /**
     * 更新控制条显示内容。
     * @param elapsedMs 已录制毫秒数。
     * @param paused 是否处于暂停状态。
     * @return 无返回值。
     */
    void updateStatus(qint64 elapsedMs, bool paused);

    /**
     * 判断覆盖窗口是否有需要显示的内容。
     * @return 有边框或控制条时返回 true。
     */
    bool hasVisibleContent() const;

signals:
    void pauseToggleRequested();
    void stopRequested();

protected:
    /**
     * 绘制录制区域边框。
     * @param event 绘制事件。
     * @return 无返回值。
     */
    void paintEvent(QPaintEvent *event) override;

private:
    /**
     * 配置窗口层级与输入透传属性。
     * @param screen 目标屏幕。
     * @return 无返回值。
     */
    void configureWindow(QScreen *screen);

    /**
     * 按布局结果放置控制条并更新可交互区域。
     * @return 无返回值。
     */
    void applyPlacement();

    /**
     * 计算需要保留的可见与可点击区域。
     * @return 覆盖窗口遮罩。
     */
    QRegion buildMask() const;

    /**
     * 读取录制区域边框在窗口坐标系中的外框。
     * @return 边框外框矩形。
     */
    QRect frameRectInWindow() const;

    RecordingControlBar *m_controlBar = nullptr;
    QRect m_screenRect;
    QRect m_regionRect;
    QRect m_controlBarRect;
    bool m_recordsWholeScreen = false;
    bool m_showRegionFrame = false;
    bool m_showControlBar = false;
};

}  // namespace markshot::recording::ui
