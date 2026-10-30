#include "capture_session_startup_routing.h"
#include "shot_window.h"
#include "shortcut_config.h"
#include "support/isolated_config.h"
#include "support/offscreen_screen_config.h"

#include <QApplication>
#include <QCursor>
#include <QDir>
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
    /** 【截图测试】【工具转移】提供扫码和取色两种停留在选区阶段的工具，尺子会立即进入整屏标注，无参数和返回值。 */
    void followsPointerEntry_data()
    {
        QTest::addColumn<int>("tool");
        QTest::newRow("code-scanner") << static_cast<int>(StartupTool::CodeScanner);
        QTest::newRow("color-picker") << static_cast<int>(StartupTool::ColorPicker);
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
        QSignalSpy sourceActivated(&source, &ShotWindow::selectionActivated);
        QSignalSpy targetActivated(&target, &ShotWindow::selectionActivated);
        QKeyEvent keyPress(QEvent::KeyPress, sequence[0].key(), sequence[0].keyboardModifiers());
        QCoreApplication::sendEvent(&source, &keyPress);
        QCOMPARE(source.startupTool(), StartupTool::None);
        // 3. 【截图测试】【工具转移】尺子转到指针屏幕后直接进入整屏标注，原屏不进入
        if (startupTool == StartupTool::Ruler) {
            QCOMPARE(target.startupTool(), StartupTool::None);
            QCOMPARE(targetActivated.count(), 1);
            QCOMPARE(sourceActivated.count(), 0);
            return;
        }
        QCOMPARE(target.startupTool(), startupTool);
    }

    /** 【截图测试】【录制转移】提供 GIF 和视频两种录制模式及指针初始位置，无参数和返回值。 */
    void routesRegionRecordingWithOptions_data()
    {
        QTest::addColumn<int>("mode");
        QTest::addColumn<bool>("pointerOnTarget");
        QTest::newRow("gif-pointer-entry") << static_cast<int>(markshot::recording::RecordingMode::Gif) << false;
        QTest::newRow("video-pointer-entry") << static_cast<int>(markshot::recording::RecordingMode::Video) << false;
        QTest::newRow("gif-pointer-screen") << static_cast<int>(markshot::recording::RecordingMode::Gif) << true;
        QTest::newRow("video-pointer-screen") << static_cast<int>(markshot::recording::RecordingMode::Video) << true;
    }

    /** 【截图测试】【录制转移】录制配置确认后在错误屏幕进入选区时，工具和配置一起转到指针屏幕（issue #139），无参数和返回值。 */
    void routesRegionRecordingWithOptions()
    {
        QFETCH(int, mode);
        QFETCH(bool, pointerOnTarget);
        const auto recordingMode = static_cast<markshot::recording::RecordingMode>(mode);
        const auto recorderTool = recordingMode == markshot::recording::RecordingMode::Gif
            ? StartupTool::GifRecorder : StartupTool::VideoRecorder;
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

        // 1. 【截图测试】【录制转移】模拟键盘焦点所在覆盖层收到已确认的区域录制配置
        QCursor::setPos(pointerOnTarget ? QPoint(900, 100) : QPoint(100, 100));
        markshot::recording::RecordingOptions options;
        options.mode = recordingMode;
        options.scope = markshot::recording::RecordingScope::Region;
        source.beginRegionRecordingSelection(options);

        // 2. 【截图测试】【录制转移】指针不在焦点屏幕时，由进入事件把工具交给指针所在覆盖层
        if (!pointerOnTarget) {
            QCOMPARE(source.startupTool(), recorderTool);
            QVERIFY(source.startupToolCanFollowPointer());
            QEnterEvent enter(QPointF(20, 20), QPointF(20, 20), QPointF(820, 20));
            QCoreApplication::sendEvent(&target, &enter);
        }

        // 3. 【截图测试】【录制转移】目标覆盖层必须同时持有录制工具和待选区配置，原覆盖层退出录制模式
        QCOMPARE(source.startupTool(), StartupTool::None);
        QVERIFY(!source.startupToolCanFollowPointer());
        QCOMPARE(target.startupTool(), recorderTool);
        QVERIFY(target.startupToolCanFollowPointer());
    }
};

/** 【截图测试】【环境隔离】用两块虚拟屏幕运行真实覆盖层事件测试，argc/argv 为命令行参数，返回测试退出码。 */
int main(int argc, char **argv)
{
    // 1. 【截图测试】【路径回归】保留空格路径，并在非 Windows 系统模拟盘符中的冒号
#if defined(Q_OS_WIN)
    const QString directoryName = QStringLiteral("mark-shot screen-routing-XXXXXX");
#else
    const QString directoryName = QStringLiteral("mark-shot screen-routing:-XXXXXX");
#endif
    QTemporaryDir isolated(QDir(QDir::tempPath()).filePath(directoryName));
    if (!markshot::test::isolateConfig(isolated)) {
        return 1;
    }
    // 2. 【截图测试】【虚拟屏幕】共享配置负责安全传参，并在应用销毁后恢复工作目录
    markshot::test::OffscreenScreenConfig config(isolated,
        R"({"synchronousWindowSystemEvents":true,"screens":[{"name":"primary","x":0,"y":0,"width":800,"height":600},{"name":"secondary","x":800,"y":0,"width":800,"height":600}]})");
    if (!config.isValid()) {
        return 1;
    }
    qputenv("XDG_SESSION_TYPE", "x11");
    qputenv("XDG_CURRENT_DESKTOP", "");
    QApplication app(argc, argv);
    StartupToolScreenRoutingTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "startup_tool_screen_routing_test.moc"
