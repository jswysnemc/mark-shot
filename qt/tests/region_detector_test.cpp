#include "region_detection/region_detector.h"

#include <QImage>
#include <QPainter>
#include <QtTest/QtTest>

#ifndef MARK_SHOT_TEST_SOURCE_DIR
#define MARK_SHOT_TEST_SOURCE_DIR "."
#endif

using markshot::region_detection::RegionCandidate;
using markshot::region_detection::RegionDetector;
using markshot::region_detection::SelectionCursor;

class RegionDetectorTest final : public QObject {
    Q_OBJECT

private:
    /**
     * 创建纯色画布。
     * @param width 宽度。
     * @param height 高度。
     * @param shade 灰度。
     * @return 图像。
     */
    static QImage canvas(int width, int height, int shade)
    {
        QImage image(width, height, QImage::Format_ARGB32);
        image.fill(QColor(shade, shade, shade));
        return image;
    }

    /**
     * 用纯色填充矩形。
     * @param image 目标图像。
     * @param rect 矩形。
     * @param color 填充色。
     * @return 无返回值。
     */
    static void fill(QImage *image, QRect rect, QColor color)
    {
        QPainter painter(image);
        painter.setCompositionMode(QPainter::CompositionMode_Source);
        painter.fillRect(rect, color);
    }

    /**
     * 绘制 1 像素空心矩形。
     * @param image 目标图像。
     * @param rect 矩形。
     * @param color 描边色。
     * @return 无返回值。
     */
    static void drawHollow(QImage *image, QRect rect, QColor color)
    {
        const QRgb pixel = color.rgba();
        for (int x = rect.left(); x < rect.x() + rect.width(); ++x) {
            image->setPixel(x, rect.top(), pixel);
            image->setPixel(x, rect.y() + rect.height() - 1, pixel);
        }
        for (int y = rect.top(); y < rect.y() + rect.height(); ++y) {
            image->setPixel(rect.left(), y, pixel);
            image->setPixel(rect.x() + rect.width() - 1, y, pixel);
        }
    }

    /**
     * 断言矩形与期望值相差不超过容差。
     * @param actual 实际矩形。
     * @param expected 期望矩形。
     * @param tolerance 容差。
     * @return 无返回值。
     */
    static void assertBounds(QRect actual, QRect expected, int tolerance)
    {
        const bool ok = qAbs(actual.x() - expected.x()) <= tolerance
            && qAbs(actual.y() - expected.y()) <= tolerance
            && qAbs(actual.x() + actual.width() - (expected.x() + expected.width())) <= tolerance
            && qAbs(actual.y() + actual.height() - (expected.y() + expected.height())) <= tolerance;
        QVERIFY2(ok,
                 qPrintable(QStringLiteral("selected %1x%2+%3+%4, expected %5x%6+%7+%8 ±%9px")
                                .arg(actual.width())
                                .arg(actual.height())
                                .arg(actual.x())
                                .arg(actual.y())
                                .arg(expected.width())
                                .arg(expected.height())
                                .arg(expected.x())
                                .arg(expected.y())
                                .arg(tolerance)));
    }

private slots:
    /**
     * 验证平坦或过小的图像不会崩溃，并回退到整图。
     * @return 无返回值。
     */
    void flatAndTinyImagesFallBackWithoutPanicking()
    {
        const QVector<QSize> sizes{{320, 200}, {1, 1}, {2, 3}, {3, 3}, {1, 80}, {0, 0}};
        for (const QSize &size : sizes) {
            const RegionDetector detector(canvas(size.width(), size.height(), 40));
            QCOMPARE(detector.regionAtNormalized(1.0f, 1.0f),
                     QRect(0, 0, size.width(), size.height()));
        }
    }

    /**
     * 验证嵌套控件保持精确边界，并保留父链。
     * @return 无返回值。
     */
    void nestedControlsKeepExactBoundariesAndParentChain()
    {
        QImage image = canvas(360, 240, 20);
        const QRect panel(30, 20, 300, 200);
        const QRect button(100, 80, 120, 48);
        fill(&image, panel, QColor(45, 45, 45));
        fill(&image, button, QColor(75, 75, 75));
        const RegionDetector detector(image);
        assertBounds(detector.regionAtNormalized(0.4f, 0.4f), button, 1);
        const QVector<QRect> chain = detector.regionsAt(detector.pointFromNormalized(0.4f, 0.4f));
        bool hasPanel = false;
        for (const QRect &rect : chain) {
            hasPanel = hasPanel || rect == panel
                || (qAbs(rect.x() - panel.x()) <= 1 && qAbs(rect.y() - panel.y()) <= 1
                    && qAbs(rect.width() - panel.width()) <= 1 && qAbs(rect.height() - panel.height()) <= 1);
        }
        QVERIFY2(hasPanel, "missing panel in region chain");
        QCOMPARE(chain.last(), QRect(0, 0, 360, 240));
    }

    /**
     * 验证弱暗色面板和等亮度色边界仍可被识别。
     * @return 无返回值。
     */
    void detectsWeakDarkPanelAndEqualLuminanceColorBoundaries()
    {
        const QVector<QPair<QColor, QColor>> cases{
            {QColor(28, 28, 28), QColor(36, 36, 36)},
            {QColor(200, 200, 200), QColor(208, 208, 208)},
            {QColor(160, 50, 50), QColor(50, 106, 50)},
        };
        for (const auto &pair : cases) {
            QImage image = canvas(360, 240, 0);
            fill(&image, QRect(0, 0, 360, 240), pair.first);
            const QRect rect(60, 45, 230, 140);
            fill(&image, rect, pair.second);
            const RegionDetector detector(image);
            assertBounds(detector.regionAtNormalized(0.5f, 0.5f), rect, 1);
        }
    }

    /**
     * 验证同图中的亮文字不会淹没弱面板边框。
     * @return 无返回值。
     */
    void weakPanelBordersSurviveBrightText()
    {
        QImage image = canvas(640, 360, 28);
        const QRect panel(35, 30, 560, 290);
        fill(&image, panel, QColor(36, 36, 36));
        for (int row = 0; row < 9; ++row) {
            for (int column = 0; column < 28; ++column) {
                const int x = 55 + column * 18;
                const int y = 48 + row * 28;
                fill(&image, QRect(x, y, 2, 17), QColor(220, 220, 220));
                fill(&image, QRect(x, y, 10, 2), QColor(220, 220, 220));
                fill(&image, QRect(x, y + 8, 8, 2), QColor(220, 220, 220));
            }
        }
        const RegionDetector detector(image);
        assertBounds(detector.regionAtNormalized(0.8f, 0.7f), panel, 1);
    }

    /**
     * 验证相邻面板不会被合并，贴边窗口仍可被截断识别。
     * @return 无返回值。
     */
    void adjacentPanelsAreNotMerged()
    {
        QImage image = canvas(360, 240, 22);
        const QRect left(0, 30, 150, 210);
        const QRect right(162, 30, 198, 210);
        fill(&image, left, QColor(45, 45, 45));
        fill(&image, right, QColor(50, 50, 50));
        const RegionDetector detector(image);
        assertBounds(detector.regionAtNormalized(0.2f, 0.5f), left, 1);
        assertBounds(detector.regionAtNormalized(0.7f, 0.5f), right, 1);
    }

    /**
     * 验证空心矩形和圆角控件仍能通过边界校验。
     * @return 无返回值。
     */
    void hollowAndRoundedControlsSurviveBoundaryValidation()
    {
        QImage image = canvas(400, 250, 30);
        drawHollow(&image, QRect(25, 30, 160, 150), QColor(210, 210, 210));
        const QRect rounded(225, 65, 130, 80);
        for (int y = rounded.y(); y < rounded.y() + rounded.height(); ++y) {
            for (int x = rounded.x(); x < rounded.x() + rounded.width(); ++x) {
                const int dx = qMax(0, qMax(rounded.x() + 12 - x, x - (rounded.x() + rounded.width() - 13)));
                const int dy = qMax(0, qMax(rounded.y() + 12 - y, y - (rounded.y() + rounded.height() - 13)));
                if (dx * dx + dy * dy <= 144) {
                    image.setPixel(x, y, qRgb(50, 50, 50));
                }
            }
        }
        const RegionDetector detector(image);
        assertBounds(detector.regionAtNormalized(0.2f, 0.4f), QRect(25, 30, 160, 150), 1);
        assertBounds(detector.regionAtNormalized(0.7f, 0.4f), rounded, 1);
    }

    /**
     * 验证 4K 原图精修仍能保留小控件。
     * @return 无返回值。
     */
    void originalResolutionRefinementPreservesSmallControlsOn4k()
    {
        QImage image = canvas(3840, 2160, 24);
        const QRect panel(421, 213, 1500, 1100);
        const QRect button(631, 451, 30, 30);
        fill(&image, panel, QColor(42, 42, 42));
        fill(&image, button, QColor(100, 100, 100));
        const RegionDetector detector(image);
        assertBounds(detector.regionAtNormalized(645.0f / 3840.0f, 465.0f / 2160.0f), button, 2);
        assertBounds(detector.regionAtNormalized(1200.0f / 3840.0f, 900.0f / 2160.0f), panel, 2);
    }

    /**
     * 验证按钮内文字不会抢走选区。
     * @return 无返回值。
     */
    void textInsideAButtonDoesNotStealTheSelection()
    {
        QImage image = canvas(640, 360, 25);
        const QRect panel(30, 30, 580, 300);
        const QRect button(180, 140, 250, 60);
        fill(&image, panel, QColor(42, 42, 42));
        fill(&image, button, QColor(65, 65, 65));
        const int glyphs[4][7] = {
            {14, 17, 17, 17, 17, 17, 14},
            {30, 17, 17, 30, 16, 16, 16},
            {31, 16, 16, 30, 16, 16, 31},
            {17, 25, 21, 19, 17, 17, 17},
        };
        for (int index = 0; index < 4; ++index) {
            for (int y = 0; y < 7; ++y) {
                for (int x = 0; x < 5; ++x) {
                    if (glyphs[index][y] & (1 << (4 - x))) {
                        fill(&image, QRect(202 + index * 30 + x * 3, 157 + y * 3, 3, 3), QColor(225, 225, 225));
                    }
                }
            }
        }
        const RegionDetector detector(image);
        for (int x : {205, 215, 240, 285, 310}) {
            assertBounds(detector.regionAtNormalized(static_cast<float>(x) / 640.0f, 168.0f / 360.0f), button, 1);
        }
    }

    /**
     * 验证带图标的紧凑按钮仍可单独选中。
     * @return 无返回值。
     */
    void compactButtonsWithIconsRemainSelectable()
    {
        QImage image = canvas(360, 240, 20);
        const QRect panel(30, 20, 300, 200);
        const QRect button(100, 80, 24, 24);
        fill(&image, panel, QColor(45, 45, 45));
        fill(&image, button, QColor(85, 85, 85));
        fill(&image, QRect(110, 84, 4, 16), QColor(220, 220, 220));
        fill(&image, QRect(104, 90, 16, 4), QColor(220, 220, 220));
        const RegionDetector detector(image);
        assertBounds(detector.regionAtNormalized(111.0f / 360.0f, 91.0f / 240.0f), button, 1);
    }

    /**
     * 验证平滑渐变不会被当成面板。
     * @return 无返回值。
     */
    void aSmoothGradientIsNotAPanel()
    {
        QImage image(480, 320, QImage::Format_ARGB32);
        for (int y = 0; y < 320; ++y) {
            for (int x = 0; x < 480; ++x) {
                image.setPixel(x, y, qRgb(20 + x / 8, 30 + y / 6, 80));
            }
        }
        const RegionDetector detector(image);
        QCOMPARE(detector.regionAtNormalized(0.5f, 0.5f), QRect(0, 0, 480, 320));
    }

    /**
     * 验证滚轮选择能抵抗轻微抖动，并保留单击刷新。
     * @return 无返回值。
     */
    void wheelChoiceSurvivesJitterAndClickRefresh()
    {
        QImage image = canvas(360, 240, 20);
        const QRect panel(30, 20, 300, 200);
        const QRect button(100, 80, 120, 48);
        fill(&image, panel, QColor(45, 45, 45));
        fill(&image, button, QColor(75, 75, 75));
        const RegionDetector detector(image);
        SelectionCursor cursor;
        const QPoint point = detector.pointFromNormalized(0.4f, 0.4f);
        assertBounds(cursor.hover(detector, point), button, 1);
        assertBounds(cursor.step(detector, point, 1), panel, 1);
        assertBounds(cursor.hover(detector, detector.pointFromNormalized(0.401f, 0.401f)), panel, 1);
        assertBounds(cursor.step(detector, point, -1), button, 1);
        cursor.step(detector, point, 1);
        QCOMPARE(cursor.step(detector, point, 1), QRect(0, 0, 360, 240));
        QCOMPARE(cursor.step(detector, point, 1), QRect(0, 0, 360, 240));
    }

    /**
     * 验证高质量大候选可以压过噪声小碎片。
     * @return 无返回值。
     */
    void qualityCanOutweighASmallNoisyCandidate()
    {
        RegionCandidate small;
        small.rect = QRect(50, 50, 20, 20);
        small.quality = 0.60f;
        small.sides = {0.6f, 0.6f, 0.6f, 0.6f};
        small.interiorEdges = 0.3f;
        small.sources.append(QStringLiteral("contour"));
        RegionCandidate large;
        large.rect = QRect(20, 20, 240, 100);
        large.quality = 0.95f;
        large.sides = {0.95f, 0.95f, 0.95f, 0.95f};
        large.interiorEdges = 0.1f;
        large.sources.append(QStringLiteral("background"));
        const RegionDetector detector(320, 200, {small, large});
        QCOMPARE(detector.regionAtNormalized(0.18f, 0.28f), QRect(20, 20, 240, 100));
    }

    /**
     * 验证合成器窗口不会压过窗口内的 UI 控件，滚轮仍能切到窗口。
     * @return 无返回值。
     */
    void compositorWindowsDoNotHideContainedUiRegions()
    {
        RegionCandidate button;
        button.rect = QRect(100, 80, 120, 48);
        button.quality = 0.72f;
        button.sides = {0.72f, 0.72f, 0.72f, 0.72f};
        button.interiorEdges = 0.12f;
        button.sources.append(QStringLiteral("contour"));
        RegionCandidate window;
        window.rect = QRect(40, 30, 280, 200);
        window.quality = 0.92f;
        window.sides = {0.92f, 0.92f, 0.92f, 0.92f};
        window.interiorEdges = 0.04f;
        window.sources.append(QStringLiteral("window"));
        RegionDetector detector(360, 240, {button, window});
        detector.addWindowRects({window.rect});
        const QPoint point = detector.pointFromNormalized(0.4f, 0.4f);
        QCOMPARE(detector.regionAt(point), button.rect);

        SelectionCursor cursor;
        QCOMPARE(cursor.hover(detector, point), button.rect);
        QCOMPARE(cursor.step(detector, point, 1), window.rect);
        QCOMPARE(cursor.step(detector, point, 1), QRect(0, 0, 360, 240));
        QCOMPARE(detector.regionAtNormalized(0.15f, 0.2f), window.rect);
    }

    /**
     * 验证越出合成器窗口的图像候选不会取代窗口矩形，窗口内的控件仍优先（issue #140）。
     * @return 无返回值。
     */
    void uiRegionsCrossingTheWindowDoNotReplaceIt()
    {
        // 1. 终端几乎占满屏幕，深色内容与顶栏、壁纸连成一块越出窗口的高质量候选
        RegionCandidate overflow;
        overflow.rect = QRect(20, 0, 3820, 2160);
        overflow.quality = 0.9f;
        overflow.sides = {0.9f, 0.9f, 0.9f, 0.9f};
        overflow.sources.append(QStringLiteral("contour"));
        RegionCandidate prompt;
        prompt.rect = QRect(60, 140, 360, 60);
        prompt.quality = 0.85f;
        prompt.sides = {0.85f, 0.85f, 0.85f, 0.85f};
        prompt.sources.append(QStringLiteral("contour"));
        const QRect window(22, 84, 3794, 2050);
        RegionDetector detector(3840, 2160, {overflow, prompt});
        detector.addWindowRects({window});

        QCOMPARE(detector.regionAt(QPoint(1900, 1000)), window);
        QCOMPARE(detector.regionAt(QPoint(200, 170)), prompt.rect);
        // 2. 没有窗口覆盖的位置仍沿用图像候选
        QCOMPARE(detector.regionAt(QPoint(1900, 30)), overflow.rect);
    }

    /**
     * 验证真实工具栏截图优先选中完整工具栏而不是文字碎片。
     * @return 无返回值。
     */
    void realToolbarSelectionPrefersTheToolbarOverLabelFragments()
    {
        const QImage image(QStringLiteral(MARK_SHOT_TEST_SOURCE_DIR "/tests/fixtures/region-toolbar-context.png"));
        QVERIFY(!image.isNull());
        const RegionDetector detector(image);
        assertBounds(detector.regionAtNormalized(1310.0f / 1600.0f, 181.0f / 205.0f), QRect(100, 70, 1350, 119), 3);
        assertBounds(detector.regionAtNormalized(195.0f / 1600.0f, 115.0f / 205.0f), QRect(176, 93, 44, 44), 2);
    }

    /**
     * 验证密集文本行不会生成竞争性小控件。
     * @return 无返回值。
     */
    void denseTextRowsDoNotCreateCompetingControls()
    {
        QImage image = canvas(640, 360, 20);
        const QRect panel(30, 30, 580, 300);
        fill(&image, panel, QColor(45, 45, 45));
        for (int row = 0; row < 8; ++row) {
            for (int column = 0; column < 18; ++column) {
                drawHollow(&image, QRect(60 + column * 28, 55 + row * 30, 11, 18), QColor(200, 200, 200));
            }
        }
        const RegionDetector detector(image);
        for (int column : {0, 4, 8, 12, 16}) {
            assertBounds(detector.regionAtNormalized(static_cast<float>(65 + column * 28) / 640.0f, 65.0f / 360.0f),
                         panel,
                         1);
        }
    }

    /**
     * 验证断边和内容分隔线仍能保留外层窗口。
     * @return 无返回值。
     */
    void brokenOutlineAndContentSeparatorsPreserveTheOuterWindow()
    {
        QImage image = canvas(600, 400, 20);
        const QRect window(70, 40, 450, 320);
        fill(&image, window, QColor(45, 45, 45));
        fill(&image, QRect(70, 100, 450, 2), QColor(80, 80, 80));
        fill(&image, QRect(70, 240, 450, 2), QColor(80, 80, 80));
        fill(&image, QRect(70, 180, 2, 3), QColor(20, 20, 20));
        const RegionDetector detector(image);
        const QVector<QRect> chain = detector.regionsAt(detector.pointFromNormalized(0.5f, 0.4f));
        bool hasWindow = false;
        for (const QRect &rect : chain) {
            hasWindow = hasWindow
                || (qAbs(rect.x() - window.x()) <= 2 && qAbs(rect.y() - window.y()) <= 2
                    && qAbs(rect.x() + rect.width() - (window.x() + window.width())) <= 2
                    && qAbs(rect.y() + rect.height() - (window.y() + window.height())) <= 2);
        }
        QVERIFY2(hasWindow, "outer window missing from region chain");
    }

    /**
     * 验证交叉候选不会截断有效父链。
     * @return 无返回值。
     */
    void crossingCandidatesDoNotHideAValidParent()
    {
        auto make = [](QRect rect, float quality) {
            RegionCandidate candidate;
            candidate.rect = rect;
            candidate.quality = quality;
            candidate.sides = {quality, quality, quality, quality};
            candidate.interiorEdges = 0.1f;
            candidate.sources.append(QStringLiteral("contour"));
            return candidate;
        };
        const QRect control(80, 80, 100, 60);
        const QRect parent(40, 40, 220, 160);
        const RegionDetector detector(320, 240, {make(QRect(120, 60, 30, 40), 0.6f), make(control, 0.95f), make(parent, 0.95f)});
        SelectionCursor cursor;
        const QPoint point = detector.pointFromNormalized(0.4f, 0.4f);
        QCOMPARE(cursor.hover(detector, point), control);
        QCOMPARE(cursor.step(detector, point, 1), parent);
        const QVector<QRect> chain = detector.regionsAt(point);
        QVERIFY(chain.contains(control));
        QVERIFY(chain.contains(parent));
    }

    /**
     * 验证缩放后区域层级保持一致。
     * @return 无返回值。
     */
    void scalesConsistentlyWithoutChangingTheRegionHierarchy()
    {
        QImage image = canvas(360, 240, 20);
        fill(&image, QRect(70, 60, 180, 100), QColor(50, 50, 50));
        const QImage scaled = image.scaled(1440, 960, Qt::IgnoreAspectRatio, Qt::FastTransformation);
        const RegionDetector detector(scaled);
        assertBounds(detector.regionAtNormalized(0.4f, 0.4f), QRect(280, 240, 720, 400), 2);
    }

    /**
     * 验证真实工具栏截图的分析耗时保持在交互可接受范围内。
     * @return 无返回值。
     */
    void toolbarFixtureDetectionFinishesQuickly()
    {
        const QImage image(QStringLiteral(MARK_SHOT_TEST_SOURCE_DIR "/tests/fixtures/region-toolbar-context.png"));
        QVERIFY(!image.isNull());
        QElapsedTimer timer;
        timer.start();
        const RegionDetector detector(image);
        const qint64 elapsedMs = timer.elapsed();
        qInfo("toolbar fixture detection %lld ms, candidates=%d",
              elapsedMs,
              detector.candidates().size());
        QVERIFY2(elapsedMs < 400, qPrintable(QStringLiteral("toolbar detection took %1 ms").arg(elapsedMs)));
        QVERIFY(!detector.candidates().isEmpty());
    }
};

QTEST_APPLESS_MAIN(RegionDetectorTest)

#include "region_detector_test.moc"
