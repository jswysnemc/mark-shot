#pragma once

#include "capture_click_action.h"

#include <QPointF>
#include <Qt>

class ShotWindow;

/// 把选区快捷手势接入截图窗口：双击和中键单击共用动作分发；中键单击另外区分单击与拖动平移，
/// 只有未拖动的单击才执行配置动作，拖动仍然平移画布
class ShotWindowClickAction {
public:
    explicit ShotWindowClickAction(ShotWindow &window);

    /**
     * 把选区快捷手势动作分发到既有动作入口，各入口自行处理关闭窗口。
     * @param action 要执行的动作。
     * @return 已执行动作返回 true，None 返回 false。
     */
    bool run(markshot::CaptureClickAction action);

    /**
     * 中键开始平移画布时登记单击候选；不满足触发条件时清除候选，平移照常进行。
     * @param widgetPoint 中键按下位置，窗口坐标。
     * @param buttons 按下后仍按住的全部鼠标按键。
     * @return 无返回值。
     */
    void beginMiddleClick(QPointF widgetPoint, Qt::MouseButtons buttons);

    /**
     * 中键释放时结束手势，未拖动且仍满足触发条件时执行配置的中键单击动作。
     * @param widgetPoint 中键释放位置，窗口坐标。
     * @return 已执行动作返回 true，否则返回 false。
     */
    bool finishMiddleClick(QPointF widgetPoint);

private:
    /**
     * 判断指定位置的中键单击是否应执行配置动作。
     * @param widgetPoint 指针位置，窗口坐标。
     * @return 满足触发条件返回 true，否则返回 false。
     */
    bool canRunMiddleClick(QPointF widgetPoint) const;

    ShotWindow &m_window;
};
