#include "ipc/single_instance_ipc.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QLocalSocket>
#include <QTemporaryDir>
#include <QtTest/QtTest>

#if defined(Q_OS_UNIX)
#include <unistd.h>
#endif

class SingleInstanceIpcTest final : public QObject {
    Q_OBJECT

private slots:
    /** @return 无返回值；保存运行目录环境，避免测试修改影响其他用例。 */
    void init()
    {
        m_hadRuntimeDir = qEnvironmentVariableIsSet("XDG_RUNTIME_DIR");
        m_runtimeDir = qgetenv("XDG_RUNTIME_DIR");
    }

    /** @return 无返回值；恢复测试前的运行目录配置。 */
    void cleanup()
    {
        if (m_hadRuntimeDir) {
            qputenv("XDG_RUNTIME_DIR", m_runtimeDir);
        } else {
            qunsetenv("XDG_RUNTIME_DIR");
        }
    }

    /** @return 无返回值；验证 Unix 使用私有运行目录，其他平台保持原名称。 */
    void serverNameUsesRuntimeDirectory()
    {
#if defined(Q_OS_UNIX)
        QTemporaryDir runtime;
        QVERIFY(runtime.isValid());
        qputenv("XDG_RUNTIME_DIR", runtime.path().toUtf8());
        QCOMPARE(markshot::ipc::singleInstanceServerName(),
                 QDir(runtime.path()).filePath(QStringLiteral("mark-shot-single-instance")));
#else
        QCOMPARE(markshot::ipc::singleInstanceServerName(), QStringLiteral("mark-shot-single-instance"));
#endif
    }

    /** @return 无返回值；验证无效运行目录仍回退到当前用户的私有地址。 */
    void invalidRuntimeDirectoryRemainsUserScoped()
    {
#if defined(Q_OS_UNIX)
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        const QString invalidPath = directory.filePath(QStringLiteral("not-a-directory"));
        QFile file(invalidPath);
        QVERIFY(file.open(QIODevice::WriteOnly));
        file.close();
        qputenv("XDG_RUNTIME_DIR", invalidPath.toUtf8());
        const QString name = markshot::ipc::singleInstanceServerName();
        QVERIFY(name != QStringLiteral("mark-shot-single-instance"));
        QVERIFY(name != QDir(invalidPath).filePath(QStringLiteral("mark-shot-single-instance")));
        if (QDir::isAbsolutePath(name)) {
            const QFileInfo parent(QFileInfo(name).absolutePath());
            QCOMPARE(parent.ownerId(), static_cast<uint>(::geteuid()));
            QVERIFY(!(parent.permissions() & (QFileDevice::ReadGroup | QFileDevice::WriteGroup
                                              | QFileDevice::ExeGroup | QFileDevice::ReadOther
                                              | QFileDevice::WriteOther | QFileDevice::ExeOther)));
        } else {
            QCOMPARE(name, QStringLiteral("mark-shot-%1-single-instance").arg(::getuid()));
        }
#else
        QSKIP("UID fallback only applies to Unix sockets");
#endif
    }

    /** @return 无返回值；验证不同运行目录可同时监听且套接字地址不同。 */
    void runtimeDirectoriesHaveIndependentListeners()
    {
#if defined(Q_OS_UNIX)
        QTemporaryDir firstRuntime;
        QTemporaryDir secondRuntime;
        QVERIFY(firstRuntime.isValid());
        QVERIFY(secondRuntime.isValid());
        QString error;
        qputenv("XDG_RUNTIME_DIR", firstRuntime.path().toUtf8());
        auto first = markshot::ipc::listenForSingleInstanceCommands(&error);
        QVERIFY2(first != nullptr, qPrintable(error));
        qputenv("XDG_RUNTIME_DIR", secondRuntime.path().toUtf8());
        auto second = markshot::ipc::listenForSingleInstanceCommands(&error);
        QVERIFY2(second != nullptr, qPrintable(error));
        QVERIFY(first->fullServerName() != second->fullServerName());
#else
        QSKIP("Runtime directories only apply to Unix sockets");
#endif
    }

    /** @return 无返回值；验证新套接字地址仍能接收命令并返回录制状态。 */
    void isolatedSocketHandlesCommands()
    {
        QTemporaryDir runtime;
        QVERIFY(runtime.isValid());
        qputenv("XDG_RUNTIME_DIR", runtime.path().toUtf8());
        QString error;
        auto server = markshot::ipc::listenForSingleInstanceCommands(&error);
        QVERIFY2(server != nullptr, qPrintable(error));
        bool received = false;
        markshot::ipc::installSingleInstanceCommandHandler(server.get(), this,
            [&received](const markshot::ipc::SingleInstanceCommand &command) {
                received = command.recordingStatus;
                markshot::ipc::SingleInstanceResponse response;
                response.handled = received;
                response.recording.active = true;
                response.recording.frameCount = 42;
                return response;
            });
        QLocalSocket client;
        client.connectToServer(markshot::ipc::singleInstanceServerName());
        QVERIFY(client.waitForConnected(1000));
        const QByteArray command = "{\"recordingStatus\":true}\n";
        QCOMPARE(client.write(command), command.size());
        QTRY_VERIFY_WITH_TIMEOUT(client.bytesAvailable() > 0, 1000);
        QVERIFY(received);
        const QJsonObject response = QJsonDocument::fromJson(client.readAll()).object();
        QVERIFY(response.value(QStringLiteral("handled")).toBool());
        const QJsonObject status = response.value(QStringLiteral("recording")).toObject();
        QCOMPARE(status.value(QStringLiteral("frameCount")).toInt(), 42);
        QVERIFY(status.value(QStringLiteral("active")).toBool());
    }

private:
    bool m_hadRuntimeDir = false;
    QByteArray m_runtimeDir;
};

QTEST_GUILESS_MAIN(SingleInstanceIpcTest)
#include "single_instance_ipc_test.moc"
