#include "screen_capture_pipewire_screencast.h"
#include "screen_capture_portal_guard.h"

#include <QSet>
#include <QStandardPaths>
#include <QTemporaryDir>
#include <QtTest/QtTest>

namespace {

QSet<QString> g_validTokens;
int g_prompts = 0;
int g_dbusCalls = 0;
int g_libportalCalls = 0;
int g_tokenSequence = 0;
uint g_portalVersion = 4;
bool g_cancel = false;
bool g_useLibportal = false;
bool g_rawCallbackPresent = false;
QRect g_frameGeometry(0, 0, 16, 16);

/** @return 模拟长截图首帧经回退后允许授权的请求。 */
CaptureRequest authorizedRequest()
{
    CaptureRequest request;
    request.preferredOutputName = QStringLiteral("DP-1");
    request.sourceGeometry = QRect(1, 1, 4, 4);
    request.preferScreencast = true;
    request.allowInteractivePortal = true;
    return request;
}

/**
 * 【授权测试】【门户替身】模拟首次授权、单次令牌消费和撤销后的重新确认。
 * @param persistence 真实会话持有的持久化控制器。
 * @param error 模拟取消时的错误输出。
 * @return 模拟授权成功时返回 true。
 */
bool authorize(markshot::portal::ScreenCastPersistence *persistence, QString *error)
{
    const QVariantMap options = persistence->prepare(g_portalVersion);
    const QString previous = options.value(QStringLiteral("restore_token")).toString();
    if (previous.isEmpty() || !g_validTokens.remove(previous)) {
        ++g_prompts;
    }
    if (g_cancel) {
        *error = QStringLiteral("ScreenCast request was cancelled");
        return false;
    }
    if (options.value(QStringLiteral("persist_mode")).toUInt() == 2) {
        const QString token = QStringLiteral("grant-%1").arg(++g_tokenSequence);
        g_validTokens.insert(token);
        persistence->accept(token);
    }
    return true;
}

}  // namespace

/** @return 无返回值；测试不向真实桌面注册应用。 */
void registerHostPortalApplication() {}

/** @param includeCursor 鼠标策略。 @param error 错误输出。 @return 模拟 D-Bus 门户建流结果。 */
bool PortalPipeWireScreencast::startWithDbusPortal(bool includeCursor, QString *error)
{
    Q_UNUSED(includeCursor);
    ++g_dbusCalls;
    g_rawCallbackPresent = bool(m_rawFrameCallback);
    if (!authorize(m_persistence.get(), error)) {
        return false;
    }
    m_streamGeometry = g_frameGeometry;
    m_latestFrame = QImage(g_frameGeometry.size(), QImage::Format_ARGB32_Premultiplied);
    m_latestFrame.fill(Qt::red);
    return true;
}

#ifdef HAVE_LIBPORTAL
/** @param includeCursor 鼠标策略。 @param submitted 是否提交请求。 @param error 错误输出。 @return 模拟 libportal 建流结果。 */
bool PortalPipeWireScreencast::startWithLibportal(bool includeCursor, bool *submitted, QString *error)
{
    Q_UNUSED(includeCursor);
    ++g_libportalCalls;
    *submitted = g_useLibportal;
    if (!g_useLibportal) {
        *error = QStringLiteral("libportal initialization unavailable");
        return false;
    }
    if (!authorize(m_persistence.get(), error)) {
        return false;
    }
    m_streamGeometry = g_frameGeometry;
    m_latestFrame = QImage(g_frameGeometry.size(), QImage::Format_ARGB32_Premultiplied);
    m_latestFrame.fill(Qt::red);
    return true;
}
#endif

class ScreenCastAuthorizationTest final : public QObject {
    Q_OBJECT

private slots:
    /** @return 无返回值；将真实存储限制在测试临时目录。 */
    void initTestCase()
    {
        QVERIFY(m_directory.isValid());
        m_hadDataHome = qEnvironmentVariableIsSet("XDG_DATA_HOME");
        m_oldDataHome = qgetenv("XDG_DATA_HOME");
        qputenv("XDG_DATA_HOME", m_directory.path().toUtf8());
        m_dataPath = QStandardPaths::writableLocation(QStandardPaths::AppLocalDataLocation);
        QVERIFY(m_dataPath.startsWith(m_directory.path() + '/'));
    }

    /** @return 无返回值；恢复环境变量。 */
    void cleanupTestCase()
    {
        if (m_hadDataHome) {
            qputenv("XDG_DATA_HOME", m_oldDataHome);
        } else {
            qunsetenv("XDG_DATA_HOME");
        }
    }

    /** @return 无返回值；重置门户替身和磁盘授权。 */
    void init()
    {
        QFile::remove(QDir(m_dataPath).filePath("portal/screencast-authorizations.json"));
        g_validTokens.clear();
        g_prompts = 0;
        g_dbusCalls = 0;
        g_libportalCalls = 0;
        g_tokenSequence = 0;
        g_portalVersion = 4;
        g_cancel = false;
        g_useLibportal = false;
        g_rawCallbackPresent = false;
        g_frameGeometry = QRect(0, 0, 16, 16);
    }

    /** @return 无返回值；覆盖直接 D-Bus 和可选 libportal 两种会话入口。 */
    void repeatedScrollSessionsAuthorizeOnce_data()
    {
        QTest::addColumn<bool>("useLibportal");
        QTest::newRow("dbus") << false;
#ifdef HAVE_LIBPORTAL
        QTest::newRow("libportal") << true;
#endif
    }

    /** @return 无返回值；验证多次销毁并重建真实采集会话后只确认一次授权。 */
    void repeatedScrollSessionsAuthorizeOnce()
    {
        QFETCH(bool, useLibportal);
        g_useLibportal = useLibportal;
        for (int session = 0; session < 3; ++session) {
            PortalPipeWireScreencast capture;
            const CaptureResult frame = capture.capture(authorizedRequest());
            QVERIFY2(!frame.image.isNull(), qPrintable(frame.error));
            QCOMPARE(frame.image.size(), QSize(4, 4));
            capture.stop();
        }
        QCOMPARE(g_dbusCalls, useLibportal ? 0 : 3);
        QCOMPARE(g_prompts, 1);
        QCOMPARE(g_tokenSequence, 3);
    }

    /** @return 无返回值；验证已有令牌也不会在非交互逐帧请求中尝试可能弹窗的恢复。 */
    void nonInteractiveRequestCannotBootstrapEvenWithToken()
    {
        {
            PortalPipeWireScreencast first;
            QVERIFY(!first.capture(authorizedRequest()).image.isNull());
        }
        PortalPipeWireScreencast second;
        CaptureRequest silent = authorizedRequest();
        silent.allowInteractivePortal = false;
        QVERIFY(second.capture(silent).image.isNull());
        QCOMPARE(g_dbusCalls, 1);
        QVERIFY(!second.capture(authorizedRequest()).image.isNull());
        QCOMPARE(g_prompts, 1);
    }

    /** @return 无返回值；验证权限撤销后只重新确认一次，再继续复用新授权。 */
    void revokedPermissionCanBeAuthorizedAgain()
    {
        {
            PortalPipeWireScreencast first;
            QVERIFY(!first.capture(authorizedRequest()).image.isNull());
        }
        g_validTokens.clear();
        for (int session = 0; session < 2; ++session) {
            PortalPipeWireScreencast capture;
            QVERIFY(!capture.capture(authorizedRequest()).image.isNull());
        }
        QCOMPARE(g_prompts, 2);
    }

    /** @return 无返回值；验证真实裁剪路径发现错误显示器后会删除恢复令牌。 */
    void cropMissForgetsWrongOutput()
    {
        {
            g_frameGeometry.moveLeft(2560);
            PortalPipeWireScreencast wrong;
            const CaptureResult frame = wrong.capture(authorizedRequest());
            QVERIFY(frame.image.isNull());
            QVERIFY(frame.error.contains("does not cover requested geometry"));
        }
        g_frameGeometry.moveLeft(0);
        PortalPipeWireScreencast next;
        QVERIFY(!next.capture(authorizedRequest()).image.isNull());
        QCOMPARE(g_prompts, 2);
    }

    /** @return 无返回值；验证 libportal 初始化失败时回退仍保留录制帧回调。 */
    void rawStreamFallbackPreservesCallbacks()
    {
        PortalPipeWireScreencast capture;
        QString error;
        QVERIFY(capture.startRawStream(authorizedRequest(), [](PipeWireScreencastRawFrame) {}, {}, &error));
        QVERIFY(g_rawCallbackPresent);
    }

    /** @return 无返回值；验证 libportal 提交后取消不会再通过 D-Bus 弹第二次授权。 */
    void cancelledLibportalRequestDoesNotRetryDbus()
    {
#ifdef HAVE_LIBPORTAL
        g_useLibportal = true;
        g_cancel = true;
        PortalPipeWireScreencast capture;
        QVERIFY(capture.capture(authorizedRequest()).image.isNull());
        QCOMPARE(g_prompts, 1);
        QCOMPARE(g_libportalCalls, 1);
        QCOMPARE(g_dbusCalls, 0);
        QVERIFY(!markshot::interactiveScreenCastInProgress());
#else
        QSKIP("libportal support is disabled in this build");
#endif
    }

private:
    QTemporaryDir m_directory;
    QString m_dataPath;
    QByteArray m_oldDataHome;
    bool m_hadDataHome = false;
};

QTEST_GUILESS_MAIN(ScreenCastAuthorizationTest)
#include "screencast_authorization_test.moc"
