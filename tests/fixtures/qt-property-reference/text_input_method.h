// 【输入法对照】【实际事件】驱动原版 QTextEdit 的预编辑与提交，正文和历史均读取真实对象

#pragma once
#include "editing_test_access.h"
#include <QApplication>
#include <QInputMethodEvent>
#include <QTextBlock>
#include <QTextEdit>
#include <QTextLayout>

/** 【输入法对照】【状态记录】读取 window 文档、预编辑和 UTF-16 上下文，将真实画面写入 path，返回实际状态 */
QJsonObject TextAnnotationEditingTest::inputMethodState(ShotWindow &window, const QString &path)
{
    QApplication::processEvents();
    auto *editor = window.m_textEditor;
    QInputMethodQueryEvent query(Qt::ImSurroundingText | Qt::ImCursorPosition | Qt::ImAnchorPosition | Qt::ImCursorRectangle);
    QApplication::sendEvent(editor, &query);
    const auto cursor = editor->textCursor();
    const auto *layout = cursor.block().layout();
    auto result = editingState(window);
    result["anchor"] = cursor.anchor();
    result["undo_steps"] = editor->document()->availableUndoSteps();
    result["redo_steps"] = editor->document()->availableRedoSteps();
    result["preedit"] = layout ? layout->preeditAreaText() : QString{};
    result["preedit_position"] = layout ? layout->preeditAreaPosition() : -1;
    result["surrounding"] = query.value(Qt::ImSurroundingText).toString();
    result["surrounding_cursor"] = query.value(Qt::ImCursorPosition).toInt();
    result["surrounding_anchor"] = query.value(Qt::ImAnchorPosition).toInt();
    const auto rect = query.value(Qt::ImCursorRectangle).toRectF();
    result["cursor_rect"] = QJsonArray{rect.x(), rect.y(), rect.width(), rect.height()};
    editor->grab().save(path);
    return result;
}

/** 【输入法对照】【完整样本】按 source 创建独立编辑器，prefix 保存每个事件后的真实画面；返回事件与状态，不代写预期 */
QJsonArray TextAnnotationEditingTest::inputMethod(const QImage &source, const QString &prefix)
{
    QJsonArray result;
    // 1. 【输入法对照】【事件集合】用输入法公开事件描述候选、确认和相对替换，不直接修改原版业务代码
    const auto ime = [](const QString &preedit, const QString &commit = {}, int start = 0, int length = 0, int caret = -2) {
        return QJsonObject{{"kind", "ime"}, {"preedit", preedit}, {"commit", commit},
            {"replace_start", start}, {"replace_length", length}, {"caret", caret}};
    };
    const auto undo = QJsonObject{{"kind", "undo"}};
    const auto redo = QJsonObject{{"kind", "redo"}};
    struct Case { QString name; QString initial; int cursor; int anchor; QJsonArray events; };
    const QList<Case> cases{
        {"preedit-cancel", {}, 0, 0, {ime("ni"), ime("nih"), ime({}), undo}},
        {"unicode-direct", {}, 0, 0, {ime({}, QStringLiteral("\u4f60")), ime({}, QStringLiteral("\u597d")), undo, undo, redo, redo}},
        {"commit-and-next", {}, 0, 0, {ime("ni"), ime("hao", QStringLiteral("\u4f60")), ime({}), undo, redo}},
        {"selection-preedit-cancel", "alpha beta", 7, 3, {ime("ni"), ime({}), undo, redo}},
        {"selection-commit", "alpha beta", 7, 3, {ime("ni"), ime({}, QStringLiteral("\u4f60")), undo, redo}},
        {"empty-event-with-selection", "alpha beta", 7, 3, {ime({}), undo, redo}},
        {"backward-selection", "alpha beta", 3, 7, {ime("ni"), ime({}, QStringLiteral("\u4f60")), undo, redo}},
        {"unicode-replacement", QStringLiteral("A\u00e9\u754cZ"), 3, 3, {ime({}, "x", -2, 2), undo, redo}},
        {"supplementary-replacement", QStringLiteral("A\U00020000Z"), 3, 3, {ime({}, "x", -2, 2), undo, redo}},
        {"forward-replacement", "alpha beta", 3, 3, {ime({}, "x", 1, 3), undo, redo}},
        {"surrounding-delete", "alpha beta", 5, 5, {ime({}, {}, -2, 4), undo, redo}},
        {"multiline-preedit", "one\ntwo", 5, 5, {ime(QStringLiteral("\u62fc\u97f3")), ime({}, QStringLiteral("\u4e2d\u6587")), undo, redo}},
        {"cross-paragraph-selection", "one\ntwo\nthree", 9, 2, {ime("ni"), ime({}, QStringLiteral("\u4f60")), undo, redo}},
        {"supplementary-preedit", {}, 0, 0, {ime(QStringLiteral("\U00020000")), ime({}, QStringLiteral("\U00020000")), undo, redo}},
        {"hidden-preedit-cursor", {}, 0, 0, {ime("ni", {}, 0, 0, -1), ime({})}},
        {"commit-with-preedit", {}, 0, 0, {ime("ni"), QJsonObject{{"kind", "annotation-commit"}}}},
        {"focus-out-preedit", "alpha", 5, 5, {ime("ni"), QJsonObject{{"kind", "focus-out"}}}},
    };
    // 2. 【输入法对照】【实际分派】每组从独立真实窗口开始，应用事件之后再读取文档和历史
    for (const auto &entry : cases) {
        auto window = editingWindow(source, false);
        if (!entry.initial.isEmpty()) editorKey(*window, Qt::Key_unknown, Qt::NoModifier, entry.initial);
        auto cursor = window->m_textEditor->textCursor();
        cursor.setPosition(entry.anchor);
        cursor.setPosition(entry.cursor, QTextCursor::KeepAnchor);
        window->m_textEditor->setTextCursor(cursor);
        const auto image = [&](int index) { return prefix + "-" + entry.name + "-" + QString::number(index) + ".png"; };
        QJsonArray states{inputMethodState(*window, image(0))};
        int index = 1;
        for (const auto &value : entry.events) {
            const auto event = value.toObject();
            const auto kind = event["kind"].toString();
            if (kind == "ime") {
                const auto preedit = event["preedit"].toString();
                QTextCharFormat style;
                style.setFontUnderline(true);
                QList<QInputMethodEvent::Attribute> attributes;
                if (!preedit.isEmpty()) {
                    attributes.append({QInputMethodEvent::TextFormat, 0, int(preedit.size()), style});
                    const int caret = event["caret"].toInt();
                    attributes.append({QInputMethodEvent::Cursor, caret == -2 ? int(preedit.size()) : qMax(0, caret), caret == -1 ? 0 : 1, {}});
                }
                QInputMethodEvent input(preedit, attributes);
                input.setCommitString(event["commit"].toString(), event["replace_start"].toInt(), event["replace_length"].toInt());
                QApplication::sendEvent(window->m_textEditor, &input);
            } else if (kind == "undo" || kind == "redo") {
                editorKey(*window, Qt::Key_Z, kind == "undo" ? Qt::ControlModifier : Qt::ControlModifier | Qt::ShiftModifier);
            } else if (kind == "annotation-commit") {
                window->commitTextEditor();
            } else if (kind == "focus-out") {
                window->m_textEditor->clearFocus();
                window->setFocus();
            }
            states.append(inputMethodState(*window, image(index++)));
        }
        result.append(QJsonObject{{"name", entry.name}, {"initial", entry.initial},
            {"cursor", entry.cursor}, {"anchor", entry.anchor}, {"events", entry.events}, {"states", states}});
    }
    return result;
}
