#pragma once

#include "shot_window_types.h"

#include <QPointF>

#include <optional>
#include <utility>

class QHBoxLayout;
class ShotWindow;

/// 把尺子工具接入截图窗口：指针输入、控制点调整、测量方式切换和选区阶段的尺子快捷键入口
class ShotWindowRulerTool {
public:
    using Annotation = markshot::shot::types::Annotation;
    using RulerMode = markshot::shot::types::RulerMode;
    using SelectionDrag = markshot::shot::types::SelectionDrag;

    explicit ShotWindowRulerTool(ShotWindow &window);

    /**
     * 计算尺子控制点和边框的命中半径。
     * @param window 截图窗口。
     * @return 命中半径，图像像素，对应屏幕上 8 像素。
     */
    static qreal imageTolerance(const ShotWindow &window);

    /**
     * 处理尺子工具下的左键按下：继续角度定点、删除、调整已有尺子或开始新的测量。
     * @param widgetPoint 指针位置，窗口坐标。
     * @param imagePoint 指针位置，图像像素坐标。
     * @return 当前是尺子工具并已处理时返回 true。
     */
    bool press(QPointF widgetPoint, QPointF imagePoint);

    /**
     * 处理尺子工具下的指针移动：刷新悬停十字线，拖动尺子或更新草稿。
     * @param widgetPoint 指针位置，窗口坐标。
     * @param imagePoint 指针位置，图像像素坐标。
     * @return 当前是尺子工具并已处理时返回 true。
     */
    bool move(QPointF widgetPoint, QPointF imagePoint);

    /**
     * 处理尺子工具下的左键松开：结束调整，或定下角度尺的一点，或提交长度尺。
     * @param imagePoint 指针位置，图像像素坐标。
     * @return 当前是尺子工具并已处理时返回 true。
     */
    bool release(QPointF imagePoint);

    /**
     * 判断角度尺是否正在等待下一次单击定点。
     * @return 有角度尺草稿且没有按住左键时返回 true。
     */
    bool anglePending() const;

    /**
     * 撤去未完成的尺子草稿。
     * @return 有尺子草稿并已撤去时返回 true。
     */
    bool cancelDraft();

    /**
     * 拖动控制点时按尺子自身规则更新形状，供通用标注拖动调用。
     * @param imagePoint 指针位置，图像像素坐标。
     * @return 当前拖动属于尺子控制点并已处理时返回 true。
     */
    bool updateDrag(QPointF imagePoint);

    /**
     * 判断指针在尺子上命中的调整方式：选择工具下尺子内部也可拖动，尺子工具下内部留给新的测量。
     * @param window 截图窗口。
     * @param annotation 尺子标注。
     * @param imagePoint 指针位置，图像像素坐标。
     * @return 调整方式，未命中时返回 None。
     */
    static SelectionDrag dragAt(const ShotWindow &window, const Annotation &annotation, QPointF imagePoint);

    /**
     * 判断尺子工具下悬停位置对应的调整方式，用于选择光标；角度尺定点期间不命中已有尺子。
     * @param imagePoint 指针位置，图像像素坐标。
     * @return 调整方式，未命中时返回 None。
     */
    SelectionDrag hoverDragAt(QPointF imagePoint) const;

    /**
     * 指针离开窗口时隐藏悬停十字线。
     * @return 无返回值。
     */
    void clearHover();

    /**
     * 选中尺子工具；已是尺子工具时在长度和角度之间切换。
     * @return 无返回值。
     */
    void activate();

    /**
     * 设置新画尺子的测量方式，并撤去未完成的草稿。
     * @param mode 测量方式。
     * @return 无返回值。
     */
    void setMode(RulerMode mode);

    /**
     * 在选区阶段按尺子快捷键后进入整屏标注并选中尺子工具。
     * @return 无返回值。
     */
    void enterFromSelection();

    /**
     * 处理尺子工具中的 Esc：先撤去草稿；由选区阶段进入时丢弃测量并回到选区阶段。
     * @return 已处理时返回 true，返回 false 时按原有逻辑关闭截图。
     */
    bool leaveToSelection();

    /**
     * 在属性栏中加入测量方式下拉框。
     * @param layout 属性栏布局。
     * @return 无返回值。
     */
    void initializePanel(QHBoxLayout *layout);

    /**
     * 同步测量方式下拉框的显隐和当前值。
     * @param visible 是否显示。
     * @return 无返回值。
     */
    void syncPanel(bool visible);

private:
    bool active() const;
    qreal minimumAngleArm() const;
    std::optional<std::pair<int, SelectionDrag>> hitAt(QPointF imagePoint) const;

    ShotWindow &m_window;
};
