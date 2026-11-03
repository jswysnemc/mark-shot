#include "probe.h"

#include <QApplication>
#include <QInputMethodEvent>
#include <QLineEdit>
#include <QShortcut>
#include <QTableWidget>
#include <QTest>

/// 【窗口输入法对照】【表格快照】返回 table 的持久单元格、选择和编辑器状态，不改变控件
static QJsonObject snapshot(QTableWidget &table)
{
    auto *editor = table.findChild<QLineEdit *>();
    const bool editing = editor && editor->isVisible();
    const QVariant surrounding = table.inputMethodQuery(Qt::ImSurroundingText);
    return {{"cell", table.item(0, 0)->text()}, {"row", table.currentRow()},
            {"column", table.currentColumn()}, {"editing", editing},
            {"editor", editing ? editor->text() : QString()},
            {"selected", editing ? editor->selectedText() : QString()},
            {"enabled", table.inputMethodQuery(Qt::ImEnabled).toBool()},
            {"surrounding", surrounding.toString()},
            {"cursor", table.inputMethodQuery(Qt::ImCursorPosition).toInt()},
            {"anchor", table.inputMethodQuery(Qt::ImAnchorPosition).toInt()}};
}

QJsonArray tableScenarios()
{
    QJsonArray result;
    for (const QString &kind : {QString("return"), QString("shift-return"), QString("tab"),
                               QString("shift-tab"), QString("input-method"), QString("escape"), QString("enter-selection")}) {
        QTableWidget table(2, 2);
        for (int row = 0; row < 2; ++row) {
            for (int column = 0; column < 2; ++column) {
                table.setItem(row, column, new QTableWidgetItem(row == 0 && column == 0 ? "cell" : "other"));
            }
        }
        table.resize(360, 200);
        table.setCurrentCell(0, 0);
        bool rootEscape = false;
        QShortcut escape(QKeySequence(Qt::Key_Escape), &table);
        QObject::connect(&escape, &QShortcut::activated, [&rootEscape] { rootEscape = true; });
        table.show();
        table.setFocus();
        QApplication::processEvents();
        QJsonArray states{snapshot(table)};
        if (kind == "enter-selection") {
            QTest::keyClick(&table, Qt::Key_Return);
        } else if (kind == "input-method") {
            QInputMethodEvent preedit("ni", {{QInputMethodEvent::Cursor, 2, 1, QVariant()}});
            QApplication::sendEvent(&table, &preedit);
            QApplication::processEvents();
            states.append(snapshot(table));
            QInputMethodEvent commit;
            commit.setCommitString(QString::fromUtf8("你"));
            QApplication::sendEvent(QApplication::focusWidget(), &commit);
            QApplication::processEvents();
            states.append(snapshot(table));
            QTest::keyClick(QApplication::focusWidget(), Qt::Key_Return);
        } else {
            QTest::keyClick(&table, Qt::Key_F2);
            QApplication::processEvents();
            states.append(snapshot(table));
            auto *editor = table.findChild<QLineEdit *>();
            if (!editor || !editor->isVisible()) return {};
            QTest::keyClicks(editor, "X");
            QApplication::processEvents();
            states.append(snapshot(table));
            if (kind == "return") QTest::keyClick(editor, Qt::Key_Return);
            else if (kind == "shift-return") QTest::keyClick(editor, Qt::Key_Return, Qt::ShiftModifier);
            else if (kind == "tab") QTest::keyClick(editor, Qt::Key_Tab);
            else if (kind == "shift-tab") QTest::keyClick(editor, Qt::Key_Backtab, Qt::ShiftModifier);
            else QTest::keyClick(editor, Qt::Key_Escape);
        }
        QApplication::processEvents();
        states.append(snapshot(table));
        result.append(QJsonObject{{"kind", kind}, {"states", states}, {"root_escape", rootEscape}});
    }
    return result;
}
