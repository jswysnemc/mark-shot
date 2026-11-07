#include "field_probe.h"

#include <QApplication>
#include <QKeyEvent>
#include <QMouseEvent>
#include <QStyleOption>
#include <QtWidgets/private/qlineedit_p.h>

/** 【字段导航对照】【字段状态】按 editor 返回正文、光标、选择和历史，不改变输入 */
QJsonObject state(const QLineEdit &editor)
{
    return {{"text", editor.text()}, {"cursor", editor.cursorPosition()},
            {"selected", editor.selectedText()}, {"selection_start", editor.selectionStart()},
            {"undo", editor.isUndoAvailable()}, {"redo", editor.isRedoAvailable()}};
}

/** 【字段导航对照】【真实按键】向 editor 投递 value 与 modifiers，返回空值 */
void key(QLineEdit &editor, int value, Qt::KeyboardModifiers modifiers)
{
    QKeyEvent event(QEvent::KeyPress, value, modifiers);
    QApplication::sendEvent(&editor, &event);
    QApplication::processEvents();
}

/** 【字段导航对照】【真实指针】向 editor 投递 type、point、button、buttons 与 modifiers，返回空值 */
void mouse(QLineEdit &editor, QEvent::Type type, QPointF point, Qt::MouseButton button,
           Qt::MouseButtons buttons, Qt::KeyboardModifiers modifiers)
{
    QMouseEvent event(type, point, editor.mapToGlobal(point.toPoint()), button, buttons, modifiers);
    QApplication::sendEvent(&editor, &event);
    QApplication::processEvents();
}

/** 【字段导航对照】【独立控件】在 host 中创建 field，以 text 初始化真实输入框，返回输入框引用 */
QLineEdit &prepare(QWidget &host, aspect::SelectionSizeField &field, const QString &text)
{
    host.resize(200, 100);
    field.resize(field.sizeHint());
    field.setValue(12345, 10000000);
    host.show();
    field.beginEditing();
    QApplication::processEvents();
    field.m_editor->setText(text);
    QApplication::processEvents();
    return *field.m_editor;
}

/** 【字段导航对照】【内部视口】按 editor 与 x 读取同版本生产控件的排版、滚动和字符命中，返回真实度量 */
QJsonObject metrics(QLineEdit &editor, int x)
{
    const auto *data = static_cast<QLineEditPrivate *>(QObjectPrivate::get(&editor));
    QStyleOptionFrame option;
    option.initFrom(&editor);
    option.lineWidth = editor.hasFrame() ? editor.style()->pixelMetric(QStyle::PM_DefaultFrameWidth, &option, &editor) : 0;
    const QRect contents = editor.style()->subElementRect(QStyle::SE_LineEditContents, &option, &editor);
    const QTextLine line = data->control->textLayout()->lineAt(0);
    QJsonArray positions;
    for (int index = 0; index <= editor.text().size(); ++index) {
        positions.append(data->control->cursorToX(index));
    }
    return {{"hscroll", data->hscroll}, {"contents", QJsonArray{contents.x(), contents.y(), contents.width(), contents.height()}},
            {"line_width", line.width()}, {"line_x", line.x()}, {"width", data->control->width()},
            {"vscroll", data->vscroll}, {"control_height", data->control->height()}, {"control_ascent", data->control->ascent()},
            {"line_height", line.height()}, {"ascent", line.ascent()}, {"descent", line.descent()},
            {"natural_width", data->control->naturalTextWidth()}, {"positions", positions},
            {"between", editor.cursorPositionAt(QPoint(x, editor.height() / 2))}};
}
