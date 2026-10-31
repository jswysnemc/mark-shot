#pragma once

#include "ui/toolbar_palette.h"

#include <QWidget>

class QLabel;
class QPushButton;

namespace markshot::selection_translate {

/**
 * 【框选翻译】【译文浮条】译文写回选区后浮在译文旁边，提供复制译文和还原原文两个操作。
 * 作为截图窗口的子控件显示，背景吞掉鼠标事件，避免点击穿透到截图画布。
 */
class TranslateResultBar final : public QWidget {
    Q_OBJECT

public:
    /**
     * 创建译文浮条，创建后保持隐藏。
     * 样式表只在构造时设置一次，构造后再换样式表时按钮和说明文字仍保留旧颜色。
     * @param palette 与截图工具栏一致的配色令牌。
     * @param parent 父控件，通常是截图窗口。
     */
    TranslateResultBar(const markshot::ui::ToolbarPalette &palette, QWidget *parent);

    /**
     * 显示本次翻译的目标语言，悬停提示给出完整说明。
     * @param languageLabel 目标语言的界面名称。
     * @return 无返回值。
     */
    void setLanguageLabel(const QString &languageLabel);

signals:
    /** 用户点击复制译文。 */
    void copyRequested();

    /** 用户点击还原原文。 */
    void restoreRequested();

protected:
    /** 绘制圆角背景与描边，参数为绘制事件，无返回值。 */
    void paintEvent(QPaintEvent *event) override;
    /** 吞掉背景上的按下事件，参数为鼠标事件，无返回值。 */
    void mousePressEvent(QMouseEvent *event) override;
    /** 吞掉背景上的松开事件，参数为鼠标事件，无返回值。 */
    void mouseReleaseEvent(QMouseEvent *event) override;

private:
    markshot::ui::ToolbarPalette m_palette;
    QLabel *m_label = nullptr;
    QPushButton *m_copyButton = nullptr;
    QPushButton *m_restoreButton = nullptr;
};

}  // namespace markshot::selection_translate
