#include "app_config_store.h"
#include "support/isolated_config.h"
#include "capture_history/history_window.h"
#include "settings/settings_design_tokens.h"
#include "ui/i18n.h"
#include "window_detection.h"

#include <QApplication>
#include <QClipboard>
#include <QDir>
#include <QFile>
#include <QFileDialog>
#include <QJsonObject>
#include <QLabel>
#include <QListWidget>
#include <QPainter>
#include <QMimeData>
#include <QPushButton>
#include <QPersistentModelIndex>
#include <QScopeGuard>
#include <QTemporaryDir>
#include <QtTest>

class CaptureHistoryWindowTest final : public QObject {
    Q_OBJECT
private slots:
    /// @brief 在隔离目录准备两张历史截图，无参数和返回值
    void initTestCase()
    {
        QImage image(640, 360, QImage::Format_RGB32);
        image.fill(QColor(245, 247, 250));
        QPainter painter(&image);
        painter.setPen(QColor(30, 41, 59));
        QFont font = painter.font();
        font.setPointSize(20);
        painter.setFont(font);
        painter.drawText(image.rect(), Qt::AlignCenter, QStringLiteral("Mark Shot\nScreenshot history"));
        painter.end();
        markshot::history::HistoryStore store;
        QVERIFY(store.append(image));
        QVERIFY(store.append(image.scaled(400, 225)));
    }

    /// @brief 提供明暗主题和紧凑窗口组合，无参数和返回值
    void followsConfiguredTheme_data()
    {
        QTest::addColumn<QString>("theme");
        QTest::addColumn<QSize>("size");
        QTest::addColumn<bool>("chinese");
        QTest::newRow("light") << QStringLiteral("light") << QSize(800, 520) << false;
        QTest::newRow("dark") << QStringLiteral("dark") << QSize(800, 520) << false;
        QTest::newRow("light-compact") << QStringLiteral("light") << QSize(440, 320) << false;
        QTest::newRow("dark-compact") << QStringLiteral("dark") << QSize(440, 320) << false;
        QTest::newRow("light-compact-zh") << QStringLiteral("light") << QSize(440, 320) << true;
        QTest::newRow("dark-compact-zh") << QStringLiteral("dark") << QSize(440, 320) << true;
    }

    /**
     * 【历史测试】【主题】历史窗口沿用设置中的浅色主题
     * @return 无返回值
     */
    void followsConfiguredTheme()
    {
        QFETCH(QString, theme);
        QFETCH(QSize, size);
        QFETCH(bool, chinese);
        markshot::i18n::setLanguage(chinese ? markshot::i18n::Language::Chinese : markshot::i18n::Language::English);
        QVERIFY(markshot::writeAppConfigRoot({{QStringLiteral("ui"),
            QJsonObject{{QStringLiteral("theme"), theme}}}}));
        markshot::history::HistoryWindow window;
        window.setAttribute(Qt::WA_DeleteOnClose, false);
        window.resize(size);
        window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));
        QTRY_VERIFY(!window.m_currentImage.isNull());
        const auto expected = markshot::settings::tokens::settingsPalette(markshot::ui::uiThemeModeFromString(theme));
        QCOMPARE(window.palette().color(QPalette::Window), expected.color(QPalette::Window));
        QCOMPARE(window.size(), size);
        const QImage rendered = window.grab().toImage();
        QCOMPARE(rendered.pixelColor(5, 5), expected.color(QPalette::Window));
        for (QPushButton *button : window.findChildren<QPushButton *>()) {
            QVERIFY(window.rect().contains(QRect(button->mapTo(&window, QPoint()), button->size())));
        }
        // 1. 【历史测试】【紧凑布局】图片操作始终位于一行，窄窗口保留可读的预览区域
        auto *list = window.findChild<QListWidget *>(QStringLiteral("historyList"));
        auto *preview = window.findChild<QLabel *>(QStringLiteral("historyPreview"));
        QVERIFY(list);
        QVERIFY(preview);
        QCOMPARE(list->iconSize(), QSize(56, 38));
        QVERIFY(preview->height() >= 160);
        int actionTop = -1;
        int actionCount = 0;
        for (QPushButton *button : window.findChildren<QPushButton *>()) {
            if (button->property("role").toString() == QStringLiteral("quiet")) {
                continue;
            }
            const int top = button->mapTo(&window, QPoint()).y();
            if (actionTop < 0) {
                actionTop = top;
            }
            QCOMPARE(top, actionTop);
            QVERIFY(button->height() <= 32);
            QVERIFY(!button->accessibleName().isEmpty());
            QVERIFY(top < list->mapTo(&window, QPoint()).y());
            ++actionCount;
        }
        QCOMPARE(actionCount, 5);
        list->setCurrentRow(1);
        const QString selected = list->currentItem()->data(Qt::UserRole).toString();
        window.refresh();
        QCOMPARE(list->currentItem()->data(Qt::UserRole).toString(), selected);
        const QString screenshotDirectory = qEnvironmentVariable("MARK_SHOT_TEST_SCREENSHOT_DIR");
        if (!screenshotDirectory.isEmpty()) {
            QVERIFY(QDir().mkpath(screenshotDirectory));
            QVERIFY(rendered.save(QDir(screenshotDirectory).filePath(
                QString::fromLatin1(QTest::currentDataTag()) + QStringLiteral(".png"))));
        }
        // 1. 【历史测试】【主题切换】窗口打开后设置变化也应同步配色
        const QString nextTheme = theme == QStringLiteral("light") ? QStringLiteral("dark") : QStringLiteral("light");
        QVERIFY(markshot::writeAppConfigValue({QStringLiteral("ui"), QStringLiteral("theme")}, nextTheme));
        QApplication::setPalette(markshot::settings::tokens::settingsPalette(
            markshot::ui::uiThemeModeFromString(nextTheme)));
        QCoreApplication::processEvents();
        QCOMPARE(window.palette().color(QPalette::Window),
                 markshot::settings::tokens::settingsPalette(markshot::ui::uiThemeModeFromString(nextTheme))
                     .color(QPalette::Window));
    }

    /**
     * 【历史测试】【关闭入口】没有系统标题栏时也能点击应用内按钮关闭窗口
     * @return 无返回值
     */
    void closesFromVisibleButton()
    {
        markshot::history::HistoryWindow window;
        window.setAttribute(Qt::WA_DeleteOnClose, false);
        window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));
        auto *close = window.findChild<QPushButton *>(QStringLiteral("historyCloseButton"));
        QVERIFY(close);
        QVERIFY(close->isVisible());
        QTest::mouseClick(close, Qt::LeftButton);
        QVERIFY(!window.isVisible());
    }

    /** 【历史测试】【无变化刷新】刷新不销毁已有列表项和选择，无参数和返回值。 */
    void preservesItemsWhenHistoryIsUnchanged()
    {
        markshot::history::HistoryWindow window;
        window.setAttribute(Qt::WA_DeleteOnClose, false);
        auto *list = window.findChild<QListWidget *>(QStringLiteral("historyList"));
        QVERIFY(list);
        list->setCurrentRow(1);
        const QPersistentModelIndex selected(list->currentIndex());
        window.refresh();
        QVERIFY2(selected.isValid(), "Unchanged history rebuilds the entire item model");
        QCOMPARE(list->currentIndex(), QModelIndex(selected));
    }

    /** 【历史测试】【预览上限】小窗口只持有显示尺寸的预览，不长期持有完整 4K 像素，无参数和返回值。 */
    void keepsOnlyDisplaySizedPreview()
    {
        markshot::history::HistoryStore store;
        QImage original(3840, 2160, QImage::Format_RGB32);
        original.fill(QColor(30, 80, 120));
        original.setDevicePixelRatio(2.0);
        QVERIFY(store.append(original));
        const QString name = store.entries().first().fileName;
        const auto cleanup = qScopeGuard([&] { store.remove(name); });
        markshot::history::HistoryWindow window;
        window.setAttribute(Qt::WA_DeleteOnClose, false);
        window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));
        QTRY_VERIFY(!window.m_currentImage.isNull());
        const QSize target = window.m_preview->contentsRect().size() * window.devicePixelRatioF();
        QVERIFY2(window.m_currentImage.width() <= target.width()
                     && window.m_currentImage.height() <= target.height(),
                 "Small preview retains a full-resolution history image");
        const QImage loaded = window.selectedOriginal();
        QCOMPARE(loaded.size(), original.size());
        QCOMPARE(loaded.devicePixelRatio(), original.devicePixelRatio());
        QCOMPARE(loaded.pixelColor(1500, 1500), original.pixelColor(1500, 1500));
        QVERIFY(window.m_currentImage.width() <= target.width());
    }

    /** 【历史测试】【原图导出】复制和保存读取原图，保存对话框期间切换选择不改变图像快照，无参数和返回值。 */
    void exportsOriginalRatherThanPreview()
    {
        QTemporaryDir output;
        QVERIFY(output.isValid());
        markshot::history::HistoryStore store;
        QImage original(2560, 1440, QImage::Format_ARGB32);
        original.fill(QColor(150, 40, 60, 170));
        QVERIFY(store.append(original));
        const QString name = store.entries().first().fileName;
        const auto cleanup = qScopeGuard([&] { store.remove(name); QApplication::clipboard()->clear(); });
        markshot::history::HistoryWindow window;
        window.setAttribute(Qt::WA_DeleteOnClose, false);
        window.show();
        QTRY_VERIFY(!window.m_currentImage.isNull());
        window.copySelected();
        const auto *mime = QApplication::clipboard()->mimeData();
        QVERIFY(mime);
        const QImage copied = mime->hasFormat(QStringLiteral("image/png"))
            ? QImage::fromData(mime->data(QStringLiteral("image/png"))) : QApplication::clipboard()->image();
        QCOMPARE(copied.size(), original.size());
        QCOMPARE(copied.pixelColor(1000, 1000), original.pixelColor(1000, 1000));
        QCOMPARE(store.entries().first().fileName, name);
        window.saveSelected();
        auto *dialog = window.findChild<QFileDialog *>();
        QVERIFY(dialog);
        window.m_list->setCurrentRow(1);
        const QString path = output.filePath(QStringLiteral("saved.png"));
        dialog->selectFile(path);
        QVERIFY(QMetaObject::invokeMethod(dialog, "accept", Qt::DirectConnection));
        QTRY_VERIFY(QFile::exists(path));
        const QImage saved(path);
        QCOMPARE(saved.size(), original.size());
        QCOMPARE(saved.pixelColor(1000, 1000), original.pixelColor(1000, 1000));
        QVERIFY(store.remove(window.selectedFileName()));
        QVERIFY(window.selectedOriginal().isNull());
        window.refresh();
        QTRY_VERIFY(!window.m_currentImage.isNull());
    }
};

/**
 * 【历史测试】【隔离环境】使用临时配置和历史目录运行窗口回归
 * @param argc 参数数量
 * @param argv 参数列表
 * @return 测试退出码
 */
int main(int argc, char **argv)
{
    QTemporaryDir isolated;
    if (!markshot::test::isolateConfig(isolated)) {
        return 1;
    }
    // 1. 【历史测试】【剪贴板隔离】离屏测试不启动真实桌面的剪贴板所有者或会话服务
    if (qEnvironmentVariable("QT_QPA_PLATFORM").startsWith(QStringLiteral("offscreen"))) {
        qputenv("XDG_SESSION_TYPE", "offscreen");
        qputenv("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent-mark-shot-test-bus");
        for (const char *key : {"DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET", "NIRI_SOCKET", "SWAYSOCK"}) qunsetenv(key);
    }
    QApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("mark-shot-history-test"));
    if (!markshot::appConfigPath().startsWith(isolated.path() + QLatin1Char('/'))) {
        return 1;
    }
    CaptureHistoryWindowTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "capture_history_window_test.moc"
