/** 【更多菜单对照】【完整窗口】只读链接原版 OcrResultWindow，采集真实父子菜单及逐层关闭行为 */
#include "app_config_store.h"
#include "ocr_result_window/ocr_result_window.h"
#include "support/isolated_config.h"
#include "ui/context_menu.h"
#include "ui/context_menu_model.h"
#include "ui/i18n.h"
#include <QApplication>
#include <QDir>
#include <QFile>
#include <QFontInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QKeyEvent>
#include <QMouseEvent>
#include <QPushButton>
#include <QTemporaryDir>
#include <QTextEdit>
#include <QtTest/QTest>
#include <iostream>
#include <stdexcept>

/** 【更多菜单对照】【整数矩形】将 value 转为四元数组，返回逻辑像素矩形 */
static QJsonArray rect(const QRect &value) { return {value.x(), value.y(), value.width(), value.height()}; }

/** 【更多菜单对照】【真实菜单】读取 menu 的动作、字体和实际行几何，返回记录，不创建替代组件 */
static QJsonObject menuState(markshot::ui::ContextMenu *menu)
{
    QJsonArray rows;
    const auto actions = menu->actions();
    for (QAction *action : actions) {
        rows.append(QJsonObject{{"label", markshot::ui::contextMenuDisplayText(action->text())},
            {"shortcut", markshot::ui::contextMenuShortcutText(action)}, {"enabled", action->isEnabled()},
            {"submenu", menu->menuForAction(action) != nullptr}, {"rect", rect(menu->actionGeometry(action))}});
    }
    return {{"size", QJsonArray{menu->sizeHint().width(), menu->sizeHint().height()}},
        {"font_family", QFontInfo(menu->font()).family()}, {"pixels", QFontInfo(menu->font()).pixelSize()},
        {"font_height", menu->fontMetrics().height()}, {"rows", rows}};
}

/** 【更多菜单对照】【键盘事件】将 key 交给 menu 的真实事件入口，无返回值，最深菜单负责导航 */
static void key(markshot::ui::ContextMenu *menu, int key)
{
    QKeyEvent event(QEvent::KeyPress, key, Qt::NoModifier);
    QApplication::sendEvent(menu, &event);
    QApplication::processEvents();
}

/** 【更多菜单对照】【鼠标事件】向 menu 的 point 发送 type、button 事件，无返回值，不改写菜单内部状态 */
static void mouse(markshot::ui::ContextMenu *menu, QEvent::Type type, QPoint point, Qt::MouseButton button)
{
    QMouseEvent event(type, QPointF(point), QPointF(menu->mapToGlobal(point)), button,
        type == QEvent::MouseButtonPress ? Qt::MouseButtons(button) : Qt::NoButton, Qt::NoModifier);
    QApplication::sendEvent(menu, &event);
    QApplication::processEvents();
}

/** 【更多菜单对照】【入口】argc、argv 指定输出和私有 Wayland 模式，返回状态；不修改原版源码或用户配置 */
int main(int argc, char **argv)
{
    const bool wayland = argc > 2 && QByteArray(argv[2]) == "--wayland";
    const QByteArray runtime = qgetenv("XDG_RUNTIME_DIR");
    if (wayland && (qgetenv("MARK_SHOT_PRIVATE_QT_REFERENCE") != runtime || !QFile::exists(QString::fromUtf8(runtime) + "/runtime.json"))) return 4;
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 2;
    if (wayland) { qputenv("XDG_RUNTIME_DIR", runtime); qputenv("QT_QPA_PLATFORM", "wayland"); }
    else { for (const char *name : {"DISPLAY", "WAYLAND_DISPLAY", "NIRI_SOCKET", "SWAYSOCK", "XDG_SESSION_TYPE"}) qunsetenv(name); }
    qunsetenv("DBUS_SESSION_BUS_ADDRESS");
    QApplication app(argc, argv);
    QApplication::setDesktopFileName("mark-shot-ocr-result");
    QApplication::setQuitOnLastWindowClosed(false);
    if (!markshot::writeAppConfigRoot({{"ui", QJsonObject{{"theme", "dark"}}}})) return 3;
    markshot::i18n::setLanguage(markshot::i18n::Language::English);
    const QString output = argc > 1 ? QString::fromLocal8Bit(argv[1]) : home.path();
    QDir().mkpath(output);
    QImage image(2, 2, QImage::Format_ARGB32); image.fill(Qt::transparent);
    markshot::shot::OcrResultWindow window("original", nullptr, image, {});
    window.resize(760, 520); window.show(); window.activateWindow(); QTest::qWait(50);
    auto *button = window.findChild<QPushButton *>("ocrMoreButton");
    auto *root = window.findChild<markshot::ui::ContextMenu *>("ocrMoreMenu");
    if (!button || !root || root->actions().size() != 1) return 5;
    auto *child = root->menuForAction(root->actions().first());
    if (!child) return 6;
    // 1. 【更多菜单对照】【真实抓取】Wayland 首次打开由外部私有驱动点击；离屏模式调用原版按钮槽
    if (wayland) {
        std::cout << "ready\n" << std::flush;
        int wait = 0;
        while (!root->isVisible() && wait++ < 400) QTest::qWait(25);
        if (!root->isVisible()) return 7;
    } else { button->click(); QApplication::processEvents(); }
    const QJsonObject rootLayout = menuState(root), childLayout = menuState(child);
    int triggered = 0;
    QObject::connect(root, &markshot::ui::ContextMenu::triggered, [&triggered](QAction *) { ++triggered; });
    QJsonArray states;
    const auto record = [&](const QString &name) {
        states.append(QJsonObject{{"name", name}, {"root_visible", root->isVisible()}, {"child_visible", child->isVisible()},
            {"root_active", root->actions().indexOf(root->activeAction())}, {"child_active", child->actions().indexOf(child->activeAction())}});
    };
    record("root-open");
    key(root, Qt::Key_Left); record("root-left-kept");
    key(root, Qt::Key_Down); record("root-selected");
    key(root, Qt::Key_Right); record("child-open");
    child->grab().save(output + "/child.png"); root->grab().save(output + "/root.png");
    key(root, Qt::Key_Down); key(root, Qt::Key_Return); key(root, Qt::Key_Space); record("disabled-kept");
    key(root, Qt::Key_Left); record("child-left-closed");
    key(root, Qt::Key_Right); key(root, Qt::Key_Escape); record("child-escape-closed");
    key(root, Qt::Key_Escape); record("root-escape-closed");
    // 2. 【更多菜单对照】【鼠标职责】父菜单行按下即展开，释放不执行说明动作
    button->click(); QApplication::processEvents();
    const QPoint point = root->actionGeometry(root->actions().first()).center();
    mouse(root, QEvent::MouseButtonPress, point, Qt::LeftButton); record("parent-press-open");
    mouse(root, QEvent::MouseButtonRelease, point, Qt::LeftButton); record("parent-release-kept");
    key(root, Qt::Key_Escape); root->hide();
    // 3. 【更多菜单对照】【悬停期限】实际事件触发一次性计时，取消悬停不展开子菜单
    button->click(); QApplication::processEvents();
    mouse(root, QEvent::MouseMove, point, Qt::NoButton); QTest::qWait(100); record("hover-before-deadline");
    QTest::qWait(130); record("hover-open");
    key(root, Qt::Key_Left); root->hide(); button->click(); QApplication::processEvents();
    mouse(root, QEvent::MouseMove, point, Qt::NoButton);
    mouse(root, QEvent::MouseMove, QPoint(1, 1), Qt::NoButton); QTest::qWait(230); record("hover-canceled");
    mouse(root, QEvent::MouseMove, point, Qt::NoButton);
    mouse(root, QEvent::MouseMove, QPoint(-20, -20), Qt::NoButton); QTest::qWait(230); record("hover-left-tree");
    root->hide();
    auto *editor = window.findChild<QTextEdit *>("ocrEditor");
    const QJsonObject result{{"platform", QApplication::platformName()}, {"root", rootLayout}, {"child", childLayout}, {"states", states},
        {"triggered", triggered}, {"source", editor ? editor->toPlainText() : QString()}, {"window_visible", window.isVisible()}};
    std::cout << QJsonDocument(result).toJson(QJsonDocument::Compact).toStdString() << std::flush;
}
