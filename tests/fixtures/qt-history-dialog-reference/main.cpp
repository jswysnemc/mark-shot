/** 【历史对话框对照】【真实窗口】只读链接原版对象，以真实鼠标和按键取得清空模态行为 */
#include "app_config_store.h"
#include "capture_history/history_window.h"
#include "support/isolated_config.h"
#include "ui/i18n.h"

#include <QApplication>
#include <QDialog>
#include <QEventLoop>
#include <QJsonArray>
#include <QJsonDocument>
#include <QKeyEvent>
#include <QLabel>
#include <QListWidget>
#include <QMouseEvent>
#include <QPushButton>
#include <QTemporaryDir>
#include <QTimer>
#include <QtTest/QTest>
#include <iostream>

class CaptureHistoryWindowTest final {
public:
    /** 【历史对话框对照】【等待】处理窗口布局及延迟销毁，无参数、无返回值 */
    static void settle()
    {
        QEventLoop loop;
        QTimer::singleShot(100, &loop, &QEventLoop::quit);
        loop.exec();
    }

    /** 【历史对话框对照】【鼠标】向 widget 的 point 投递 button，release 控制是否包含释放，无返回值 */
    static void click(QWidget *widget, QPoint point, Qt::MouseButton button = Qt::LeftButton, bool release = true)
    {
        QTest::mousePress(widget, button, {}, point);
        if (release) QTest::mouseRelease(widget, button, {}, point);
        settle();
    }

    /** 【历史对话框对照】【键盘】向 dialog 当前焦点发送 code、mods 和 type，完整默认按下释放，无返回值 */
    static void key(QDialog *dialog, int code, Qt::KeyboardModifiers mods = {}, QEvent::Type type = QEvent::None)
    {
        QWidget *target = dialog->focusWidget() ? dialog->focusWidget() : dialog;
        if (type == QEvent::None || type == QEvent::KeyPress) {
            QKeyEvent press(QEvent::KeyPress, code, mods);
            QApplication::sendEvent(target, &press);
        }
        if (type == QEvent::None || type == QEvent::KeyRelease) {
            QKeyEvent release(QEvent::KeyRelease, code, mods);
            QApplication::sendEvent(target, &release);
        }
        settle();
    }

    /** 【历史对话框对照】【矩形】返回 widget 相对于 dialog 的实际矩形，不设置或猜测尺寸 */
    static QJsonArray rect(QWidget *widget, QWidget *dialog)
    {
        const auto point = widget->mapTo(dialog, QPoint());
        return {point.x(), point.y(), widget->width(), widget->height()};
    }

    /** 【历史对话框对照】【快照】返回 dialog 的布局、焦点、默认按钮与模态属性，无业务写入 */
    static QJsonObject snapshot(QDialog *dialog)
    {
        QJsonArray buttons, labels;
        for (auto *button : dialog->findChildren<QPushButton *>()) {
            buttons.append(QJsonObject{{"text", button->text()}, {"rect", rect(button, dialog)},
                {"default", button->isDefault()}, {"auto_default", button->autoDefault()},
                {"focused", dialog->focusWidget() == button}, {"down", button->isDown()},
                {"point_size", button->font().pointSizeF()}, {"font_family", button->font().family()}});
        }
        for (auto *label : dialog->findChildren<QLabel *>()) {
            labels.append(QJsonObject{{"text", label->text()}, {"rect", rect(label, dialog)},
                {"wrap", label->wordWrap()}, {"point_size", label->font().pointSizeF()}});
        }
        return {{"size", QJsonArray{dialog->width(), dialog->height()}},
            {"minimum", QJsonArray{dialog->minimumWidth(), dialog->minimumHeight()}},
            {"title", dialog->windowTitle()}, {"modality", int(dialog->windowModality())},
            {"modal", dialog->isModal()}, {"visible", dialog->isVisible()}, {"result", dialog->result()},
            {"buttons", buttons}, {"labels", labels}, {"font_family", dialog->font().family()},
            {"point_size", dialog->font().pointSizeF()}, {"color", dialog->palette().color(QPalette::Window).name()}};
    }

    /** 【历史对话框对照】【场景】在私有配置中构造 name 场景，output 保存真实画面，返回状态与最终记录数量 */
    static QJsonObject scenario(const QString &name, const QString &output)
    {
        markshot::history::HistoryStore store;
        QString error;
        if (!store.clear(&error)) throw std::runtime_error(error.toStdString());
        QImage image(QSize(400, 225), QImage::Format_ARGB32);
        image.fill(QColor(10, 80, 170, 91));
        if (!store.append(image)) throw std::runtime_error("cannot create private history");
        markshot::history::HistoryWindow window;
        window.setAttribute(Qt::WA_DeleteOnClose, false);
        window.show();
        settle();
        click(window.m_clear, window.m_clear->rect().center());
        auto *dialog = window.findChild<QDialog *>("historyConfirmDialog");
        if (!dialog) throw std::runtime_error("the actual clear button did not open a dialog");
        dialog->setAttribute(Qt::WA_DeleteOnClose, false);
        int accepted = 0, rejected = 0;
        QObject::connect(dialog, &QDialog::accepted, [&accepted] { ++accepted; });
        QObject::connect(dialog, &QDialog::rejected, [&rejected] { ++rejected; });
        const auto initial = snapshot(dialog);
        QJsonArray states;
        auto *cancel = dialog->findChildren<QPushButton *>().first();
        auto *clear = dialog->findChildren<QPushButton *>().last();
        if (name == "layout") {
            dialog->grab().save(output + "/qt-history-clear-dialog.png");
            for (int step = 0; step < 4; ++step) {
                key(dialog, Qt::Key_Tab);
                states.append(snapshot(dialog));
            }
        } else if (name == "default-enter") key(dialog, Qt::Key_Return);
        else if (name == "escape") key(dialog, Qt::Key_Escape);
        else if (name == "tab-enter") { key(dialog, Qt::Key_Tab); key(dialog, Qt::Key_Return); }
        else if (name == "right-enter") { key(dialog, Qt::Key_Right); states.append(snapshot(dialog)); key(dialog, Qt::Key_Return); }
        else if (name == "up-enter" || name == "down-enter") { key(dialog, name == "up-enter" ? Qt::Key_Up : Qt::Key_Down); key(dialog, Qt::Key_Return); }
        else if (name == "ctrl-enter") key(dialog, Qt::Key_Return, Qt::ControlModifier);
        else if (name == "shift-enter") key(dialog, Qt::Key_Return, Qt::ShiftModifier);
        else if (name == "alt-enter") key(dialog, Qt::Key_Return, Qt::AltModifier);
        else if (name == "ctrl-escape") key(dialog, Qt::Key_Escape, Qt::ControlModifier);
        else if (name == "shift-escape") key(dialog, Qt::Key_Escape, Qt::ShiftModifier);
        else if (name == "ctrl-tab-enter") { key(dialog, Qt::Key_Tab, Qt::ControlModifier); key(dialog, Qt::Key_Return); }
        else if (name == "tab-ctrl-enter" || name == "tab-alt-enter" || name == "tab-shift-enter") {
            key(dialog, Qt::Key_Tab);
            key(dialog, Qt::Key_Return, name == "tab-ctrl-enter" ? Qt::ControlModifier : name == "tab-alt-enter" ? Qt::AltModifier : Qt::ShiftModifier);
        }
        else if (name == "logo-enter") key(dialog, Qt::Key_Return, Qt::MetaModifier);
        else if (name == "logo-escape") key(dialog, Qt::Key_Escape, Qt::MetaModifier);
        else if (name == "shift-tab-enter") { key(dialog, Qt::Key_Tab, Qt::ShiftModifier); key(dialog, Qt::Key_Return); }
        else if (name == "right-clear") click(clear, clear->rect().center(), Qt::RightButton);
        else if (name == "right-clear-enter") { click(clear, clear->rect().center(), Qt::RightButton); key(dialog, Qt::Key_Return); }
        else if (name == "left-clear-press") click(clear, clear->rect().center(), Qt::LeftButton, false);
        else if (name == "space-clear" || name == "ctrl-space-clear" || name == "shift-space-clear" || name == "alt-space-clear") {
            key(dialog, Qt::Key_Tab);
            const Qt::KeyboardModifiers mods = name == "ctrl-space-clear" ? Qt::ControlModifier : name == "shift-space-clear" ? Qt::ShiftModifier : name == "alt-space-clear" ? Qt::AltModifier : Qt::NoModifier;
            key(dialog, Qt::Key_Space, mods, QEvent::KeyPress);
            states.append(snapshot(dialog));
            key(dialog, Qt::Key_Space, mods, QEvent::KeyRelease);
        } else if (name == "left-clear-release-outside") {
            click(clear, clear->rect().center(), Qt::LeftButton, false);
            QTest::mouseRelease(clear, Qt::LeftButton, {}, QPoint(-10, -10));
            settle();
        } else if (name == "left-clear") click(clear, clear->rect().center());
        else if (name == "left-cancel") click(cancel, cancel->rect().center());
        else if (name == "wm-close") { dialog->close(); settle(); }
        const auto final = snapshot(dialog);
        window.refresh();
        settle();
        QJsonObject result{{"name", name}, {"initial", initial}, {"states", states}, {"final", final},
            {"accepted", accepted}, {"rejected", rejected}, {"remaining", window.m_list->count()}};
        dialog->reject();
        delete dialog;
        return result;
    }
};

/** 【历史对话框对照】【入口】argc、argv 指定证据目录，返回执行状态；原版配置和源码均保持不变 */
int main(int argc, char **argv)
{
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 2;
    for (const char *name : {"DISPLAY", "WAYLAND_DISPLAY", "NIRI_SOCKET", "SWAYSOCK", "XDG_SESSION_TYPE", "DBUS_SESSION_BUS_ADDRESS"}) qunsetenv(name);
    QApplication app(argc, argv);
    markshot::i18n::setLanguage(markshot::i18n::Language::English);
    if (!markshot::writeAppConfigRoot({{"ui", QJsonObject{{"theme", "dark"}}}})) return 3;
    const QString output = argc > 1 ? QString::fromLocal8Bit(argv[1]) : home.path();
    QDir().mkpath(output);
    QJsonArray scenarios;
    for (const QString &name : {"layout", "default-enter", "escape", "tab-enter", "right-enter", "up-enter", "down-enter", "ctrl-enter", "shift-enter", "alt-enter", "ctrl-escape", "shift-escape", "ctrl-tab-enter", "tab-ctrl-enter", "tab-alt-enter", "tab-shift-enter", "logo-enter", "logo-escape", "shift-tab-enter", "right-clear", "right-clear-enter", "left-clear-press", "space-clear", "ctrl-space-clear", "shift-space-clear", "alt-space-clear", "left-clear-release-outside", "left-clear", "left-cancel", "wm-close"}) {
        scenarios.append(CaptureHistoryWindowTest::scenario(name, output));
    }
    std::cout << QJsonDocument(QJsonObject{{"qt_version", QT_VERSION_STR}, {"scenarios", scenarios}}).toJson().toStdString();
}
