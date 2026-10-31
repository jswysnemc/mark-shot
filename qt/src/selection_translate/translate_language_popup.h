#pragma once

#include "translation_language_options.h"
#include "ui/toolbar_palette.h"

#include <QRect>
#include <QVector>
#include <QWidget>

namespace markshot::selection_translate {

/**
 * 【框选翻译】【目标语言】点击翻译后弹出的语言列表，自绘标题、语言名与当前语言标记。
 * 作为截图窗口的子控件显示，不创建独立顶层窗口，避免 Wayland 下弹窗定位与焦点问题。
 */
class TranslateLanguagePopup final : public QWidget {
    Q_OBJECT

public:
    /**
     * 创建语言弹层，创建后保持隐藏。
     * @param parent 父控件，通常是截图窗口。
     */
    explicit TranslateLanguagePopup(QWidget *parent);

    /**
     * 设置弹层配色。
     * @param palette 与截图工具栏一致的配色令牌。
     * @return 无返回值。
     */
    void setToolbarPalette(const markshot::ui::ToolbarPalette &palette);

    /**
     * 设置语言列表；当前语言不在内置列表中时放在第一行，保证始终能看到当前选择。
     * @param options 内置语言选项。
     * @param currentValue 当前目标语言的实际值。
     * @return 无返回值。
     */
    void setLanguages(QVector<markshot::TranslationLanguageOption> options, const QString &currentValue);

    /**
     * 按给定几何显示弹层，接管键盘并监听弹层外的点击。
     * @param geometry 弹层在父控件中的矩形。
     * @return 无返回值。
     */
    void popup(const QRect &geometry);

    /**
     * 关闭弹层并发出 closed 信号。
     * @return 无返回值。
     */
    void dismiss();

    /**
     * 按最长语言名计算的推荐尺寸。
     * @return 推荐尺寸。
     */
    QSize sizeHint() const override;

signals:
    /**
     * 用户选中目标语言，弹层已关闭。
     * @param value 实际传给翻译器的语言名称。
     */
    void languageChosen(const QString &value);

    /** 弹层已关闭。 */
    void closed();

protected:
    /** 绘制背景、标题和语言行，参数为绘制事件，无返回值。 */
    void paintEvent(QPaintEvent *event) override;
    /** 跟踪悬停行，参数为鼠标事件，无返回值。 */
    void mouseMoveEvent(QMouseEvent *event) override;
    /** 吞掉按下事件，避免穿透到截图画布，参数为鼠标事件，无返回值。 */
    void mousePressEvent(QMouseEvent *event) override;
    /** 左键松开时选中悬停行，参数为鼠标事件，无返回值。 */
    void mouseReleaseEvent(QMouseEvent *event) override;
    /** 上下键移动、回车确认、Esc 关闭，参数为键盘事件，无返回值。 */
    void keyPressEvent(QKeyEvent *event) override;
    /** 隐藏时移除应用级事件过滤器，参数为隐藏事件，无返回值。 */
    void hideEvent(QHideEvent *event) override;
    /** 弹层外按下时关闭弹层，并屏蔽截图窗口快捷键，返回事件是否已处理。 */
    bool eventFilter(QObject *watched, QEvent *event) override;

private:
    /**
     * 计算语言行的矩形。
     * @param index 行下标。
     * @return 行矩形，弹层坐标。
     */
    QRect rowRect(int index) const;

    /**
     * 查找坐标所在的语言行。
     * @param position 弹层坐标。
     * @return 行下标，不在任何行上时返回 -1。
     */
    int rowAt(const QPoint &position) const;

    /**
     * 选中语言并关闭弹层。
     * @param index 行下标。
     * @return 无返回值。
     */
    void choose(int index);

    markshot::ui::ToolbarPalette m_palette;
    QVector<markshot::TranslationLanguageOption> m_options;
    int m_currentIndex = -1;
    int m_activeIndex = 0;
};

}  // namespace markshot::selection_translate
