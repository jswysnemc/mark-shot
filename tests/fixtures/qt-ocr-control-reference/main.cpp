/** 【结果按钮对照】【真实控件】只读链接原版结果窗口，记录按钮与页签各自的事件时机 */
#include "app_config_store.h"
#include "ocr_result_window/ocr_result_window.h"
#include "support/isolated_config.h"
#include "ui/i18n.h"

#include <QApplication>
#include <QClipboard>
#include <QJsonArray>
#include <QJsonDocument>
#include <QPushButton>
#include <QTabBar>
#include <QTemporaryDir>
#include <QTextEdit>
#include <QtTest/QTest>
#include <iostream>
#include <stdexcept>

/** 【结果按钮对照】【编辑快照】按 window、button、clicks 返回实际正文、焦点、按钮及页面状态，无副作用 */
static QJsonObject snapshot(QWidget *window, QPushButton *button, int clicks)
{
    auto *editor = window->findChild<QTextEdit *>("ocrEditor");
    auto *tabs = window->findChild<QTabBar *>("ocrViewTabs");
    QWidget *focus = QApplication::focusWidget();
    return {{"text", editor->toPlainText()}, {"selected", editor->textCursor().selectedText()},
        {"clipboard", QApplication::clipboard()->text()}, {"clicks", clicks},
        {"focus", focus ? focus->objectName() : ""}, {"down", button->isDown()},
        {"enabled", button->isEnabled()}, {"visible", window->isVisible()},
        {"button_visible", button->isVisible()}, {"tab", tabs->currentIndex()},
        {"auto_default", button->autoDefault()}, {"default", button->isDefault()}};
}

/** 【结果按钮对照】【案例执行】name 指定真实鼠标或键盘案例，返回各事件后的窗口状态，不调用识别或翻译 */
static QJsonObject scenario(const QString &name)
{
    QImage image(100, 60, QImage::Format_ARGB32);
    image.fill(Qt::darkCyan);
    markshot::shot::OcrResultWindow window(name.startsWith("disabled") ? "  " : "alpha beta\ngamma", nullptr, image);
    window.setAttribute(Qt::WA_DeleteOnClose, false);
    window.show();
    window.activateWindow();
    QTest::qWait(80);
    auto *editor = window.findChild<QTextEdit *>("ocrEditor");
    if (name.startsWith("undo")) {
        editor->moveCursor(QTextCursor::End);
        editor->insertPlainText("!");
        QTest::qWait(30);
    }
    auto *source = window.findChild<QWidget *>("ocrSourcePane");
    auto *title = window.findChild<QWidget *>("ocrTitleBar");
    if (!source || !title) throw std::runtime_error("missing original OCR control owner");
    auto *button = name.startsWith("undo") ? source->findChild<QPushButton *>("ocrUndoButton")
        : title->findChild<QPushButton *>(name.startsWith("close") ? "ocrCloseButton" : "ocrCopyButton");
    auto *tabs = window.findChild<QTabBar *>("ocrViewTabs");
    if (!button || !editor || !tabs) throw std::runtime_error("missing original OCR control");
    if (!button->isVisible()) throw std::runtime_error("the reference targeted a hidden OCR control");
    int clicks = 0;
    QObject::connect(button, &QPushButton::clicked, [&] { ++clicks; });
    QApplication::clipboard()->setText("clipboard sentinel");
    QJsonArray states;
    const auto record = [&] { states.append(snapshot(&window, button, clicks)); };
    record();
    if (name == "layout") {
        QJsonArray controls;
        for (auto *control : window.findChildren<QPushButton *>()) {
            const QPoint point = control->mapTo(&window, QPoint());
            controls.append(QJsonObject{{"name", control->objectName()}, {"visible", control->isVisible()},
                {"rect", QJsonArray{point.x(), point.y(), control->width(), control->height()}},
                {"focus_policy", int(control->focusPolicy())}});
        }
        return {{"name", name}, {"states", states}, {"controls", controls}};
    }
    if (name.startsWith("tab")) {
        const auto mouse = name == "tab-right" ? Qt::RightButton : Qt::LeftButton;
        QTest::mousePress(tabs, mouse, {}, tabs->tabRect(1).center());
        record();
        QTest::mouseRelease(tabs, mouse, {}, tabs->tabRect(1).center());
        record();
        return {{"name", name}, {"states", states}};
    }
    if (name.contains("space") || (name.contains("return") && !name.contains("return-inside")) || name.contains("escape")) {
        button->setFocus(Qt::OtherFocusReason);
        Qt::KeyboardModifiers mods;
        if (name.contains("ctrl")) mods = Qt::ControlModifier;
        if (name.contains("shift")) mods = Qt::ShiftModifier;
        if (name.contains("alt")) mods = Qt::AltModifier;
        if (name.contains("logo")) mods = Qt::MetaModifier;
        const auto key = name.contains("space") ? Qt::Key_Space : name.contains("return") ? Qt::Key_Return : Qt::Key_Escape;
        QTest::keyPress(button, key, mods);
        record();
        if (name.contains("focus-change")) editor->setFocus(Qt::OtherFocusReason);
        QTest::keyRelease(button, key, mods);
        record();
    } else {
        const auto mouse = name.contains("right") && !name.contains("left-right") ? Qt::RightButton
            : name.contains("middle") ? Qt::MiddleButton : Qt::LeftButton;
        QTest::mousePress(button, mouse, {}, button->rect().center());
        record();
        if (name.contains("left-right")) {
            QTest::mousePress(button, Qt::RightButton, {}, button->rect().center());
            QTest::mouseRelease(button, Qt::RightButton, {}, button->rect().center());
            record();
        }
        if (name.contains("focus-change")) editor->setFocus(Qt::OtherFocusReason);
        if (name.contains("outside")) QTest::mouseMove(button, QPoint(-10, -10));
        if (name.contains("return-inside")) {
            QTest::mouseMove(button, QPoint(-10, -10));
            record();
            QTest::mouseMove(button, button->rect().center());
            record();
        }
        QTest::mouseRelease(button, mouse, {}, name.contains("outside") ? QPoint(-10, -10) : button->rect().center());
        record();
    }
    return {{"name", name}, {"states", states}};
}

/** 【结果按钮对照】【入口】argc、argv 为标准程序参数，返回采集结果；源码、配置和显示服务保持隔离 */
int main(int argc, char **argv)
{
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 2;
    for (const char *name : {"DISPLAY", "WAYLAND_DISPLAY", "NIRI_SOCKET", "SWAYSOCK", "XDG_SESSION_TYPE", "DBUS_SESSION_BUS_ADDRESS"}) qunsetenv(name);
    QApplication app(argc, argv);
    markshot::i18n::setLanguage(markshot::i18n::Language::English);
    if (!markshot::writeAppConfigRoot({{"ui", QJsonObject{{"theme", "dark"}}}})) return 3;
    QJsonArray scenarios;
    for (const QString &name : {"layout", "copy-left", "copy-right", "copy-middle", "copy-left-outside",
        "copy-left-return-inside", "copy-left-right", "copy-left-focus-change", "copy-space", "copy-space-focus-change",
        "copy-ctrl-space", "copy-shift-space", "copy-alt-space", "copy-logo-space", "copy-return", "copy-shift-return",
        "copy-alt-return", "copy-logo-return", "copy-escape", "copy-ctrl-escape", "copy-shift-escape", "copy-alt-escape",
        "copy-logo-escape", "undo-left", "undo-right", "undo-space", "undo-return", "close-left", "close-right", "close-space",
        "close-return", "disabled-left", "disabled-right", "disabled-space", "tab-left", "tab-right"}) {
        scenarios.append(scenario(name));
    }
    std::cout << QJsonDocument(QJsonObject{{"qt_version", QT_VERSION_STR}, {"scenarios", scenarios}}).toJson().toStdString();
}
