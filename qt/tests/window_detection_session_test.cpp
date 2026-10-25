#include "window_detection_session.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QTemporaryDir>
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

    /**
     * 验证内置脚本会改用应用程序目录中的可执行文件，自定义命令保持不变。
     * @return 无返回值。
     */
    void bundledCommandResolvesBesideExecutable()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString path = QDir(dir.path()).filePath(QStringLiteral("mark-shot-window-detection-kde"));
        QFile file(path);
        QVERIFY(file.open(QIODevice::WriteOnly));
        file.write("#!/bin/sh\n");
        file.close();
#if !defined(Q_OS_WIN)
        QVERIFY(QFile::setPermissions(path,
                                      QFileDevice::ReadOwner | QFileDevice::WriteOwner | QFileDevice::ExeOwner));
#endif

        QCOMPARE(markshot::window_detection::resolveBundledCommand(
                     QStringLiteral("mark-shot-window-detection-kde"), {dir.path()}),
                 QFileInfo(path).absoluteFilePath());
        QCOMPARE(markshot::window_detection::resolveBundledCommand(
                     QStringLiteral("mark-shot-window-detection-kde"), {QStringLiteral("/missing")}),
                 QStringLiteral("mark-shot-window-detection-kde"));
        QCOMPARE(markshot::window_detection::resolveBundledCommand(
                     QStringLiteral("/opt/custom-detector"), {dir.path()}),
                 QStringLiteral("/opt/custom-detector"));
        QCOMPARE(markshot::window_detection::quoteShellWord(QStringLiteral("/opt/Mark Shot/bin")),
                 QStringLiteral("'/opt/Mark Shot/bin'"));
    }

    /**
     * 验证内置脚本还会到安装前缀和构建树的 scripts 目录查找。
     * @return 无返回值。
     */
    void bundledCommandSearchIncludesInstallAndBuildLayouts()
    {
        const QStringList directories = markshot::window_detection::bundledCommandSearchDirectories(
            QStringLiteral("/opt/mark-shot/bin"),
            QStringLiteral("/home/user"),
            QStringLiteral("/src/mark-shot"));
        QVERIFY(directories.contains(QStringLiteral("/opt/mark-shot/bin")));
        QVERIFY(directories.contains(QStringLiteral("/opt/mark-shot/scripts")));
        QVERIFY(directories.contains(QStringLiteral("/opt/mark-shot/qt/scripts")));
        QVERIFY(directories.contains(QStringLiteral("/opt/mark-shot/libexec/mark-shot")));
        QCOMPARE(directories.first(), QStringLiteral("/opt/mark-shot/libexec/mark-shot"));
        QVERIFY(directories.contains(QStringLiteral("/usr/bin")));
        QVERIFY(directories.contains(QStringLiteral("/home/user/.local/bin")));
        QVERIFY(directories.contains(QStringLiteral("/src/mark-shot/scripts")));
    }
};

QTEST_APPLESS_MAIN(WindowDetectionSessionTest)

#include "window_detection_session_test.moc"
