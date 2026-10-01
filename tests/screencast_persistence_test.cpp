#include "portal/screencast_persistence.h"

#include <QTemporaryDir>
#include <QtTest/QtTest>

using markshot::portal::ScreenCastPersistence;

namespace {

/** @return 与长截图一致的显示器区域请求。 */
CaptureRequest scrollRequest()
{
    CaptureRequest request;
    request.preferredOutputName = QStringLiteral("DP-1");
    request.sourceGeometry = QRect(100, 100, 800, 600);
    request.preferScreencast = true;
    request.allowInteractivePortal = false;
    request.allowInteractiveScreencastInit = true;
    return request;
}

}  // namespace

class ScreenCastPersistenceTest final : public QObject {
    Q_OBJECT

private slots:
    /** @return 无返回值；验证首次授权请求持久化，随后每次使用门户返回的最新令牌。 */
    void remembersAuthorizationAcrossSessionsAndRotatesToken()
    {
        QTemporaryDir directory;
        const QString path = directory.filePath("tokens.json");
        {
            ScreenCastPersistence first(scrollRequest(), false, path);
            const QVariantMap options = first.prepare(4);
            QCOMPARE(options.value("persist_mode").toUInt(), uint(2));
            QVERIFY(!options.contains("restore_token"));
            first.accept("first-grant");
        }
        {
            ScreenCastPersistence second(scrollRequest(), false, path);
            const QVariantMap options = second.prepare(6);
            QCOMPARE(options.value("restore_token").toString(), QStringLiteral("first-grant"));
            QCOMPARE(options.value("persist_mode").toUInt(), uint(2));
            second.accept("rotated-grant");
        }
        ScreenCastPersistence third(scrollRequest(), false, path);
        QCOMPARE(third.prepare(4).value("restore_token").toString(), QStringLiteral("rotated-grant"));
    }

    /** @return 无返回值；验证同一显示器内改变长截图区域仍复用原授权。 */
    void movingRegionOnSameOutputKeepsAuthorization()
    {
        QTemporaryDir directory;
        const QString path = directory.filePath("tokens.json");
        ScreenCastPersistence first(scrollRequest(), false, path);
        first.prepare(4);
        first.accept("screen-grant");
        CaptureRequest next = scrollRequest();
        next.sourceGeometry = QRect(20, 50, 1024, 700);
        ScreenCastPersistence second(next, false, path);
        QCOMPARE(second.prepare(4).value("restore_token").toString(), QStringLiteral("screen-grant"));
    }

    /** @return 无返回值；验证不同显示器、采集范围、鼠标策略和用途分别管理授权。 */
    void differentCaptureTargetsNeverConsumeOtherTokens()
    {
        QTemporaryDir directory;
        const QString path = directory.filePath("tokens.json");
        ScreenCastPersistence first(scrollRequest(), false, path);
        first.prepare(4);
        first.accept("primary-grant");

        CaptureRequest secondary = scrollRequest();
        secondary.preferredOutputName = QStringLiteral("HDMI-A-1");
        secondary.sourceGeometry.translate(2560, 0);
        QVERIFY(!ScreenCastPersistence(secondary, false, path).prepare(4).contains("restore_token"));
        CaptureRequest allOutputs = scrollRequest();
        allOutputs.allOutputs = true;
        QVERIFY(!ScreenCastPersistence(allOutputs, false, path).prepare(4).contains("restore_token"));
        CaptureRequest cursor = scrollRequest();
        cursor.includeCursor = true;
        QVERIFY(!ScreenCastPersistence(cursor, false, path).prepare(4).contains("restore_token"));
        QVERIFY(!ScreenCastPersistence(scrollRequest(), true, path).prepare(4).contains("restore_token"));
        QCOMPARE(ScreenCastPersistence(scrollRequest(), false, path).prepare(4).value("restore_token").toString(),
                 QStringLiteral("primary-grant"));
    }

    /** @return 无返回值；验证版本 4 以前的门户不收到新参数，也不会消费已保存令牌。 */
    void legacyPortalsKeepOriginalProtocol()
    {
        QTemporaryDir directory;
        const QString path = directory.filePath("tokens.json");
        ScreenCastPersistence current(scrollRequest(), false, path);
        current.prepare(4);
        current.accept("supported-grant");
        for (uint version : {uint(0), uint(1), uint(3)}) {
            ScreenCastPersistence legacy(scrollRequest(), false, path);
            QVERIFY(legacy.prepare(version).isEmpty());
            legacy.accept("unsupported-grant");
        }
        QCOMPARE(ScreenCastPersistence(scrollRequest(), false, path).prepare(4).value("restore_token").toString(),
                 QStringLiteral("supported-grant"));
    }

    /** @return 无返回值；验证取消授权或未返回新令牌时，不再次使用已消费的旧令牌。 */
    void cancelledOrNonPersistentAuthorizationDoesNotReuseToken()
    {
        QTemporaryDir directory;
        const QString path = directory.filePath("tokens.json");
        ScreenCastPersistence first(scrollRequest(), false, path);
        first.prepare(4);
        first.accept("old-grant");
        {
            ScreenCastPersistence cancelled(scrollRequest(), false, path);
            QVERIFY(cancelled.prepare(4).contains("restore_token"));
        }
        ScreenCastPersistence next(scrollRequest(), false, path);
        QVERIFY(!next.prepare(4).contains("restore_token"));
        next.accept({});
        QVERIFY(!ScreenCastPersistence(scrollRequest(), false, path).prepare(4).contains("restore_token"));
    }

    /** @return 无返回值；验证恢复后出现区域不匹配时，下一次长截图重新请求源选择。 */
    void wrongOutputInvalidatesSavedAuthorization()
    {
        QTemporaryDir directory;
        const QString path = directory.filePath("tokens.json");
        ScreenCastPersistence first(scrollRequest(), false, path);
        first.prepare(4);
        first.accept("wrong-output-grant");
        first.invalidate();
        QVERIFY(!ScreenCastPersistence(scrollRequest(), false, path).prepare(4).contains("restore_token"));
    }
};

QTEST_GUILESS_MAIN(ScreenCastPersistenceTest)
#include "screencast_persistence_test.moc"
