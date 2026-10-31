#pragma once

#include "ui/toolbar_palette.h"

#include <QComboBox>
#include <QPointer>

namespace markshot::ui {

class ContextMenu;

/**
 * 截图属性栏使用的紧凑下拉框：保留 QComboBox 数据接口，按钮与弹出列表均为自绘样式，配色跟随工具栏。
 */
class CompactComboBox final : public QComboBox {
    Q_OBJECT

public:
    /**
     * 创建紧凑下拉框。
     * @param parent 父控件。
     */
    explicit CompactComboBox(QWidget *parent = nullptr);

    /**
     * 按当前选项生成自绘菜单，并在按钮下方展开。
     * @return 无返回值。
     */
    void showPopup() override;

    /**
     * 关闭自绘菜单。
     * @return 无返回值。
     */
    void hidePopup() override;

    /**
     * 按最长选项计算紧凑尺寸，避免选中项变化时按钮宽度跳动。
     * @return 推荐尺寸。
     */
    QSize sizeHint() const override;

    /**
     * 与推荐尺寸保持一致。
     * @return 最小推荐尺寸。
     */
    QSize minimumSizeHint() const override;

    /**
     * 设置按钮与弹出菜单使用的工具栏配色。
     * @param palette 工具栏配色令牌。
     * @return 无返回值。
     */
    void setToolbarPalette(const ToolbarPalette &palette);

protected:
    /**
     * 绘制圆角按钮、当前文本与下拉箭头。
     * @param event 绘制事件。
     * @return 无返回值。
     */
    void paintEvent(QPaintEvent *event) override;

    /**
     * 记录悬停状态并刷新。
     * @param event 进入事件。
     * @return 无返回值。
     */
    void enterEvent(QEnterEvent *event) override;

    /**
     * 清除悬停状态并刷新。
     * @param event 离开事件。
     * @return 无返回值。
     */
    void leaveEvent(QEvent *event) override;

private:
    QPointer<ContextMenu> m_menu;
    ToolbarPalette m_toolbarPalette;
    bool m_hovered = false;
};

}  // namespace markshot::ui
