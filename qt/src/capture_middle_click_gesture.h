#pragma once

#include "capture_middle_click_action.h"

#include <QPointF>

namespace markshot {

// 选区中键手势：保存配置的中键单击动作，并区分一次中键按下最终是单击还是拖动平移。
// 指针移动未超过拖动阈值时仍是单击候选，超过阈值后本次按下转为平移，直到下一次按下前不再视为单击。
class CaptureMiddleClickGesture {
public:
    /// @brief 设置中键单击执行的动作。
    /// @param action 配置的动作，None 表示保留中键内置行为。
    void setAction(CaptureClickAction action);

    /// @brief 返回中键单击执行的动作。
    /// @return 配置的动作。
    CaptureClickAction action() const;

    /// @brief 在中键按下处登记单击候选。
    /// @param position 按下位置，窗口坐标。
    /// @param dragThreshold 判定为拖动的最小曼哈顿距离，小于 1 时按 1 处理。
    void press(QPointF position, int dragThreshold);

    /// @brief 根据指针移动更新单击候选。
    /// @param position 当前指针位置，窗口坐标。
    /// @return 仍是单击候选时返回 true；超过阈值或没有候选时返回 false。
    bool track(QPointF position);

    /// @brief 在中键释放时结束本次手势。
    /// @param position 释放位置，窗口坐标。
    /// @return 本次按下构成单击时返回 true，否则返回 false。
    bool release(QPointF position);

    /// @brief 放弃当前单击候选，例如交互被中断或同时按下了其他按键。
    void cancel();

    /// @brief 判断当前是否存在单击候选。
    /// @return 存在候选时返回 true。
    bool pending() const;

private:
    /// @brief 判断位置是否仍在按下点的拖动阈值内。
    /// @param position 窗口坐标中的指针位置。
    /// @return 曼哈顿距离小于阈值时返回 true。
    bool withinThreshold(QPointF position) const;

    CaptureClickAction m_action = defaultCaptureMiddleClickAction();
    QPointF m_pressPosition;
    int m_dragThreshold = 1;
    bool m_pending = false;
};

}  // namespace markshot
