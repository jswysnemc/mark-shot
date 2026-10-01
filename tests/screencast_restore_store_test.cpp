#include "portal/screencast_restore_store.h"

#include <QFile>
#include <QFileInfo>
#include <QLockFile>
#include <QTemporaryDir>
#include <QtTest/QtTest>

#include <future>

using markshot::portal::ScreenCastRestoreStore;

class ScreenCastRestoreStoreTest final : public QObject {
    Q_OBJECT

private slots:
    /** @return 无返回值；验证重新创建存储对象后仍能恢复一次性令牌。 */
    void survivesReopeningAndIsConsumedOnce()
    {
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        const QString path = directory.filePath(QStringLiteral("portal/authorizations.json"));
        QString error;
        QVERIFY(ScreenCastRestoreStore(path).save("screen", "first-token", &error));
        QVERIFY(error.isEmpty());
        QCOMPARE(ScreenCastRestoreStore(path).take("screen", &error), QStringLiteral("first-token"));
        QVERIFY(error.isEmpty());
        QVERIFY(ScreenCastRestoreStore(path).take("screen").isEmpty());
    }

    /** @return 无返回值；验证不同授权范围的保存和消费互不影响。 */
    void scopesAreIndependent()
    {
        QTemporaryDir directory;
        ScreenCastRestoreStore store(directory.filePath("tokens.json"));
        QVERIFY(store.save("DP-1", "primary-token"));
        QVERIFY(store.save("HDMI-A-1", "secondary-token"));
        QCOMPARE(store.take("DP-1"), QStringLiteral("primary-token"));
        QCOMPARE(store.take("HDMI-A-1"), QStringLiteral("secondary-token"));
    }

    /** @return 无返回值；验证同时取用时只有一个调用获得令牌。 */
    void concurrentReadersCannotReuseToken()
    {
        QTemporaryDir directory;
        ScreenCastRestoreStore store(directory.filePath("tokens.json"));
        QVERIFY(store.save("screen", "single-use-token"));
        auto first = std::async(std::launch::async, [&store] { return store.take("screen"); });
        auto second = std::async(std::launch::async, [&store] { return store.take("screen"); });
        const QString firstToken = first.get();
        const QString secondToken = second.get();
        QCOMPARE(int(!firstToken.isEmpty()) + int(!secondToken.isEmpty()), 1);
        QCOMPARE(firstToken + secondToken, QStringLiteral("single-use-token"));
    }

    /** @return 无返回值；验证失效通知不会删除另一会话刚保存的新令牌。 */
    void staleInvalidationPreservesReplacement()
    {
        QTemporaryDir directory;
        ScreenCastRestoreStore store(directory.filePath("tokens.json"));
        QVERIFY(store.save("screen", "old-token"));
        QVERIFY(store.save("screen", "replacement-token"));
        QVERIFY(store.forget("screen", "old-token"));
        QCOMPARE(store.take("screen"), QStringLiteral("replacement-token"));
        QVERIFY(store.save("screen", "invalid-token"));
        QVERIFY(store.forget("screen", "invalid-token"));
        QVERIFY(store.take("screen").isEmpty());
    }

    /** @return 无返回值；验证恢复令牌文件仅允许当前用户读写。 */
    void tokenFileIsPrivate()
    {
#ifdef Q_OS_UNIX
        QTemporaryDir directory;
        const QString path = directory.filePath("tokens.json");
        QVERIFY(ScreenCastRestoreStore(path).save("screen", "private-token"));
        const auto permissions = QFileInfo(path).permissions();
        QVERIFY(permissions & QFileDevice::ReadOwner);
        QVERIFY(permissions & QFileDevice::WriteOwner);
        QVERIFY(!(permissions & (QFileDevice::ReadGroup | QFileDevice::WriteGroup | QFileDevice::ExeGroup
                                  | QFileDevice::ReadOther | QFileDevice::WriteOther | QFileDevice::ExeOther)));
#else
        QSKIP("Unix file permissions only apply to Unix platforms");
#endif
    }

    /** @return 无返回值；验证锁冲突时不泄漏未成功消费的令牌，解锁后仍能取用。 */
    void lockedFileKeepsTokenUnconsumed()
    {
        QTemporaryDir directory;
        const QString path = directory.filePath("tokens.json");
        ScreenCastRestoreStore store(path);
        QVERIFY(store.save("screen", "locked-token"));
        QLockFile otherProcess(path + ".lock");
        QVERIFY(otherProcess.tryLock());
        QString error;
        QVERIFY(store.take("screen", &error).isEmpty());
        QVERIFY(!error.isEmpty());
        otherProcess.unlock();
        QCOMPARE(store.take("screen", &error), QStringLiteral("locked-token"));
        QVERIFY(error.isEmpty());
    }

    /** @return 无返回值；验证损坏的缓存可在用户重新授权后恢复。 */
    void corruptFileRecoversOnNewAuthorization()
    {
        QTemporaryDir directory;
        const QString path = directory.filePath("tokens.json");
        QFile file(path);
        QVERIFY(file.open(QIODevice::WriteOnly));
        file.write("{broken");
        file.close();
        ScreenCastRestoreStore store(path);
        QVERIFY(store.take("screen").isEmpty());
        QVERIFY(store.save("screen", "new-authorization"));
        QCOMPARE(store.take("screen"), QStringLiteral("new-authorization"));
    }

    /** @return 无返回值；验证存储路径不可写时返回可诊断错误。 */
    void invalidDirectoryFailsWithoutReturningToken()
    {
        QTemporaryDir directory;
        const QString path = directory.filePath("regular-file");
        QFile file(path);
        QVERIFY(file.open(QIODevice::WriteOnly));
        file.close();
        ScreenCastRestoreStore store(path + "/tokens.json");
        QString error;
        QVERIFY(!store.save("screen", "not-saved", &error));
        QVERIFY(!error.isEmpty());
        QVERIFY(store.take("screen", &error).isEmpty());
        QVERIFY(!error.isEmpty());
    }
};

QTEST_GUILESS_MAIN(ScreenCastRestoreStoreTest)
#include "screencast_restore_store_test.moc"
