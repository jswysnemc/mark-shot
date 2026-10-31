#include "app_config_store.h"
#include "support/isolated_config.h"
#include "capture_history/history_window.h"
#include "settings/settings_design_tokens.h"
#include "ui/i18n.h"
#include "window_detection.h"

#include <QApplication>
#include <QDir>
#include <QFile>
#include <QJsonObject>
#include <QLabel>
#include <QListWidget>
#include <QPainter>
#include <QPushButton>
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
    QApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("mark-shot-history-test"));
    if (!markshot::appConfigPath().startsWith(isolated.path() + QLatin1Char('/'))) {
        return 1;
    }
    CaptureHistoryWindowTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "capture_history_window_test.moc"
