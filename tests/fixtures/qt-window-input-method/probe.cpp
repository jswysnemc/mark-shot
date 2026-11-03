#include "probe.h"

#include <QApplication>
#include <QClipboard>
#include <QFile>
#include <QInputMethodEvent>
#include <QJsonDocument>
#include <QLineEdit>
#include <QTableWidget>
#include <QTest>
#include <QTextCursor>
#include <QTextEdit>

/// 【窗口输入法对照】【字节位置】将 text 的 UTF-16 position 转为 UTF-8 字节，返回协议位置
static int bytePosition(const QString &text, int position)
{
    return text.left(position).toUtf8().size();
}

Probe::Probe(const Scenario &scenario)
{
    if (scenario.mode == "multiline") {
        multiline = new QTextEdit;
        widget.reset(multiline);
        multiline->setAcceptRichText(false);
        multiline->setWordWrapMode(QTextOption::WrapAtWordBoundaryOrAnywhere);
        multiline->document()->setDocumentMargin(2);
        multiline->setPlainText(scenario.initial);
        auto cursor = multiline->textCursor();
        cursor.setPosition(scenario.anchor < 0 ? scenario.initial.size() : scenario.anchor);
        cursor.setPosition(scenario.cursor < 0 ? scenario.initial.size() : scenario.cursor,
                           QTextCursor::KeepAnchor);
        multiline->setTextCursor(cursor);
    } else {
        singleline = new QLineEdit(scenario.initial);
        widget.reset(singleline);
        const int cursor = scenario.cursor < 0 ? scenario.initial.size() : scenario.cursor;
        const int anchor = scenario.anchor < 0 ? cursor : scenario.anchor;
        singleline->setSelection(anchor, cursor - anchor);
    }
    widget->resize(320, 180);
    widget->show();
    widget->setFocus();
    QApplication::processEvents();
}

Probe::~Probe() = default;

QJsonObject Probe::state() const
{
    const QString text = multiline ? multiline->toPlainText() : singleline->text();
    const int cursor = multiline ? multiline->textCursor().position() : singleline->cursorPosition();
    const int anchor = multiline ? multiline->textCursor().anchor()
        : singleline->selectionStart() < 0 ? cursor
        : cursor == singleline->selectionStart() ? cursor + singleline->selectedText().size()
        : singleline->selectionStart();
    return {{"text", text}, {"cursor", bytePosition(text, cursor)},
            {"anchor", bytePosition(text, anchor)},
            {"selected", multiline ? multiline->textCursor().selectedText().replace(QChar(0x2029), '\n') : singleline->selectedText()},
            {"undo", multiline ? multiline->document()->isUndoAvailable() : singleline->isUndoAvailable()},
            {"redo", multiline ? multiline->document()->isRedoAvailable() : singleline->isRedoAvailable()}};
}

QJsonObject Probe::apply(const Operation &operation)
{
    QJsonObject input{{"kind", operation.kind}};
    if (operation.kind == "input") {
        const int cursor = operation.cursor == -2 ? operation.text.size() : operation.cursor;
        QList<QInputMethodEvent::Attribute> attributes;
        if (!operation.text.isEmpty()) {
            attributes.emplaceBack(QInputMethodEvent::Cursor, cursor < 0 ? 0 : cursor,
                                   cursor < 0 ? 0 : 1, QVariant());
        }
        QInputMethodEvent event(operation.text, attributes);
        event.setCommitString(operation.commit, operation.start, operation.length);
        QApplication::sendEvent(widget.get(), &event);
        input.insert("preedit", operation.text);
        input.insert("commit", operation.commit);
        input.insert("cursor", cursor < 0 ? -1 : bytePosition(operation.text, cursor));
        input.insert("replacement_start_utf16", operation.start);
        input.insert("replacement_length_utf16", operation.length);
    } else if (operation.kind == "undo") {
        if (multiline) multiline->undo();
        else singleline->undo();
    } else if (operation.kind == "redo") {
        if (multiline) multiline->redo();
        else singleline->redo();
    } else if (operation.kind == "type") {
        QTest::keyClicks(widget.get(), operation.text);
        input.insert("text", operation.text);
    } else if (operation.kind == "select") {
        if (multiline) multiline->selectAll();
        else singleline->selectAll();
    } else if (operation.kind == "left") {
        QTest::keyClick(widget.get(), Qt::Key_Left);
    } else if (operation.kind == "backspace") {
        QTest::keyClick(widget.get(), Qt::Key_Backspace);
    } else if (operation.kind == "enter") {
        QTest::keyClick(widget.get(), Qt::Key_Return);
    } else if (operation.kind == "paste") {
        QApplication::clipboard()->setText(operation.text);
        QTest::keyClick(widget.get(), Qt::Key_V, Qt::ControlModifier);
        input.insert("text", operation.text);
    } else if (operation.kind == "cut") {
        QTest::keyClick(widget.get(), Qt::Key_X, Qt::ControlModifier);
    }
    QApplication::processEvents();
    return input;
}

/// 【窗口输入法对照】【执行入口】argv 指定输出文件，投递全部真实场景并确认表格编辑控件，返回退出状态
int main(int argc, char **argv)
{
    QApplication application(argc, argv);
    if (argc != 2) return 2;
    QTableWidget table(1, 1);
    table.setItem(0, 0, new QTableWidgetItem("cell"));
    table.show();
    table.editItem(table.item(0, 0));
    QApplication::processEvents();
    if (!table.findChild<QLineEdit *>()) return 3;
    table.hide();
    QFile output(argv[1]);
    if (!output.open(QIODevice::WriteOnly)) return 4;
    output.write("{\"qt_version\":\"");
    output.write(qVersion());
    output.write("\",\"table_editor\":\"QLineEdit\",\"table_scenarios\":");
    output.write(QJsonDocument(tableScenarios()).toJson(QJsonDocument::Compact));
    output.write(",\"scenarios\":[\n");
    bool first = true;
    for (const auto &scenario : scenarios()) {
        Probe probe(scenario);
        QJsonArray inputs, states;
        states.append(probe.state());
        for (const auto &operation : scenario.operations) {
            inputs.append(probe.apply(operation));
            states.append(probe.state());
        }
        QJsonObject result{{"name", scenario.name}, {"mode", scenario.mode},
                           {"initial", scenario.initial}, {"states", states}, {"operations", inputs}};
        if (!first) output.write(",\n");
        first = false;
        output.write(QJsonDocument(result).toJson(QJsonDocument::Compact));
    }
    output.write("\n]}\n");
    return 0;
}
