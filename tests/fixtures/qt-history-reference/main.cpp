/** 【历史对照】【真实窗口】只读链接原版窗口对象，在临时配置中取得几何和真实 Qt 输入行为 */
#include "app_config_store.h"
#include "capture_history/history_window.h"
#include "support/isolated_config.h"
#include "ui/i18n.h"

#include <QApplication>
#include <QCheckBox>
#include <QClipboard>
#include <QEventLoop>
#include <QJsonArray>
#include <QJsonDocument>
#include <QKeyEvent>
#include <QLabel>
#include <QListWidget>
#include <QMouseEvent>
#include <QMimeData>
#include <QPushButton>
#include <QSplitter>
#include <QTemporaryDir>
#include <QTimer>
#include <iostream>

class CaptureHistoryWindowTest final {
public:
    /** 【历史对照】【矩形记录】返回 widget 在 root 内的逻辑矩形，不修改几何 */
    static QJsonArray geometry(QWidget *widget, QWidget *root)
    {
        const auto point = widget->mapTo(root, QPoint());
        return {point.x(), point.y(), widget->width(), widget->height()};
    }

    /** 【历史对照】【呈现等待】等待实际预览定时与布局处理，无参数和返回值 */
    static void settle()
    {
        QEventLoop loop;
        QTimer::singleShot(100, &loop, &QEventLoop::quit);
        loop.exec();
    }

    /** 【历史对照】【键盘投递】向 window 当前焦点投递 key、mods，保留真实 Qt 路由，无返回值 */
    static void key(QWidget &window, int key, Qt::KeyboardModifiers mods = {})
    {
        QWidget *target = window.focusWidget() ? window.focusWidget() : &window;
        QKeyEvent press(QEvent::KeyPress, key, mods);
        QApplication::sendEvent(target, &press);
        QKeyEvent release(QEvent::KeyRelease, key, mods);
        QApplication::sendEvent(target, &release);
        QCoreApplication::processEvents();
    }

    /** 【历史对照】【鼠标投递】向 widget 的 point 投递 button 按下与释放，不调用业务方法，无返回值 */
    static void click(QWidget *widget, QPoint point, Qt::MouseButton button)
    {
        const QPointF local(point), global(widget->mapToGlobal(point));
        QMouseEvent press(QEvent::MouseButtonPress, local, global, button, button, {});
        QApplication::sendEvent(widget, &press);
        QMouseEvent release(QEvent::MouseButtonRelease, local, global, button, {}, {});
        QApplication::sendEvent(widget, &release);
        settle();
    }

    /** 【历史对照】【窗口快照】返回 window 真实布局、按钮、列表行和字体参数，不根据预期修正结果 */
    static QJsonObject snapshot(markshot::history::HistoryWindow &window)
    {
        QJsonArray actions, rows, focus;
        for (auto *button : window.m_imageActions) {
            actions.append(QJsonObject{{"label", button->property("actionText").toString()},
                {"text", button->text()}, {"rect", geometry(button, &window)},
                {"enabled", button->isEnabled()}, {"point_size", button->font().pointSizeF()}});
        }
        for (int row = 0; row < window.m_list->count(); ++row) {
            const QRect rect = window.m_list->visualItemRect(window.m_list->item(row));
            rows.append(QJsonObject{{"text", window.m_list->item(row)->text()},
                {"rect", QJsonArray{rect.x(), rect.y(), rect.width(), rect.height()}}});
        }
        window.m_list->setFocus();
        for (int step = 0; step < 10; ++step) {
            auto *widget = window.focusWidget();
            QString name = widget ? widget->accessibleName() : QString();
            if (widget == window.m_list) name = "list";
            if (widget == window.m_enabled) name = "enabled";
            focus.append(name);
            key(window, Qt::Key_Tab);
        }
        window.m_list->setFocus();
        auto *splitter = window.findChild<QSplitter *>();
        auto *close = window.findChild<QPushButton *>("historyCloseButton");
        return {{"size", QJsonArray{window.width(), window.height()}},
            {"minimum", QJsonArray{window.minimumWidth(), window.minimumHeight()}},
            {"list", geometry(window.m_list, &window)}, {"preview", geometry(window.m_preview, &window)},
            {"handle", geometry(splitter->handle(1), &window)}, {"close", geometry(close, &window)},
            {"summary", window.m_summary->text()}, {"summary_rect", geometry(window.m_summary, &window)},
            {"enabled", geometry(window.m_enabled, &window)}, {"enabled_text", window.m_enabled->text()},
            {"clear", geometry(window.m_clear, &window)}, {"clear_text", window.m_clear->text()},
            {"actions", actions}, {"rows", rows}, {"focus_sequence", focus},
            {"window_color", window.palette().color(QPalette::Window).name()},
            {"preview_color", window.m_preview->palette().color(QPalette::Base).name()}};
    }

    /** 【历史对照】【补充行为】取得 window 的分隔夹紧、状态布局和按键释放语义，返回真实结果，不调整预期 */
    static QJsonObject details(markshot::history::HistoryWindow &window)
    {
        QJsonArray splits;
        auto *splitter = window.findChild<QSplitter *>();
        for (int requested : {1, 184, 210, 300, 800}) {
            splitter->setSizes({requested, 800 - requested});
            settle();
            splits.append(QJsonObject{{"requested", requested}, {"list", geometry(window.m_list, &window)},
                {"preview", geometry(window.m_preview, &window)}, {"handle", geometry(splitter->handle(1), &window)}});
        }
        splitter->setSizes({210, 566});
        settle();
        window.m_list->setFocus();
        const int before = QApplication::topLevelWidgets().size();
        key(window, Qt::Key_Return);
        const bool enterEdited = QApplication::topLevelWidgets().size() != before;
        auto *copy = window.m_imageActions.first();
        int clicked = 0;
        const auto connection = QObject::connect(copy, &QPushButton::clicked, &window, [&clicked] { ++clicked; });
        QApplication::clipboard()->setText("history sentinel");
        copy->setFocus();
        key(window, Qt::Key_Return);
        const bool enterCopied = QApplication::clipboard()->text() != "history sentinel";
        const bool enterClicked = clicked != 0;
        QKeyEvent press(QEvent::KeyPress, Qt::Key_Space, {});
        QApplication::sendEvent(copy, &press);
        const bool spaceCopiedOnPress = QApplication::clipboard()->text() != "history sentinel";
        const bool spaceClickedOnPress = clicked != 0;
        QKeyEvent release(QEvent::KeyRelease, Qt::Key_Space, {});
        QApplication::sendEvent(copy, &release);
        settle();
        const bool spaceCopiedOnRelease = QApplication::clipboard()->mimeData()->hasFormat("image/png");
        const bool spaceClickedOnRelease = clicked == 1;
        const bool enabled = copy->isEnabled();
        QObject::disconnect(connection);
        window.showStatus("Copied");
        settle();
        return {{"splitters", splits}, {"enter_edited", enterEdited}, {"enter_copied", enterCopied},
            {"copy_enabled", enabled}, {"enter_clicked", enterClicked},
            {"space_clicked_on_press", spaceClickedOnPress}, {"space_clicked_on_release", spaceClickedOnRelease},
            {"space_copied_on_press", spaceCopiedOnPress}, {"space_copied_on_release", spaceCopiedOnRelease},
            {"status", geometry(window.m_status, &window)}, {"status_list", geometry(window.m_list, &window)},
            {"status_enabled", geometry(window.m_enabled, &window)}, {"status_clear", geometry(window.m_clear, &window)},
            {"preview_contents", QJsonArray{window.m_preview->contentsRect().x(), window.m_preview->contentsRect().y(),
                window.m_preview->contentsRect().width(), window.m_preview->contentsRect().height()}},
            {"font_family", copy->font().family()}, {"font_weight", copy->font().weight()},
            {"list_point_size", window.m_list->font().pointSizeF()}, {"enabled_point_size", window.m_enabled->font().pointSizeF()}};
    }

    /** 【历史对照】【场景采集】运行真实历史 window，在 output 保存画面并返回全部场景，无业务配置写入 */
    static QJsonObject run(const QString &output)
    {
        markshot::history::HistoryWindow window;
        window.setAttribute(Qt::WA_DeleteOnClose, false);
        window.show();
        settle();
        const QString initialFocus = window.focusWidget() == window.m_list ? "list"
            : window.focusWidget() ? window.focusWidget()->accessibleName() : QString();
        QJsonArray layouts;
        for (const auto size : {QSize(440, 320), QSize(620, 460), QSize(760, 460), QSize(800, 520)}) {
            window.resize(size);
            settle();
            layouts.append(snapshot(window));
            window.grab().save(output + QString("/qt-history-%1x%2.png").arg(size.width()).arg(size.height()));
        }
        window.m_list->setCurrentRow(0);
        auto *viewport = window.m_list->viewport();
        const QPoint second = window.m_list->visualItemRect(window.m_list->item(1)).center();
        click(viewport, second, Qt::RightButton);
        const int rightSelected = window.m_list->currentRow();
        window.m_list->setFocus();
        key(window, Qt::Key_F, Qt::ControlModifier);
        const bool searchChangedFocus = window.focusWidget() != window.m_list;
        key(window, Qt::Key_Up);
        const int upSelected = window.m_list->currentRow();
        QApplication::clipboard()->setText("history sentinel");
        auto *copy = window.m_imageActions.first();
        click(copy, copy->rect().center(), Qt::RightButton);
        const bool rightCopied = QApplication::clipboard()->text() != "history sentinel";
        const auto extra = details(window);
        return {{"qt_version", QT_VERSION_STR}, {"layouts", layouts}, {"details", extra}, {"initial_focus", initialFocus},
            {"right_click_selected", rightSelected}, {"up_selected", upSelected},
            {"ctrl_f_changed_focus", searchChangedFocus}, {"right_button_copied", rightCopied}};
    }
};

/** 【历史对照】【入口】argc、argv 提供画面目录，返回执行状态；配置和存储仅在临时目录创建 */
int main(int argc, char **argv)
{
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 2;
    // 1. 【历史对照】【会话隔离】离屏探针只使用本进程剪贴板，不继承真实桌面或外部持有者后端
    for (const char *name : {"DISPLAY", "WAYLAND_DISPLAY", "NIRI_SOCKET", "SWAYSOCK", "XDG_SESSION_TYPE", "DBUS_SESSION_BUS_ADDRESS"}) {
        qunsetenv(name);
    }
    QApplication app(argc, argv);
    markshot::i18n::setLanguage(markshot::i18n::Language::English);
    if (!markshot::writeAppConfigRoot({{"ui", QJsonObject{{"theme", "dark"}}}})) return 3;
    markshot::history::HistoryStore store;
    for (const QSize size : {QSize(640, 360), QSize(400, 225)}) {
        QImage image(size, QImage::Format_ARGB32);
        image.fill(QColor(10, 80, 170, 91));
        if (!store.append(image)) return 4;
    }
    const QString output = argc > 1 ? QString::fromLocal8Bit(argv[1]) : home.path();
    QDir().mkpath(output);
    std::cout << QJsonDocument(CaptureHistoryWindowTest::run(output)).toJson().toStdString();
}
