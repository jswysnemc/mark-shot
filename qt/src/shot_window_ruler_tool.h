#pragma once

#include "shot_window_types.h"

#include <QPointF>
#include <Qt>

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
     * 处理尺子工具下的左键按下：继续角度定点、删除、旋转或调整已有尺子，或开始新的测量。
     * @param widgetPoint 指针位置，窗口坐标。
     * @param imagePoint 指针位置，图像像素坐标。
     * @param modifiers 按住的修饰键，按住 Shift 时角度尺的边吸附到 45 度倍数。
     * @return 当前是尺子工具并已处理时返回 true。
     */
    bool press(QPointF widgetPoint, QPointF imagePoint, Qt::KeyboardModifiers modifiers);

    /**
     * 处理尺子工具下的指针移动：刷新悬停十字线，拖动尺子或更新草稿。
     * @param widgetPoint 指针位置，窗口坐标。
     * @param imagePoint 指针位置，图像像素坐标。
     * @param modifiers 按住的修饰键，按住 Shift 时旋转和角度尺的边吸附到 45 度倍数。
     * @return 当前是尺子工具并已处理时返回 true。
     */
    bool move(QPointF widgetPoint, QPointF imagePoint, Qt::KeyboardModifiers modifiers);

    /**
     * 处理尺子工具下的左键松开：结束调整，或定下角度尺的一点，或提交长度尺。
     * @param imagePoint 指针位置，图像像素坐标。
     * @param modifiers 按住的修饰键，按住 Shift 时角度尺的边吸附到 45 度倍数。
     * @return 当前是尺子工具并已处理时返回 true。
     */
    bool release(QPointF imagePoint, Qt::KeyboardModifiers modifiers);

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
     * 拖动单把尺子的控制点、角度尺选中框或旋转角度尺时按尺子自身规则更新，供通用标注拖动调用；
     * 长度尺的旋转与其他标注一致，交给通用旋转处理。
     * @param imagePoint 指针位置，图像像素坐标。
     * @param keepAspectRatio 按住 Ctrl 时为 true：拖动角度尺选中框时等比缩放，夹角不变。
     * @param snap 按住 Shift 时为 true：角度尺的边和旋转吸附到 45 度倍数。
     * @return 当前拖动已在这里处理时返回 true。
     */
    bool updateDrag(QPointF imagePoint, bool keepAspectRatio, bool snap);

    /**
     * 多选旋转时让角度尺的三个定点绕组中心转动。
     * @param annotation 待旋转的标注，原地修改。
     * @param before 拖动前的标注。
     * @param center 组中心，图像像素坐标。
     * @param degrees 旋转角度。
     * @return 是角度尺并已旋转时返回 true。
     */
    bool rotateInGroup(Annotation &annotation, const Annotation &before, QPointF center, qreal degrees) const;

    /**
     * 判断指针在选中的尺子上命中的调整方式：先看旋转把手，再看控制点和边框；
     * 选择工具下长度尺内部也可拖动，尺子工具下内部留给新的测量；
     * 角度尺最后看选中框，框的四角和四边缩放，框内任意位置整体拖动。
     * @param window 截图窗口。
     * @param annotation 尺子标注。
     * @param imagePoint 指针位置，图像像素坐标。
     * @return 调整方式，未命中时返回 None。
     */
    static SelectionDrag dragAt(const ShotWindow &window, const Annotation &annotation, QPointF imagePoint);

    /**
     * 判断尺子工具下悬停位置对应的调整方式，用于选择光标；角度尺定点期间不命中已有尺子。
     * @param imagePoint 指针位置，图像像素坐标。
     * @param rotationDegrees 不为空时写入命中尺子的旋转角度，用于缩放光标的方向。
     * @return 调整方式，未命中时返回 None。
     */
    SelectionDrag hoverDragAt(QPointF imagePoint, qreal *rotationDegrees = nullptr) const;

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
    QPointF anglePoint(const QVector<QPointF> &points, QPointF imagePoint, Qt::KeyboardModifiers modifiers) const;
    std::optional<std::pair<int, SelectionDrag>> hitAt(QPointF imagePoint) const;
    static bool rotationHandleHit(const ShotWindow &window, const Annotation &annotation, QPointF imagePoint);
    void rotateAngle(Annotation &annotation, QPointF imagePoint, bool snap) const;
    void resizeAngle(Annotation &annotation, QPointF imagePoint, bool keepAspectRatio) const;

    ShotWindow &m_window;
};
