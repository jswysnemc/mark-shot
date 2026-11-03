#pragma once

#include "selection_aspect/selection_hud_colors.h"

#include <QWidget>

class QLineEdit;

namespace markshot::selection_aspect {

/**
 * HUD 中的单个像素数值：平时自绘数字，点击后切换为行内输入框，滚轮可按步长调整。
 */
class SelectionSizeField final : public QWidget {
    Q_OBJECT

public:
    /**
     * 创建数值字段。
     * @param accessibleName 无障碍名称与提示文本。
     * @param parent 父控件，通常是尺寸 HUD。
     */
    explicit SelectionSizeField(const QString &accessibleName, QWidget *parent = nullptr);

    /**
     * 设置显示值与可输入上限；编辑中只更新上限，不覆盖用户正在输入的文本。
     * @param value 当前像素值。
     * @param maximum 可输入上限，至少为 1。
     * @return 无返回值。
     */
    void setValue(int value, int maximum);

    /**
     * 【选区尺寸】【主题】应用截图会话的 HUD 颜色并重绘。
     * @param colors HUD 颜色。
     * @return 无返回值。
     */
    void setHudColors(const hud_style::HudColors &colors);

    /**
     * 当前显示值。
     * @return 像素值。
     */
    int value() const;

    /**
     * 进入行内编辑并全选文本。
     * @return 无返回值。
     */
    void beginEditing();

    /**
     * 放弃输入并退出编辑，不发出提交信号。
     * @return 无返回值。
     */
    void cancelEditing();

    /**
     * 是否处于行内编辑状态。
     * @return 编辑中返回 true。
     */
    bool isEditing() const;

    /**
     * 按显示字体计算的推荐尺寸。
     * @return 推荐尺寸。
     */
    QSize sizeHint() const override;

signals:
    /**
     * 用户确认输入值。
     * @param value 输入值，已限制在 1 与上限之间。
     */
    void valueCommitted(int value);

    /**
     * 用户滚动滚轮请求调整数值。
     * @param delta 调整量，向上为正，按住 Shift 时为 10 倍步长。
     */
    void stepRequested(int delta);

    /**
     * 用户按 Tab 或 Shift+Tab 请求切换到另一个字段，信号发出前当前输入已提交。
     */
    void focusNextRequested();

    /**
     * 用户按 Enter 或 Esc 结束编辑，需要把键盘焦点还给截图窗口。
     */
    void editingClosed();

protected:
    /**
     * 绘制数值与悬停背景。
     * @param event 绘制事件。
     * @return 无返回值。
     */
    void paintEvent(QPaintEvent *event) override;

    /**
     * 左键点击进入编辑。
     * @param event 鼠标事件。
     * @return 无返回值。
     */
    void mousePressEvent(QMouseEvent *event) override;

    /**
     * 滚轮按步长调整；未编辑时请求外部调整，编辑中同步输入框并立即应用。
     * @param event 滚轮事件。
     * @return 无返回值。
     */
    void wheelEvent(QWheelEvent *event) override;

    /**
     * 记录悬停状态。
     * @param event 进入事件。
     * @return 无返回值。
     */
    void enterEvent(QEnterEvent *event) override;

    /**
     * 清除悬停状态。
     * @param event 离开事件。
     * @return 无返回值。
     */
    void leaveEvent(QEvent *event) override;

    /**
     * 让输入框跟随字段尺寸。
     * @param event 尺寸事件。
     * @return 无返回值。
     */
    void resizeEvent(QResizeEvent *event) override;

    /**
     * 处理输入框的按键、滚轮与失焦：Enter 提交、Esc 放弃、Tab 提交后切换、上下键微调、滚轮实时调整、失焦提交。
     * @param watched 事件目标。
     * @param event 事件。
     * @return 事件已处理时返回 true。
     */
    bool eventFilter(QObject *watched, QEvent *event) override;

private:
    /**
     * 解析输入框文本并退出编辑；文本合法时发出 valueCommitted。
     * @return 无返回值。
     */
    void commitEditing();

    /**
     * 隐藏输入框并刷新显示。
     * @return 无返回值。
     */
    void finishEditing();

    /**
     * 编辑中按步长调整输入值，并立即应用到选区，输入框保持编辑状态。
     * @param delta 调整量，向上为正。
     * @return 无返回值。
     */
    void stepEditing(int delta);

    QLineEdit *m_editor = nullptr;
    hud_style::HudColors m_colors = hud_style::hudColors();
    int m_value = 1;
    int m_maximum = 1;
    bool m_hovered = false;
    bool m_editing = false;
    // 最近一次由程序写入输入框的文本；文本被用户改动后不再跟随实际值，避免覆盖输入
    QString m_syncedText;
};

}  // namespace markshot::selection_aspect
