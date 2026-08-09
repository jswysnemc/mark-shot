#include "window_detection_session.h"

#include <QtTest/QtTest>

class WindowDetectionSessionTest final : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证已支持的 Wayland 合成器会选择匹配的内置脚本。
     * @return 无返回值。
     */
    void knownWaylandSessionUsesMatchingBundledCommand()
    {
        QProcessEnvironment environment;
        environment.insert(QStringLiteral("XDG_SESSION_TYPE"), QStringLiteral("wayland"));
        environment.insert(QStringLiteral("XDG_CURRENT_DESKTOP"), QStringLiteral("GNOME"));

        const markshot::window_detection::Session session =
            markshot::window_detection::detectSession(environment);
        QVERIFY(session.wayland);
        QCOMPARE(session.compositor, QStringLiteral("gnome"));
        QVERIFY(markshot::window_detection::commandMatchesSession(
            QStringLiteral("mark-shot-window-detection-gnome"), session));
        QVERIFY(!markshot::window_detection::commandMatchesSession(
            QStringLiteral("mark-shot-window-detection-niri"), session));
    }

    /**
     * 验证未知 Wayland 合成器不会继续执行已有的 niri 内置脚本。
     * @return 无返回值。
     */
    void unknownWaylandSessionUsesPlatformFallback()
    {
        QProcessEnvironment environment;
        environment.insert(QStringLiteral("XDG_SESSION_TYPE"), QStringLiteral("wayland"));
        environment.insert(QStringLiteral("XDG_CURRENT_DESKTOP"), QStringLiteral("sway"));

        const markshot::window_detection::Session session =
            markshot::window_detection::detectSession(environment);
        QVERIFY(session.wayland);
        QVERIFY(session.compositor.isEmpty());
        QVERIFY(!markshot::window_detection::commandMatchesSession(
            QStringLiteral("mark-shot-window-detection-niri"), session));
        QVERIFY(markshot::window_detection::defaultCommand(session).isEmpty());
    }

    /**
     * 验证自定义命令即使名称带有内置前缀也不会被自动替换。
     * @return 无返回值。
     */
    void customCommandsAreAlwaysPreserved()
    {
        const markshot::window_detection::Session session{true, QStringLiteral("gnome")};
        QVERIFY(markshot::window_detection::commandMatchesSession(
            QStringLiteral("mark-shot-window-detection-custom"), session));
        QVERIFY(markshot::window_detection::commandMatchesSession(
            QStringLiteral("/opt/tools/mark-shot-window-detection-niri"), session));
    }
};

QTEST_APPLESS_MAIN(WindowDetectionSessionTest)

#include "window_detection_session_test.moc"
