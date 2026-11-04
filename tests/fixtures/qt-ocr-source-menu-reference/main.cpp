/** 【原文菜单对照】【真实组件】只读链接原版 OcrTextPane，采集实际菜单和正文操作 */
#include "app_config_store.h"
#include "ocr_result_window/ocr_text_pane.h"
#include "support/isolated_config.h"
#include "settings/application_theme.h"
#include "ui/context_menu.h"
#include "ui/i18n.h"

#include <QApplication>
#include <QClipboard>
#include <QContextMenuEvent>
#include <QJsonArray>
#include <QJsonDocument>
#include <QTemporaryDir>
#include <QTextEdit>
#include <QTimer>
#include <QtTest/QTest>
#include <iostream>

/** 【原文菜单对照】【编辑快照】返回 editor 的正文、选择、光标和历史资格，不修改正文 */
static QJsonObject editorState(QTextEdit *editor)
{
    const QTextCursor cursor = editor->textCursor();
    return {{"text", editor->toPlainText()}, {"selected", cursor.selectedText()},
        {"cursor", cursor.position()}, {"anchor", cursor.anchor()},
        {"undo", editor->document()->isUndoAvailable()},
        {"redo", editor->document()->isRedoAvailable()},
        {"clipboard", QApplication::clipboard()->text()}};
}

/** 【原文菜单对照】【菜单快照】返回 menu 的实际动作矩形、禁用状态和快捷键，无业务写入 */
static QJsonObject menuState(markshot::ui::ContextMenu *menu)
{
    QJsonArray rows;
    for (QAction *action : menu->actions()) {
        const QRect rect = menu->actionGeometry(action);
        rows.append(QJsonObject{{"label", action->text()}, {"enabled", action->isEnabled()},
            {"separator", action->isSeparator()},
            {"shortcut", action->shortcut().toString(QKeySequence::NativeText)},
            {"rect", QJsonArray{rect.x(), rect.y(), rect.width(), rect.height()}}});
    }
    return {{"size", QJsonArray{menu->width(), menu->height()}}, {"rows", rows},
        {"active", menu->activeAction() ? menu->actions().indexOf(menu->activeAction()) : -1},
        {"family", menu->font().family()}, {"point_size", menu->font().pointSizeF()}};
}

/** 【原文菜单对照】【场景采集】name 指定真实操作，output 保存菜单图像，返回初始和最终编辑状态 */
static QJsonObject scenario(const QString &name, const QString &output)
{
    markshot::shot::OcrTextPane pane("Source", "");
    pane.resize(500, 300);
    pane.setText(name == "empty" ? "" : "alpha beta\ngamma");
    auto *editor = pane.editor();
    pane.show();
    editor->setFocus();
    QTest::qWait(100);
    QApplication::clipboard()->setText(name == "empty-clipboard" ? "" : "pasted text");
    if (name == "undo" || name == "redo") {
        editor->moveCursor(QTextCursor::End);
        editor->insertPlainText("!");
        if (name == "redo") editor->undo();
    }
    if (name != "unselected" && name != "empty" && name != "keyboard" && name != "menu-key") {
        QTextCursor cursor = editor->textCursor();
        cursor.setPosition(0);
        cursor.setPosition(5, QTextCursor::KeepAnchor);
        editor->setTextCursor(cursor);
    }
    const QJsonObject before = editorState(editor);
    if (name == "readonly") editor->setReadOnly(true);
    QJsonObject result{{"name", name}, {"before", before}};
    QTimer timer;
    timer.setSingleShot(true);
    bool opened = false;
    QObject::connect(&timer, &QTimer::timeout, [&] {
        auto *menu = qobject_cast<markshot::ui::ContextMenu *>(QApplication::activePopupWidget());
        if (!menu) return;
        opened = true;
        result["menu"] = menuState(menu);
        result["during"] = editorState(editor);
        if (name == "layout") menu->grab().save(output + "/qt-ocr-source-menu.png");
        QJsonArray states;
        if (name == "navigation") {
            for (int code : {Qt::Key_Down, Qt::Key_Down, Qt::Key_Up, Qt::Key_Home, Qt::Key_End, Qt::Key_Tab}) {
                QTest::keyClick(menu, Qt::Key(code));
                states.append(menuState(menu));
            }
        } else if (name == "mnemonic") {
            for (int code : {Qt::Key_C, Qt::Key_C, Qt::Key_P}) {
                QTest::keyClick(menu, Qt::Key(code));
                states.append(menuState(menu));
            }
        } else if (name == "shortcut") {
            QTest::keyClick(menu, Qt::Key_X, Qt::ControlModifier);
            states.append(menuState(menu));
        } else if (name == "outside") {
            QTest::mousePress(menu, Qt::LeftButton, {}, QPoint(-10, -10));
        } else {
            const QString actionName = name == "copy" || name == "right-copy" || name == "press-copy" ? "Copy"
                : name == "cut" ? "Cut" : name == "paste" ? "Paste" : name == "delete" ? "Delete"
                : name == "select-all" ? "Select All" : name == "undo" ? "Undo" : name == "redo" ? "Redo" : "";
            for (QAction *action : menu->actions()) {
                if (action->text() != actionName) continue;
                const QPoint point = menu->actionGeometry(action).center();
                const auto button = name == "right-copy" ? Qt::RightButton : Qt::LeftButton;
                QTest::mousePress(menu, button, {}, point);
                result["after_press"] = editorState(editor);
                if (name != "press-copy") QTest::mouseRelease(menu, button, {}, point);
                break;
            }
        }
        result["states"] = states;
        if (menu->isVisible()) menu->dismiss();
    });
    timer.start(60);
    const QPoint point = name == "selection-outside" || name == "unselected" ? QPoint(150, 40) : QPoint(10, 10);
    if (name == "keyboard" || name == "menu-key") {
        QTest::keyClick(editor, name == "keyboard" ? Qt::Key_F10 : Qt::Key_Menu,
            name == "keyboard" ? Qt::ShiftModifier : Qt::NoModifier);
        QTest::qWait(100);
    } else {
        QTest::mousePress(editor->viewport(), Qt::RightButton, {}, point);
        QContextMenuEvent event(QContextMenuEvent::Mouse, point, editor->viewport()->mapToGlobal(point));
        QApplication::sendEvent(editor->viewport(), &event);
        QTest::mouseRelease(editor->viewport(), Qt::RightButton, {}, point);
    }
    result["opened"] = opened;
    result["after"] = editorState(editor);
    return result;
}

/** 【原文菜单对照】【入口】argc、argv 指定证据目录，返回退出状态；原版配置、源码和构建文件保持不变 */
int main(int argc, char **argv)
{
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 2;
    for (const char *name : {"DISPLAY", "WAYLAND_DISPLAY", "NIRI_SOCKET", "SWAYSOCK", "XDG_SESSION_TYPE", "DBUS_SESSION_BUS_ADDRESS"}) qunsetenv(name);
    QApplication app(argc, argv);
    markshot::i18n::setLanguage(markshot::i18n::Language::English);
    if (!markshot::writeAppConfigRoot({{"ui", QJsonObject{{"theme", "dark"}}}})) return 3;
    markshot::settings::applyConfiguredApplicationTheme();
    const QString output = argc > 1 ? QString::fromLocal8Bit(argv[1]) : home.path();
    QDir().mkpath(output);
    QJsonArray scenarios;
    for (const QString &name : {"layout", "empty", "empty-clipboard", "unselected", "selection-outside", "readonly", "undo", "redo", "copy", "cut", "paste", "delete", "select-all", "press-copy", "right-copy", "outside", "navigation", "mnemonic", "shortcut", "keyboard", "menu-key"}) {
        scenarios.append(scenario(name, output));
    }
    std::cout << QJsonDocument(QJsonObject{{"qt_version", QT_VERSION_STR}, {"scenarios", scenarios}}).toJson().toStdString();
}
