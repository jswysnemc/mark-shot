#include "ocr_result_window/ocr_result_window_style.h"
#include "scroll/scroll_preview_chrome.h"

#include <QImage>
#include <QPushButton>
#include <QTabBar>
#include <QVBoxLayout>
#include <QtTest/QtTest>

class OcrScrollWindowStyleTest final : public QObject {
    Q_OBJECT

private slots:
    /// @brief 提供明暗两种应用调色板，确保灰色胶囊不退化亮色主题
    /// @return 无返回值
    void preservesTheme_data()
    {
        QTest::addColumn<bool>("light");
        QTest::newRow("light") << true;
        QTest::newRow("dark") << false;
    }

    /// @brief 验证窗口配色、灰色胶囊、无边框图标与置顶浅底激活规则
    /// @return 无返回值
    void preservesTheme()
    {
        QFETCH(bool, light);
        QPalette palette;
        palette.setColor(QPalette::Window, QColor(light ? "#F8FAFC" : "#0F172A"));
        palette.setColor(QPalette::Base, QColor(light ? "#FFFFFF" : "#172033"));
        palette.setColor(QPalette::Text, QColor(light ? "#0F172A" : "#E2E8F0"));
        palette.setColor(QPalette::Button, QColor(light ? "#FFFFFF" : "#1E293B"));
        palette.setColor(QPalette::AlternateBase, QColor(light ? "#E2E8F0" : "#1E293B"));
        palette.setColor(QPalette::Highlight, QColor(light ? "#0D9488" : "#5EEAD4"));
        const QString style = markshot::shot::ocrWindowStyleSheet(palette);
        QVERIFY(style.contains("background: " + palette.color(QPalette::Window).name()));
        QVERIFY(style.contains("QTabBar#ocrViewTabs { background: "
                                + QString(light ? "#E8E8E8" : "#2D2D2D")));
        QVERIFY(style.contains("QTabBar#ocrViewTabs::tab:selected { background: "
                                + QString(light ? "#FFFFFF" : "#474747")));
        QVERIFY(style.contains("QPushButton[role=\"icon\"] { background: transparent; border: 0;"));
        QVERIFY(style.contains("QPushButton#ocrPinButton:checked { background: "
                                + QString(light ? "#CCFBF1" : "#163D40") + "; border: 0;"));
        QVERIFY(!style.contains("QPushButton#ocrPinButton:checked { background: "
                                 + palette.color(QPalette::Highlight).name()));
        const QIcon translate = markshot::shot::ocrTranslationIcon(
            palette.color(QPalette::Text), palette.color(QPalette::Highlight));
        QVERIFY(!translate.isNull());
        QVERIFY(translate.pixmap(18, 18, QIcon::Normal, QIcon::Off).toImage()
                != translate.pixmap(18, 18, QIcon::Normal, QIcon::On).toImage());
    }

    /// @brief 覆盖滚动预览的深浅主题，无参数和返回值
    void paintsCompactScrollStatus_data() { preservesTheme_data(); }

    /// @brief 验证主题背景及紧凑状态行，无整行强调色填充
    /// @return 无返回值
    void paintsCompactScrollStatus()
    {
        QFETCH(bool, light);
        const auto palette = markshot::ui::toolbarPalette(light
            ? markshot::ui::UiThemeMode::Light : markshot::ui::UiThemeMode::Dark);
        QImage image(240, 80, QImage::Format_ARGB32_Premultiplied);
        image.fill(Qt::transparent);
        {
            QPainter painter(&image);
            markshot::scroll::drawScrollPreviewSurface(painter, image.rect(), false, palette);
            markshot::scroll::drawScrollPreviewStatus(painter, QRect(12, 10, 216, 22),
                                                      QStringLiteral("Capturing"), {}, palette);
        }
        QCOMPARE(image.pixelColor(120, 60), palette.listFill);
        QVERIFY(image.pixelColor(0, 40) != palette.listFill);
        QCOMPARE(image.pixelColor(16, 20), palette.sliderEnd);
        int tealPixels = 0;
        int whitePixels = 0;
        for (int y = 10; y < 32; ++y) {
            for (int x = 12; x < 228; ++x) {
                const QColor pixel = image.pixelColor(x, y);
                if (pixel.green() > pixel.red() + 50 && pixel.green() > pixel.blue()) {
                    ++tealPixels;
                }
                if (light ? pixel.lightness() < 100 : pixel.lightness() > 200) {
                    ++whitePixels;
                }
            }
        }
        QVERIFY(tealPixels > 10);
        QVERIFY(tealPixels < 50);
        QVERIFY(whitePixels > 10);
        QCOMPARE(image.pixelColor(210, 20), palette.listFill);
    }

    /// @brief 验证拼接长度绘制在状态行右侧，不覆盖左侧状态文字
    /// @return 无返回值
    void paintsStitchedLengthOnTheRight()
    {
        QImage image(240, 80, QImage::Format_ARGB32_Premultiplied);
        image.fill(Qt::transparent);
        {
            QPainter painter(&image);
            markshot::scroll::drawScrollPreviewSurface(painter, image.rect());
            markshot::scroll::drawScrollPreviewStatus(painter, QRect(12, 10, 216, 22),
                                                      QStringLiteral("Waiting"), QStringLiteral("2400 px"));
        }
        QCOMPARE(image.pixelColor(16, 20), QColor("#2DD4BF"));
        int rightWhite = 0;
        for (int x = 180; x < 226; ++x) {
            const QColor pixel = image.pixelColor(x, 21);
            if (pixel.lightness() > 120) {
                ++rightWhite;
            }
        }
        QVERIFY(rightWhite > 4);
    }
};

QTEST_MAIN(OcrScrollWindowStyleTest)

#include "ocr_scroll_window_style_test.moc"
