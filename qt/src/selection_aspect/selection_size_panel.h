#pragma once

#include <QPointer>
#include <QRect>
#include <QSize>
#include <QWidget>

#include "selection_aspect/selection_aspect_ratio.h"
#include "selection_aspect/selection_hud_colors.h"

namespace markshot::selection_aspect {

class SelectionRatioPopup;
class SelectionSizeField;

/**
 * 截图选区左上角的紧凑尺寸 HUD：实时显示像素宽高，支持行内修改尺寸、锁定比例与比例预设下拉。
 * 背景与图标全部自绘，数值单位为截图图像像素。
 */
class SelectionSizePanel final : public QWidget {
    Q_OBJECT

public:
    /**
     * 创建尺寸 HUD。
     * @param parent 父控件，通常是截图窗口；比例弹层同样挂在该控件下。
     */
    explicit SelectionSizePanel(QWidget *parent);

    /**
     * 同步当前选区尺寸、输入上限与锁定比例，不发出信号；正在编辑的字段保留用户输入。
     * @param size 当前选区尺寸。
     * @param maximum 图像尺寸，作为输入上限。
     * @param ratio 当前锁定比例，0 表示自由。
     * @return 无返回值。
     */
    void setValues(QSize size, QSize maximum, qreal ratio);

    /**
     * 让宽度字段进入行内编辑并全选，供快捷键调用。
     * @return 无返回值。
     */
    void focusWidth();

    /**
     * 是否有字段处于编辑状态或比例弹层已打开。
     * @return 交互中返回 true。
     */
    bool isInteracting() const;

    /**
     * 监听控件的显示、隐藏、移动与缩放，变化时请求重新定位 HUD。
     * @param widget 需要避让的控件，例如标注文本输入框；为空时忽略。
     * @return 无返回值。
     */
    void watchObstacle(QWidget *widget);

    /**
     * 【选区尺寸】【主题】按截图会话的工具栏配色设置 HUD、输入框与比例弹层颜色。
     * @param palette 截图会话的工具栏配色。
     * @return 无返回值。
     */
    void setToolbarPalette(const ui::ToolbarPalette &palette);

    /**
     * 按字体与内容计算的推荐尺寸。
     * @return 推荐尺寸。
     */
    QSize sizeHint() const override;

signals:
    /**
     * 用户修改宽或高后请求应用的新尺寸，已按锁定比例联动并限制在上限内。
     * @param size 目标尺寸（图像像素）。
     */
    void sizeApplied(QSize size);

    /**
     * 用户切换锁定状态或选择比例预设。
     * @param ratio 宽高比，0 表示自由比例。
     */
    void aspectRatioChanged(qreal ratio);

    /**
     * 编辑或弹层结束，需要把键盘焦点还给截图窗口。
     */
    void focusReleaseRequested();

    /**
     * 被监听的避让控件发生变化，需要重新定位 HUD。
     */
    void layoutRequested();

protected:
    /**
     * 绘制背景、分隔符、单位、锁形图标与比例触发区。
     * @param event 绘制事件。
     * @return 无返回值。
     */
    void paintEvent(QPaintEvent *event) override;

    /**
     * 点击锁形图标切换锁定，点击比例区打开弹层；其余区域吞掉事件避免穿透到画布。
     * @param event 鼠标事件。
     * @return 无返回值。
     */
    void mousePressEvent(QMouseEvent *event) override;

    /**
     * 跟踪悬停区域以绘制高亮。
     * @param event 鼠标事件。
     * @return 无返回值。
     */
    void mouseMoveEvent(QMouseEvent *event) override;

    /**
     * 清除悬停高亮。
     * @param event 离开事件。
     * @return 无返回值。
     */
    void leaveEvent(QEvent *event) override;

    /**
     * 吞掉滚轮事件，避免缩放底层画布。
     * @param event 滚轮事件。
     * @return 无返回值。
     */
    void wheelEvent(QWheelEvent *event) override;

    /**
     * 隐藏时结束编辑并关闭弹层。
     * @param event 隐藏事件。
     * @return 无返回值。
     */
    void hideEvent(QHideEvent *event) override;

    /**
     * 监听避让控件的几何与可见性变化。
     * @param watched 事件目标。
     * @param event 事件。
     * @return 始终返回 false，不拦截事件。
     */
    bool eventFilter(QObject *watched, QEvent *event) override;

private:
    /**
     * 可点击的自绘区域。
     */
    enum class HotZone {
        None,
        Lock,
        Ratio,
    };

    /**
     * 按当前字体重新排列子控件与自绘区域。
     * @return 无返回值。
     */
    void relayout();

    /**
     * 查找坐标所在的自绘区域。
     * @param position 控件坐标。
     * @return 区域类型。
     */
    HotZone zoneAt(const QPoint &position) const;

    /**
     * 当前比例在触发区显示的文本。
     * @return 预设名称、自定义或自由。
     */
    QString ratioLabel() const;

    /**
     * 滚轮调整某一侧并按比例联动。
     * @param field 被调整的一侧。
     * @param delta 调整量。
     * @return 无返回值。
     */
    void stepField(SizeField field, int delta);

    /**
     * 提交某一侧的输入值并按比例联动。
     * @param field 被修改的一侧。
     * @param value 输入值。
     * @return 无返回值。
     */
    void commitField(SizeField field, int value);

    /**
     * 发出联动后的尺寸，尺寸未变化时不发信号。
     * @param requested 用户请求的尺寸。
     * @param edited 被修改的一侧。
     * @return 无返回值。
     */
    void emitLinkedSize(QSize requested, SizeField edited);

    /**
     * 切换锁定：已锁定时解锁，未锁定时锁定当前选区的宽高比。
     * @return 无返回值。
     */
    void toggleLock();

    /**
     * 打开或关闭比例弹层。
     * @return 无返回值。
     */
    void toggleRatioPopup();

    SelectionSizeField *m_widthField = nullptr;
    SelectionSizeField *m_heightField = nullptr;
    QPointer<SelectionRatioPopup> m_ratioPopup;
    hud_style::HudColors m_colors = hud_style::hudColors();
    QRect m_timesRect;
    QRect m_unitRect;
    QRect m_separatorRect;
    QRect m_lockRect;
    QRect m_ratioRect;
    QSize m_contentSize;
    QSize m_size = QSize(1, 1);
    QSize m_maximum = QSize(1, 1);
    qreal m_ratio = 0.0;
    HotZone m_hoverZone = HotZone::None;
    bool m_layoutPending = false;
};

}  // namespace markshot::selection_aspect
