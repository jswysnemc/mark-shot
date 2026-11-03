#pragma once

#include "selection_aspect/selection_hud_colors.h"

#include <QRect>
#include <QWidget>

namespace markshot::selection_aspect {

/**
 * 选区比例下拉弹层：自绘比例示意图、名称与选中标记。
 * 作为截图窗口的子控件显示，不创建独立顶层窗口，避免 Wayland 下弹窗定位与焦点问题。
 */
class SelectionRatioPopup final : public QWidget {
    Q_OBJECT

public:
    /**
     * 创建比例弹层。
     * @param parent 父控件，通常是截图窗口。
     */
    explicit SelectionRatioPopup(QWidget *parent);

    /**
     * 在锚点矩形下方弹出，下方空间不足时显示在上方。
     * @param anchor 触发区域在父控件中的矩形。
     * @param currentRatio 当前锁定比例，0 表示自由比例。
     * @return 无返回值。
     */
    void popup(const QRect &anchor, qreal currentRatio);

    /**
     * 关闭弹层并发出 closed 信号。
     * @return 无返回值。
     */
    void dismiss();

    /**
     * 【选区尺寸】【主题】应用截图会话的 HUD 颜色并重绘。
     * @param colors HUD 颜色。
     * @return 无返回值。
     */
    void setHudColors(const hud_style::HudColors &colors);

    /**
     * 按行数计算的推荐尺寸。
     * @return 推荐尺寸。
     */
    QSize sizeHint() const override;

signals:
    /**
     * 用户选择比例预设。
     * @param ratio 宽高比，0 表示自由比例。
     */
    void ratioSelected(qreal ratio);

    /**
     * 弹层已关闭。
     */
    void closed();

protected:
    /**
     * 绘制背景与比例行。
     * @param event 绘制事件。
     * @return 无返回值。
     */
    void paintEvent(QPaintEvent *event) override;

    /**
     * 跟踪悬停行。
     * @param event 鼠标事件。
     * @return 无返回值。
     */
    void mouseMoveEvent(QMouseEvent *event) override;

    /**
     * 左键松开时选择悬停行。
     * @param event 鼠标事件。
     * @return 无返回值。
     */
    void mouseReleaseEvent(QMouseEvent *event) override;

    /**
     * 吞掉按下事件，避免穿透到截图画布。
     * @param event 鼠标事件。
     * @return 无返回值。
     */
    void mousePressEvent(QMouseEvent *event) override;

    /**
     * 清除悬停行。
     * @param event 离开事件。
     * @return 无返回值。
     */
    void leaveEvent(QEvent *event) override;

    /**
     * 上下键移动、Enter 确认、Esc 关闭。
     * @param event 键盘事件。
     * @return 无返回值。
     */
    void keyPressEvent(QKeyEvent *event) override;

    /**
     * 隐藏时移除应用级事件过滤器。
     * @param event 隐藏事件。
     * @return 无返回值。
     */
    void hideEvent(QHideEvent *event) override;

    /**
     * 拦截弹层外部的按下事件并关闭弹层，同时屏蔽编辑键触发的快捷键。
     * @param watched 事件目标。
     * @param event 事件。
     * @return 外部按下事件已处理时返回 true。
     */
    bool eventFilter(QObject *watched, QEvent *event) override;

private:
    /**
     * 计算指定行的矩形。
     * @param index 行下标。
     * @return 行矩形。
     */
    QRect rowRect(int index) const;

    /**
     * 根据坐标查找所在行。
     * @param position 控件坐标。
     * @return 行下标，不在任何行上时返回 -1。
     */
    int rowAt(const QPoint &position) const;

    /**
     * 选择指定行并关闭弹层。
     * @param index 行下标。
     * @return 无返回值。
     */
    void choose(int index);

    QRect m_anchor;
    hud_style::HudColors m_colors = hud_style::hudColors();
    int m_currentIndex = 0;
    int m_activeIndex = 0;
};

}  // namespace markshot::selection_aspect
