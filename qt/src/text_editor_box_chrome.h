#pragma once

#include <QColor>

class QEvent;
class QObject;
class QPainter;
class QTextEdit;

namespace markshot::shot {

/**
 * 记录输入框背景色，供窗口在编辑器下方绘制。
 * @param editor 文字编辑器。
 * @param backgroundColor 文字背景色。
 * @return 无返回值。
 */
void setTextEditorBoxBackground(QTextEdit *editor, const QColor &backgroundColor);

/**
 * 在编辑器所在位置绘制输入框：背景色与提交后的圆角背景一致，外加虚线边框提示输入范围。
 * @param painter 窗口画笔。
 * @param editor 文字编辑器，隐藏或未记录背景色时不绘制。
 * @param scale 标注尺寸在窗口中的缩放比例。
 * @param outlined 是否绘制虚线边框，选中框已标出该文字时传 false。
 * @return 无返回值。
 */
void paintTextEditorBox(QPainter &painter, const QTextEdit *editor, qreal scale, bool outlined);

/**
 * 处理落在输入框内边距上的鼠标事件：按下时把光标移到最近的文字位置，悬停移动之外的事件不再传给窗口。
 * @param editor 文字编辑器。
 * @param watched 事件来源。
 * @param event 当前事件。
 * @return 事件已处理时返回 true。
 */
bool handleTextEditorBoxPaddingMouse(QTextEdit *editor, QObject *watched, QEvent *event);

}  // namespace markshot::shot
