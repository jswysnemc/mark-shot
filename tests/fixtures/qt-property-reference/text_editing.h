// 【文字编辑对照】【实际入口】驱动原版 QTextEdit、提交、局部历史与标注历史，不模拟业务结果

#pragma once
#include "editing_test_access.h"
#include <QApplication>
#include <QKeyEvent>
#include <QTextEdit>

/** 【文字编辑对照】【窗口准备】按 source 和 existing 返回私有窗口，新建时没有持久标注，已有场景保留真实文字标注。 */
std::unique_ptr<ShotWindow> TextAnnotationEditingTest::editingWindow(const QImage &source, bool existing)
{
    auto window = create(source, false);
    window->m_textFontFamily = QStringLiteral("Inter");
    window->resize(1067,667);
    window->show();
    if (existing) {
        window->m_tool = ShotWindow::Tool::Select;
        window->beginEditingSelectedTextAnnotation();
    } else {
        window->m_annotations.clear();
        window->m_selectedAnnotationId.reset();
        window->m_selectedAnnotationIds.clear();
        window->m_tool = ShotWindow::Tool::Text;
        window->beginTextAnnotation(QPointF(400,300));
    }
    QApplication::processEvents();
    return window;
}

/** 【文字编辑对照】【状态记录】读取 window 的持久标注、编辑器正文和历史可用性，返回实际值，不推算提交结果。 */
QJsonObject TextAnnotationEditingTest::editingState(const ShotWindow &window)
{
    QJsonArray annotations;
    for (const auto &annotation : window.m_annotations) {
        annotations.append(QJsonObject{{"id",annotation.id},{"text",annotation.text},{"points",19 + annotation.width}});
    }
    return QJsonObject{{"annotations",annotations},{"editor_text",window.m_textEditor->toPlainText()},
        {"editor_geometry",QJsonArray{window.m_textEditor->x(),window.m_textEditor->y(),window.m_textEditor->width(),window.m_textEditor->height()}},
        {"editor_cursor",window.m_textEditor->textCursor().position()},
        {"editor_visible",window.m_textEditor->isVisible()},
        {"editor_undo",window.m_textEditor->document()->isUndoAvailable()},
        {"editor_redo",window.m_textEditor->document()->isRedoAvailable()},
        {"selected",window.m_selectedAnnotationId.value_or(-1)},
        {"tool",window.m_tool == ShotWindow::Tool::Text ? "text" : "select"}};
}

/** 【文字编辑对照】【按键发送】向 window 的真实编辑器发送 key、modifiers 与 text，无返回值，原版事件过滤器继续负责提交。 */
void TextAnnotationEditingTest::editorKey(ShotWindow &window, int key, Qt::KeyboardModifiers modifiers, const QString &text)
{
    QKeyEvent event(QEvent::KeyPress,key,modifiers,text);
    QApplication::sendEvent(window.m_textEditor,&event);
}

/** 【文字编辑对照】【完整样本】对 source 执行新建、重新编辑、局部撤销和提交；返回真实状态序列。 */
QJsonArray TextAnnotationEditingTest::editing(const QImage &source)
{
    QJsonArray result;
    for (const QString &entry : {QStringLiteral("escape"),QStringLiteral("control-enter"),QStringLiteral("outside")}) {
        auto window = editingWindow(source,false);
        QJsonArray states{editingState(*window)};
        editorKey(*window,Qt::Key_unknown,Qt::NoModifier,QStringLiteral("  alpha\n\n  "));
        states.append(editingState(*window));
        if (entry == "outside") window->commitTextEditor();
        else editorKey(*window,entry == "escape" ? Qt::Key_Escape : Qt::Key_Return,
            entry == "escape" ? Qt::NoModifier : Qt::ControlModifier);
        states.append(editingState(*window));
        window->undoAnnotationEdit();
        states.append(editingState(*window));
        window->redoAnnotation();
        states.append(editingState(*window));
        result.append(QJsonObject{{"name","new-"+entry},{"states",states}});
    }
    for (const QString &text : {QString{},QStringLiteral(" \n\t\u00a0 ")}) {
        auto window = editingWindow(source,false);
        editorKey(*window,Qt::Key_unknown,Qt::NoModifier,text);
        const auto before = editingState(*window);
        window->commitTextEditor();
        result.append(QJsonObject{{"name",text.isEmpty() ? "new-empty" : "new-whitespace"},
            {"states",QJsonArray{before,editingState(*window)}}});
    }
    for (const QString &entry : {QStringLiteral("changed"),QStringLiteral("deleted"),QStringLiteral("unchanged")}) {
        auto window = editingWindow(source,true);
        QJsonArray states{editingState(*window)};
        if (entry != "unchanged") {
            editorKey(*window,Qt::Key_A,Qt::ControlModifier);
            if (entry == "changed") editorKey(*window,Qt::Key_unknown,Qt::NoModifier,QStringLiteral("updated  "));
            else editorKey(*window,Qt::Key_Backspace,Qt::NoModifier);
        }
        states.append(editingState(*window));
        editorKey(*window,Qt::Key_Return,Qt::ControlModifier);
        states.append(editingState(*window));
        window->undoAnnotationEdit();
        states.append(editingState(*window));
        window->redoAnnotation();
        states.append(editingState(*window));
        result.append(QJsonObject{{"name","existing-"+entry},{"states",states}});
    }
    {
        auto window = editingWindow(source,false);
        QJsonArray states{editingState(*window)};
        for (const QChar character : QStringLiteral("one two")) editorKey(*window,Qt::Key_unknown,Qt::NoModifier,QString(character));
        states.append(editingState(*window));
        editorKey(*window,Qt::Key_Return,Qt::NoModifier,QStringLiteral("\r"));
        for (const QChar character : QStringLiteral("three")) editorKey(*window,Qt::Key_unknown,Qt::NoModifier,QString(character));
        states.append(editingState(*window));
        for (int index=0;index<4;++index) {
            editorKey(*window,Qt::Key_Z,Qt::ControlModifier);
            states.append(editingState(*window));
        }
        result.append(QJsonObject{{"name","local-undo"},{"states",states}});
    }
    return result;
}
