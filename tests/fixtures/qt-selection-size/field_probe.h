#pragma once

#include <QEvent>
#include <QJsonArray>
#include <QJsonObject>
#include <QLineEdit>
#include <QStringList>

#define private public
#include "selection_aspect/selection_size_field.h"
#undef private

namespace aspect = markshot::selection_aspect;

/** 【字段导航对照】【字段状态】按 editor 返回正文、光标、选择和历史，不改变输入 */
QJsonObject state(const QLineEdit &editor);

/** 【字段导航对照】【真实按键】向 editor 投递 value 与 modifiers，返回空值 */
void key(QLineEdit &editor, int value, Qt::KeyboardModifiers modifiers);

/** 【字段导航对照】【真实指针】向 editor 投递 type、point、button、buttons 与 modifiers，返回空值 */
void mouse(QLineEdit &editor, QEvent::Type type, QPointF point, Qt::MouseButton button,
           Qt::MouseButtons buttons, Qt::KeyboardModifiers modifiers = Qt::NoModifier);

/** 【字段导航对照】【独立控件】在 host 中创建 field，以 text 初始化真实输入框，返回输入框引用 */
QLineEdit &prepare(QWidget &host, aspect::SelectionSizeField &field, const QString &text);

/** 【字段导航对照】【内部视口】按 editor 与 x 读取生产控件的排版、滚动和字符命中，返回真实度量 */
QJsonObject metrics(QLineEdit &editor, int x);

/** 【字段点击对照】【平台序列】按 texts 生成 Qt 平台分派及字段三击的完整序列，返回事件和字段状态 */
QJsonArray click_sequences(const QStringList &texts);
