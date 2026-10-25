#include "window_detection_bundled_script.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QTemporaryDir>
#include <QtTest>

class WindowDetectionBundleTest final : public QObject {
    Q_OBJECT

private slots:
    /** 【窗口识别测试】【内置脚本】模拟只下载主程序而没有安装检测脚本，无参数和返回值。 */
    void missingKdeScriptUsesBundledCopy()
    {
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        qputenv("PATH", directory.path().toUtf8());
        const QString resolved = markshot::window_detection::resolveBundledWindowDetectionScript(
            QStringLiteral("mark-shot-window-detection-kde"), {directory.path()});
        QVERIFY2(QFileInfo(resolved).isFile(), qPrintable(resolved));
        QFile script(resolved);
        QVERIFY(script.open(QIODevice::ReadOnly));
        QVERIFY(script.readAll().contains("MARK_SHOT_KDE_WINDOW_DETECTION"));
    }

    /** 【窗口识别测试】【内置脚本】验证 niri 回退同时提供共享 Python 模块，无参数和返回值。 */
    void bundledNiriIncludesSharedModules()
    {
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        qputenv("PATH", directory.path().toUtf8());
        const QString resolved = markshot::window_detection::resolveBundledWindowDetectionScript(
            QStringLiteral("mark-shot-window-detection-niri"), {directory.path()});
        QVERIFY(QFileInfo(resolved).isFile());
        const QDir parent = QFileInfo(resolved).dir();
        QVERIFY(parent.exists(QStringLiteral("lib/mark_shot_window_detection/__init__.py")));
        QVERIFY(parent.exists(QStringLiteral("lib/mark_shot_window_detection/niri_geometry.py")));
        QVERIFY(parent.exists(QStringLiteral("lib/mark_shot_window_detection/niri_dms.py")));
    }

    /** 【窗口识别测试】【内置脚本】保留已安装脚本的优先级，无参数和返回值。 */
    void installedScriptRemainsPreferred()
    {
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        const QString path = directory.filePath(QStringLiteral("mark-shot-window-detection-kde"));
        QFile file(path);
        QVERIFY(file.open(QIODevice::WriteOnly));
        const QByteArray contents("#!/usr/bin/env python3\nprint('installed')\n");
        QCOMPARE(file.write(contents), contents.size());
        file.close();
        QVERIFY(file.setPermissions(QFileDevice::ReadOwner | QFileDevice::WriteOwner | QFileDevice::ExeOwner));
        QCOMPARE(markshot::window_detection::resolveBundledWindowDetectionScript(
                     QStringLiteral("mark-shot-window-detection-kde"), {directory.path()}), path);
    }

    /** 【窗口识别测试】【内置脚本】自定义命令不参与内置脚本回退，无参数和返回值。 */
    void customCommandIsUntouched()
    {
        const QString command = QStringLiteral("/custom/detect-windows --kde");
        QCOMPARE(markshot::window_detection::resolveBundledWindowDetectionScript(command, {}), command);
        QCOMPARE(markshot::window_detection::windowDetectionInvocation(command), command);
    }
};

QTEST_GUILESS_MAIN(WindowDetectionBundleTest)
#include "window_detection_bundle_test.moc"
