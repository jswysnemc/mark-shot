#pragma once

#include "ui/toolbar_palette.h"

#include <QWidget>

class QPushButton;

namespace markshot::selection_translate {

/**
 * 【框选翻译】【译文浮条】翻译模式下浮在译文旁边：切换目标语言、在原文与译文之间切换、复制译文、退出翻译模式。
 * 作为截图窗口的子控件显示，背景吞掉鼠标事件，避免点击穿透到截图画布。
 */
class TranslateResultBar final : public QWidget {
    Q_OBJECT

public:
    /**
     * 创建译文浮条，创建后保持隐藏。
     * 样式表只在构造时设置一次，构造后再换样式表时按钮仍保留旧颜色。
     * @param palette 与截图工具栏一致的配色令牌。
     * @param parent 父控件，通常是截图窗口。
     */
    TranslateResultBar(const markshot::ui::ToolbarPalette &palette, QWidget *parent);

    /**
     * 显示当前目标语言。
     * @param languageLabel 目标语言的界面名称。
     * @return 无返回值。
     */
    void setLanguageLabel(const QString &languageLabel);

    /**
     * 同步原文与译文切换按钮的选中状态，不发出信号。
     * @param showingTranslation 区域当前显示译文时为 true。
     * @return 无返回值。
     */
    void setShowingTranslation(bool showingTranslation);

signals:
    /** 用户点击目标语言按钮，请求打开语言列表。 */
    void languageRequested();

    /** 用户在原文与译文之间切换，参数为切换后是否显示译文。 */
    void viewChanged(bool showTranslation);

    /** 用户点击复制译文。 */
    void copyRequested();

    /** 用户点击退出翻译模式。 */
    void exitRequested();

protected:
    /** 绘制圆角背景与描边，参数为绘制事件，无返回值。 */
    void paintEvent(QPaintEvent *event) override;
    /** 吞掉背景上的按下事件，参数为鼠标事件，无返回值。 */
    void mousePressEvent(QMouseEvent *event) override;
    /** 吞掉背景上的松开事件，参数为鼠标事件，无返回值。 */
    void mouseReleaseEvent(QMouseEvent *event) override;

private:
    markshot::ui::ToolbarPalette m_palette;
    QPushButton *m_languageButton = nullptr;
    QPushButton *m_originalButton = nullptr;
    QPushButton *m_translatedButton = nullptr;
};

}  // namespace markshot::selection_translate
