#include "capture_session_startup_routing.h"
#include "shot_window.h"
#include "shortcut_config.h"
#include "support/isolated_config.h"

#include <QApplication>
#include <QCursor>
#include <QEnterEvent>
#include <QFile>
#include <QKeyEvent>
#include <QScreen>
#include <QScopeGuard>
#include <QTemporaryDir>
#include <QtTest>

using StartupTool = markshot::shot::types::StartupTool;

class StartupToolScreenRoutingTest final : public QObject {
    Q_OBJECT

private slots:
    /** 【截图测试】【工具转移】提供扫码、取色和测量三种现有工具，无参数和返回值。 */
    void followsPointerEntry_data()
    {
        QTest::addColumn<int>("tool");
        QTest::newRow("code-scanner") << static_cast<int>(StartupTool::CodeScanner);
        QTest::newRow("color-picker") << static_cast<int>(StartupTool::ColorPicker);
        QTest::newRow("ruler") << static_cast<int>(StartupTool::Ruler);
    }

    /** 【截图测试】【工具转移】通过实际覆盖层事件验证工具跟随指针进入另一屏幕，无参数和返回值。 */
    void followsPointerEntry()
    {
        QFETCH(int, tool);
        const auto startupTool = static_cast<StartupTool>(tool);
        const auto screens = QGuiApplication::screens();
        QCOMPARE(screens.size(), 2);
        QImage image(800, 600, QImage::Format_RGB32);
        image.fill(Qt::white);
        ShotWindow source(image, {}, screens.first()->geometry(), {}, false);
        ShotWindow target(image, {}, screens.last()->geometry(), {}, false);
        source.setCaptureScreen(screens.first());
        target.setCaptureScreen(screens.last());
        const QVector<QPointer<ShotWindow>> windows{&source, &target};
        auto *app = qobject_cast<QApplication *>(QApplication::instance());
        markshot::capture_session::connectStartupToolScreenRouting(app, &source, windows);
        markshot::capture_session::connectStartupToolScreenRouting(app, &target, windows);

        // 1. 【截图测试】【工具转移】在主屏进入工具模式，不依赖真实合成器或全局指针位置
        source.applyRoutedStartupTool(startupTool);
        QCOMPARE(source.startupTool(), startupTool);
        QCOMPARE(target.startupTool(), StartupTool::None);

        // 2. 【截图测试】【工具转移】向副屏覆盖层发送实际进入事件，原屏应退出并转移相同工具
        QEnterEvent enter(QPointF(20, 20), QPointF(20, 20), QPointF(820, 20));
        QCoreApplication::sendEvent(&target, &enter);
        QCOMPARE(target.startupTool(), startupTool);
        QCOMPARE(source.startupTool(), StartupTool::None);
    }

    /** 【截图测试】【工具转移】提供三种指针工具和独立 niri 焦点，无参数和返回值。 */
    void routesInitialToolToPointerScreen_data()
    {
        QTest::addColumn<int>("tool");
        QTest::addColumn<bool>("niriSession");
        QTest::newRow("code-scanner") << static_cast<int>(StartupTool::CodeScanner) << false;
        QTest::newRow("color-picker") << static_cast<int>(StartupTool::ColorPicker) << false;
        QTest::newRow("ruler") << static_cast<int>(StartupTool::Ruler) << false;
        QTest::newRow("niri-pointer-not-focus") << static_cast<int>(StartupTool::ColorPicker) << true;
    }

    /** 【截图测试】【工具转移】在错误屏幕启用工具时立即转移到指针所在屏幕，无参数和返回值。 */
    void routesInitialToolToPointerScreen()
    {
        QFETCH(int, tool);
        QFETCH(bool, niriSession);
        const auto startupTool = static_cast<StartupTool>(tool);
        const auto screens = QGuiApplication::screens();
        QCOMPARE(screens.size(), 2);
        QImage image(800, 600, QImage::Format_RGB32);
        image.fill(Qt::white);
        ShotWindow source(image, {}, screens.first()->geometry(), {}, false);
        ShotWindow target(image, {}, screens.last()->geometry(), {}, false);
        source.setCaptureScreen(screens.first());
        target.setCaptureScreen(screens.last());
        const QVector<QPointer<ShotWindow>> windows{&source, &target};
        auto *app = qobject_cast<QApplication *>(QApplication::instance());
        markshot::capture_session::connectStartupToolScreenRouting(app, &source, windows);
        markshot::capture_session::connectStartupToolScreenRouting(app, &target, windows);

        // 1. 【截图测试】【工具转移】模拟 niri 焦点留在主屏，但指针已经进入副屏
        QTemporaryDir commands;
        QVERIFY(commands.isValid());
        const auto originalPath = qgetenv("PATH");
        const auto originalType = qgetenv("XDG_SESSION_TYPE");
        const auto originalDesktop = qgetenv("XDG_CURRENT_DESKTOP");
        const auto restoreEnvironment = qScopeGuard([&] {
            qputenv("PATH", originalPath);
            qputenv("XDG_SESSION_TYPE", originalType);
            qputenv("XDG_CURRENT_DESKTOP", originalDesktop);
        });
        if (niriSession) {
            QFile niri(commands.filePath(QStringLiteral("niri")));
            QVERIFY(niri.open(QIODevice::WriteOnly));
            const QByteArray contents("#!/bin/sh\nprintf '{\"name\":\"primary\"}'\n");
            QCOMPARE(niri.write(contents), contents.size());
            niri.close();
            QVERIFY(niri.setPermissions(QFileDevice::ReadOwner | QFileDevice::WriteOwner | QFileDevice::ExeOwner));
            qputenv("PATH", commands.path().toUtf8());
            qputenv("XDG_SESSION_TYPE", "wayland");
            qputenv("XDG_CURRENT_DESKTOP", "niri");
        }

        // 2. 【截图测试】【工具转移】发送实际快捷键，工具必须按指针位置而不是键盘焦点转移
        QCursor::setPos(QPoint(900, 100));
        const auto shortcuts = markshot::shortcut::configuredShortcuts({});
        const auto sequence = startupTool == StartupTool::ColorPicker ? shortcuts.startupColorPicker
            : startupTool == StartupTool::Ruler ? shortcuts.startupRuler : shortcuts.startupCodeScanner;
        QVERIFY(!sequence.isEmpty());
        QKeyEvent keyPress(QEvent::KeyPress, sequence[0].key(), sequence[0].keyboardModifiers());
        QCoreApplication::sendEvent(&source, &keyPress);
        QCOMPARE(source.startupTool(), StartupTool::None);
        QCOMPARE(target.startupTool(), startupTool);
    }
};

/** 【截图测试】【环境隔离】用两块虚拟屏幕运行真实覆盖层事件测试，argc/argv 为命令行参数，返回测试退出码。 */
int main(int argc, char **argv)
{
    QTemporaryDir isolated;
    if (!markshot::test::isolateConfig(isolated)) {
        return 1;
    }
    QFile config(isolated.filePath(QStringLiteral("screens.json")));
    if (!config.open(QIODevice::WriteOnly)) {
        return 1;
    }
    config.write(R"({"synchronousWindowSystemEvents":true,"screens":[{"name":"primary","x":0,"y":0,"width":800,"height":600},{"name":"secondary","x":800,"y":0,"width":800,"height":600}]})");
    config.close();
    qputenv("QT_QPA_PLATFORM", QByteArray("offscreen:configfile=") + config.fileName().toUtf8());
    qputenv("XDG_SESSION_TYPE", "x11");
    qputenv("XDG_CURRENT_DESKTOP", "");
    QApplication app(argc, argv);
    StartupToolScreenRoutingTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "startup_tool_screen_routing_test.moc"
