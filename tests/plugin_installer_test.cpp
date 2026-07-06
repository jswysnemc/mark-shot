#include "marketplace/plugin_installer.h"

#include <QCryptographicHash>
#include <QFile>
#include <QTemporaryDir>
#include <QtTest/QtTest>

class PluginInstallerTest : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证动态库资产可以安装到指定插件目录。
     * @return 无返回值。
     */
    void installsLibraryAsset()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());

        const QString sourcePath = dir.filePath(pluginFileName());
        const QByteArray content = QByteArrayLiteral("plugin-binary");
        QFile source(sourcePath);
        QVERIFY(source.open(QIODevice::WriteOnly));
        QCOMPARE(source.write(content), qint64(content.size()));
        source.close();

        const QString sha256 = QString::fromLatin1(QCryptographicHash::hash(content, QCryptographicHash::Sha256).toHex());
        const markshot::marketplace::PluginInstallResult result =
            markshot::marketplace::installPluginAsset({sourcePath,
                                                       pluginFileName(),
                                                       dir.filePath(QStringLiteral("plugins")),
                                                       sha256});

        QVERIFY2(result.success, qPrintable(result.error));
        QFile installed(result.installedPath);
        QVERIFY(installed.open(QIODevice::ReadOnly));
        QCOMPARE(installed.readAll(), content);
    }

    /**
     * 验证安装器拒绝压缩包资产。
     * @return 无返回值。
     */
    void rejectsArchiveAsset()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());

        const QString sourcePath = dir.filePath(QStringLiteral("plugin.zip"));
        QFile source(sourcePath);
        QVERIFY(source.open(QIODevice::WriteOnly));
        source.write(QByteArrayLiteral("zip"));
        source.close();

        const markshot::marketplace::PluginInstallResult result =
            markshot::marketplace::installPluginAsset({sourcePath,
                                                       QStringLiteral("plugin.zip"),
                                                       dir.filePath(QStringLiteral("plugins")),
                                                       QString()});

        QVERIFY(!result.success);
        QVERIFY(result.error.contains(QStringLiteral("dynamic library")));
    }

    /**
     * 验证安装器拒绝带路径分隔符的目标文件名。
     * @return 无返回值。
     */
    void rejectsPathTraversal()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());

        const QString sourcePath = dir.filePath(pluginFileName());
        QFile source(sourcePath);
        QVERIFY(source.open(QIODevice::WriteOnly));
        source.write(QByteArrayLiteral("plugin-binary"));
        source.close();

        const markshot::marketplace::PluginInstallResult result =
            markshot::marketplace::installPluginAsset({sourcePath,
                                                       QStringLiteral("../") + pluginFileName(),
                                                       dir.filePath(QStringLiteral("plugins")),
                                                       QString()});

        QVERIFY(!result.success);
        QVERIFY(result.error.contains(QStringLiteral("path separators")));
    }

private:
    /**
     * 读取当前平台测试用插件文件名。
     * @return 插件文件名。
     */
    QString pluginFileName() const
    {
#ifdef Q_OS_WIN
        return QStringLiteral("mark-shot-sample.dll");
#elif defined(Q_OS_MACOS)
        return QStringLiteral("libmark-shot-sample.dylib");
#else
        return QStringLiteral("libmark-shot-sample.so");
#endif
    }
};

QTEST_APPLESS_MAIN(PluginInstallerTest)

#include "plugin_installer_test.moc"
