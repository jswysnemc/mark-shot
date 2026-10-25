#include "annotation_launch.h"
#include "platform/wayland/compositor_pointer.h"
#include "support/isolated_config.h"
#include "support/offscreen_screen_config.h"

#include <QApplication>
#include <QCursor>
#include <QDBusConnection>
#include <QDBusMessage>
#include <QFile>
#include <QScreen>
#include <QTemporaryDir>
#include <QtTest>

class GnomePointerHelper final : public QObject {
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.gnome.Shell.Extensions.MarkShotScrollHelper")

public:
    QPoint pointer{900, 100};

public slots:
    /** 【GNOME测试】【指针位置】返回配置的指针位置，pointerY 接收纵坐标，返回横坐标。 */
    Q_SCRIPTABLE int PointerPosition(int &pointerY)
    {
        pointerY = pointer.y();
        return pointer.x();
    }
};

class GnomeCursorScreenTest final : public QObject {
    Q_OBJECT

private:
    GnomePointerHelper m_helper;
    QObject m_legacyHelper;

private slots:
    /** 【GNOME测试】【总线隔离】每例注册独立模拟扩展，无参数和返回值。 */
    void init()
    {
        auto bus = QDBusConnection::sessionBus();
        QVERIFY(bus.isConnected());
        QVERIFY(bus.registerService(QStringLiteral("org.gnome.Shell")));
        QVERIFY(bus.registerObject(QStringLiteral("/org/gnome/Shell/Extensions/MarkShotScrollHelper"),
                                   &m_helper, QDBusConnection::ExportScriptableSlots));
    }

    /** 【GNOME测试】【总线隔离】断言失败时也清理模拟扩展，无参数和返回值。 */
    void cleanup()
    {
        auto bus = QDBusConnection::sessionBus();
        bus.unregisterObject(QStringLiteral("/org/gnome/Shell/Extensions/MarkShotScrollHelper"));
        bus.unregisterService(QStringLiteral("org.gnome.Shell"));
    }

    /** 【GNOME测试】【屏幕选择】覆盖副屏、负坐标、原点和输出外坐标，无参数和返回值。 */
    void choosesCompositorPointerScreen_data()
    {
        QTest::addColumn<QPoint>("pointer");
        QTest::addColumn<QString>("screenName");
        QTest::newRow("secondary") << QPoint(900, 100) << QStringLiteral("secondary");
        QTest::newRow("negative") << QPoint(-100, -100) << QStringLiteral("negative");
        QTest::newRow("origin") << QPoint(0, 0) << QStringLiteral("primary");
        QTest::newRow("outside-outputs") << QPoint(2000, 2000) << QStringLiteral("primary");
    }

    /** 【GNOME测试】【屏幕选择】验证全局快捷键启动时以合成器位置替代过期 Qt 指针，无参数和返回值。 */
    void choosesCompositorPointerScreen()
    {
        QFETCH(QPoint, pointer);
        QFETCH(QString, screenName);
        m_helper.pointer = pointer;
        // 1. 【GNOME测试】【屏幕选择】私有总线提供 GNOME 指针接口，不接触真实桌面
        auto bus = QDBusConnection::sessionBus();
        const auto request = QDBusMessage::createMethodCall(
            QStringLiteral("org.gnome.Shell"),
            QStringLiteral("/org/gnome/Shell/Extensions/MarkShotScrollHelper"),
            QStringLiteral("org.gnome.Shell.Extensions.MarkShotScrollHelper"),
            QStringLiteral("PointerPosition"));
        const auto reply = bus.call(request, QDBus::Block, 300);
        QCOMPARE(reply.type(), QDBusMessage::ReplyMessage);
        QCOMPARE(reply.arguments().size(), 2);
        const auto compositorPointer = markshot::wayland::compositorPointerPosition();
        QVERIFY(compositorPointer.has_value());
        QCOMPARE(*compositorPointer, pointer);

        // 2. 【GNOME测试】【屏幕选择】以合成器坐标选择屏幕，输出外坐标保留 Qt 回退
        QCursor::setPos(QPoint(100, 100));
        QCOMPARE(QGuiApplication::screenAt(QCursor::pos())->name(), QStringLiteral("primary"));
        QCOMPARE(markshot::focusedScreen()->name(), screenName);
    }

    /** 【GNOME测试】【接口回退】提供未安装和旧版扩展两种情况，无参数和返回值。 */
    void unavailableHelperFallsBack_data()
    {
        QTest::addColumn<bool>("legacy");
        QTest::newRow("not-installed") << false;
        QTest::newRow("legacy-interface") << true;
    }

    /** 【GNOME测试】【接口回退】缺少指针接口时继续使用 Qt 坐标，无参数和返回值。 */
    void unavailableHelperFallsBack()
    {
        QFETCH(bool, legacy);
        auto bus = QDBusConnection::sessionBus();
        bus.unregisterObject(QStringLiteral("/org/gnome/Shell/Extensions/MarkShotScrollHelper"));
        if (legacy) {
            QVERIFY(bus.registerObject(QStringLiteral("/org/gnome/Shell/Extensions/MarkShotScrollHelper"),
                                       &m_legacyHelper, QDBusConnection::ExportScriptableSlots));
        } else {
            bus.unregisterService(QStringLiteral("org.gnome.Shell"));
        }
        QCursor::setPos(QPoint(900, 100));
        QVERIFY(!markshot::wayland::compositorPointerPosition().has_value());
        QCOMPARE(markshot::focusedScreen()->name(), QStringLiteral("secondary"));
    }
};

/** 【GNOME测试】【环境隔离】使用私有总线和多屏配置，argc/argv 为命令行参数，返回测试退出码。 */
int main(int argc, char **argv)
{
    // 1. 【GNOME测试】【路径回归】覆盖包含空格和冒号的配置目录
    QTemporaryDir isolated(QDir(QDir::tempPath()).filePath(QStringLiteral("mark-shot gnome:pointer-XXXXXX")));
    if (!markshot::test::isolateConfig(isolated)) {
        return 1;
    }
    // 2. 【GNOME测试】【虚拟屏幕】通过相对配置文件名加载三块屏幕
    markshot::test::OffscreenScreenConfig config(isolated,
        R"({"synchronousWindowSystemEvents":true,"screens":[{"name":"primary","x":0,"y":0,"width":800,"height":600},{"name":"secondary","x":800,"y":0,"width":800,"height":600},{"name":"negative","x":-800,"y":-600,"width":800,"height":600}]})");
    if (!config.isValid()) {
        return 1;
    }
    qputenv("XDG_SESSION_TYPE", "wayland");
    qputenv("XDG_CURRENT_DESKTOP", "GNOME");
    qputenv("PATH", isolated.path().toUtf8());
    QApplication app(argc, argv);
    GnomeCursorScreenTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "gnome_cursor_screen_test.moc"
