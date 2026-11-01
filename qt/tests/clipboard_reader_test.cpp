#include "clipboard_paste/clipboard_reader.h"

#include <QApplication>
#include <QClipboard>
#include <QFile>
#include <QImage>
#include <QProcess>
#include <QScopeGuard>
#include <QTemporaryDir>
#include <QTimer>
#include <QWidget>
#include <QtTest>

#include <cstdio>

namespace {

/** 【剪贴板测试】【图像样本】生成固定像素图片，无参数，返回透明图像。 */
QImage fixtureImage()
{
    QImage image(32, 24, QImage::Format_ARGB32);
    image.fill(QColor(30, 90, 150, 180));
    return image;
}

/** 【剪贴板测试】【独立所有者】向剪贴板发布 kind 指定的样本，以 readyFile 通知就绪，返回事件循环退出码。 */
int runOwner(QApplication &app, const QString &kind, const QString &readyFile)
{
    QWidget window;
    window.setWindowTitle(QStringLiteral("Clipboard fixture owner"));
    window.resize(240, 160);
    window.show();
    QTimer timer;
    QObject::connect(&timer, &QTimer::timeout, &app, [&] {
        if (!app.focusWindow()) {
            return;
        }
        timer.stop();
        if (kind == QStringLiteral("image")) {
            app.clipboard()->setImage(fixtureImage());
        } else {
            app.clipboard()->setText(kind == QStringLiteral("old")
                ? QStringLiteral("previous fixture") : QStringLiteral("current fixture"));
        }
        QFile ready(readyFile);
        if (!ready.open(QIODevice::WriteOnly)) {
            app.exit(2);
            return;
        }
        ready.write("ready");
    });
    timer.start(20);
    return app.exec();
}

}

class ClipboardReaderTest final : public QObject {
    Q_OBJECT

private slots:
    /** 【剪贴板测试】【数据组合】覆盖文字、图片以及已缓存和未缓存的旧数据，无参数和返回值。 */
    void readsCurrentSelectionWithoutFocus_data()
    {
        QTest::addColumn<QString>("kind");
        QTest::addColumn<bool>("cached");
        QTest::newRow("text-uncached") << QStringLiteral("text") << false;
        QTest::newRow("text-cached") << QStringLiteral("text") << true;
        QTest::newRow("image-uncached") << QStringLiteral("image") << false;
        QTest::newRow("image-cached") << QStringLiteral("image") << true;
    }

    /** 【剪贴板测试】【无焦点读取】模拟托盘回调时外部程序已更换剪贴板，校验最新内容，无参数和返回值。 */
    void readsCurrentSelectionWithoutFocus()
    {
        QFETCH(QString, kind);
        QFETCH(bool, cached);
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        QProcess previousOwner;
        QProcess currentOwner;
        const auto cleanup = qScopeGuard([&] {
            for (QProcess *process : {&previousOwner, &currentOwner}) {
                process->kill();
                process->waitForFinished(1000);
            }
        });

        // 1. 【剪贴板测试】【旧选择】外部进程发布旧内容，应用获得焦点并接收格式信息
        const QString oldReady = directory.filePath(QStringLiteral("old-ready"));
        previousOwner.start(QCoreApplication::applicationFilePath(),
                            {QStringLiteral("--owner"), QStringLiteral("old"), oldReady});
        QTRY_VERIFY_WITH_TIMEOUT(QFile::exists(oldReady), 5000);
        QWidget reader;
        reader.setWindowTitle(QStringLiteral("Clipboard reader"));
        reader.resize(240, 160);
        reader.show();
        QTRY_VERIFY_WITH_TIMEOUT(QGuiApplication::focusWindow() == reader.windowHandle(), 5000);
        QTRY_VERIFY(QGuiApplication::clipboard()->mimeData()->hasText());
        if (cached) {
            QCOMPARE(QGuiApplication::clipboard()->text(), QStringLiteral("previous fixture"));
        }

        // 2. 【剪贴板测试】【失去焦点】新进程接管焦点并替换剪贴板，销毁旧数据提供者
        const QString ready = directory.filePath(QStringLiteral("ready"));
        currentOwner.start(QCoreApplication::applicationFilePath(),
                           {QStringLiteral("--owner"), kind, ready});
        QTRY_VERIFY_WITH_TIMEOUT(QFile::exists(ready), 5000);
        QTRY_VERIFY(!QGuiApplication::focusWindow());
        previousOwner.kill();
        QVERIFY(previousOwner.waitForFinished(1000));
        const auto content = markshot::clipboard::readContent();
        if (kind == QStringLiteral("image")) {
            QCOMPARE(QImage::fromData(content->data(QStringLiteral("image/png"))), fixtureImage());
        } else {
            QCOMPARE(content->text(), QStringLiteral("current fixture"));
        }

        // 3. 【剪贴板测试】【恢复焦点】重新映射窗口，验证获得焦点后的正常读取路径
        reader.hide();
        reader.show();
        QTRY_VERIFY_WITH_TIMEOUT(QGuiApplication::focusWindow() == reader.windowHandle(), 5000);
        const auto focusedContent = markshot::clipboard::readContent();
        if (kind == QStringLiteral("text")) {
            QCOMPARE(focusedContent->text(), QStringLiteral("current fixture"));
        } else {
            QCOMPARE(QImage::fromData(focusedContent->data(QStringLiteral("image/png"))), fixtureImage());
        }
    }

    /** 【剪贴板测试】【本地所有者】无焦点但仍持有剪贴板时直接取本地数据，避免等待自身提供数据，无参数和返回值。 */
    void readsOwnedClipboardWithoutFocus()
    {
        QWidget reader;
        reader.resize(240, 160);
        reader.show();
        QTRY_VERIFY_WITH_TIMEOUT(QGuiApplication::focusWindow() == reader.windowHandle(), 5000);
        QGuiApplication::clipboard()->setText(QStringLiteral("owned fixture"));
        QVERIFY(QGuiApplication::clipboard()->ownsClipboard());
        reader.hide();
        QTRY_VERIFY(!QGuiApplication::focusWindow());
        const auto content = markshot::clipboard::readContent();
        QCOMPARE(content->text(), QStringLiteral("owned fixture"));
    }
};

/** 【剪贴板测试】【入口】运行独立所有者或真实 Wayland 读取测试，argc/argv 为命令行参数，返回退出码。 */
int main(int argc, char **argv)
{
    // 【剪贴板测试】【安全入口】拒绝绕过隔离包装器启动，避免测试样本覆盖用户剪贴板
    const QByteArray runtime = qgetenv("MARK_SHOT_CLIPBOARD_TEST_RUNTIME");
    if (runtime.isEmpty() || runtime != qgetenv("XDG_RUNTIME_DIR")) {
        std::fprintf(stderr, "Run through clipboard_wayland_session_test.py to isolate the clipboard.\n");
        return 77;
    }
    QApplication app(argc, argv);
    app.setQuitOnLastWindowClosed(false);
    if (argc == 4 && QString::fromLocal8Bit(argv[1]) == QStringLiteral("--owner")) {
        return runOwner(app, QString::fromLocal8Bit(argv[2]), QString::fromLocal8Bit(argv[3]));
    }
    ClipboardReaderTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "clipboard_reader_test.moc"
