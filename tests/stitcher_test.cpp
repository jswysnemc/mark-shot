#include "scroll/stitcher.h"

#include <QPainter>

#include <QtTest/QtTest>

namespace {

/// @brief Helper function to compute a pseudo-random color for a given row index.
/// @param index The index of the row.
/// @return The calculated QColor object.
QColor rowColor(int index)
{
    return QColor((index * 3 + 17) % 256,
                  (index * 5 + 29) % 256,
                  (index * 7 + 43) % 256);
}

/// @brief Helper function to generate a test frame with vertical color strips.
/// @param firstRow Starting index for row color calculations.
/// @param height Height of the generated frame.
/// @param width Width of the generated frame (defaults to 24).
/// @return The generated test frame QImage.
QImage verticalFrame(int firstRow, int height, int width = 24)
{
    QImage image(width, height, QImage::Format_ARGB32_Premultiplied);
    QPainter painter(&image);
    for (int y = 0; y < height; ++y) {
        painter.fillRect(QRect(0, y, width, 1), rowColor(firstRow + y));
    }
    painter.end();
    return image;
}

/// @brief Helper function to generate a test frame with horizontal color strips.
/// @param firstColumn Starting index for row color calculations.
/// @param width Width of the generated frame.
/// @param height Height of the generated frame (defaults to 12).
/// @return The generated test frame QImage.
QImage horizontalFrame(int firstColumn, int width, int height = 12)
{
    QImage image(width, height, QImage::Format_ARGB32_Premultiplied);
    QPainter painter(&image);
    for (int x = 0; x < width; ++x) {
        painter.fillRect(QRect(x, 0, 1, height), rowColor(firstColumn + x));
    }
    painter.end();
    return image;
}

/// @brief Helper function to generate a standard StitchConfig for testing.
/// @return The generated StitchConfig structure.
markshot::scroll::StitchConfig testConfig()
{
    return markshot::scroll::StitchConfig{20, 0.5f, 10, 0.01f};
}

// 固定带测试使用的 header/footer 颜色
const QColor kHeaderColor(200, 60, 40);
const QColor kFooterColor(30, 90, 180);

/// @brief 生成滚动内容行的颜色,三个通道的模数互质,避免周期性撞色。
/// @param index 内容行下标。
/// @return 该行的颜色。
QColor contentColor(int index)
{
    return QColor((index * 7 + 31) % 251,
                  (index * 11 + 59) % 241,
                  (index * 13 + 83) % 239);
}

/// @brief 生成带固定 header/footer 条带的测试帧。
/// @param headerRows 顶部固定带行数。
/// @param footerRows 底部固定带行数。
/// @param firstContent 内容区首行的内容下标。
/// @param height 帧高。
/// @param width 帧宽。
/// @return 生成的测试帧。
QImage stickyFrame(int headerRows, int footerRows, int firstContent, int height, int width = 64)
{
    QImage image(width, height, QImage::Format_ARGB32_Premultiplied);
    QPainter painter(&image);
    for (int y = 0; y < height; ++y) {
        QColor color;
        if (y < headerRows) {
            color = kHeaderColor;
        } else if (y >= height - footerRows) {
            color = kFooterColor;
        } else {
            color = contentColor(firstContent + y - headerRows);
        }
        painter.fillRect(QRect(0, y, width, 1), color);
    }
    painter.end();
    return image;
}

}  // namespace

/// @brief Test suite for testing the Stitcher class.
class StitcherTest : public QObject
{
    /// @brief Qt private signal structure.
    /// @brief Qt meta-object instance.
    Q_OBJECT

private slots:
    void appendsForwardScrollFrames()
    {
        markshot::scroll::Stitcher stitcher(testConfig());

        const markshot::scroll::StitchResult first = stitcher.pushFrame(verticalFrame(0, 80));
        QCOMPARE(first.status, markshot::scroll::StitchStatus::FirstFrame);
        QCOMPARE(first.added, 80);

        const markshot::scroll::StitchResult second = stitcher.pushFrame(verticalFrame(60, 80));
        QCOMPARE(second.status, markshot::scroll::StitchStatus::Appended);
        QCOMPARE(second.edge, markshot::scroll::StitchEdge::End);
        QCOMPARE(second.added, 60);
        QCOMPARE(second.position, 60);

        const QImage full = stitcher.fullImage();
        QCOMPARE(full.size(), QSize(24, 140));
        QCOMPARE(full.pixelColor(0, 0), rowColor(0));
        QCOMPARE(full.pixelColor(0, 139), rowColor(139));
        QCOMPARE(stitcher.stats().frameCount, 2);
        QCOMPARE(stitcher.stats().totalHeight, 140);
    }

    void prependsReverseScrollFrames()
    {
        markshot::scroll::Stitcher stitcher(testConfig());

        stitcher.pushFrame(verticalFrame(60, 80));
        const markshot::scroll::StitchResult second = stitcher.pushFrame(verticalFrame(0, 80));

        QCOMPARE(second.status, markshot::scroll::StitchStatus::Appended);
        QCOMPARE(second.edge, markshot::scroll::StitchEdge::Start);
        QCOMPARE(second.added, 60);

        const QImage full = stitcher.fullImage();
        QCOMPARE(full.size(), QSize(24, 140));
        QCOMPARE(full.pixelColor(0, 0), rowColor(0));
        QCOMPARE(full.pixelColor(0, 139), rowColor(139));
    }

    void stitchesHorizontalAxisByTransposingFrames()
    {
        markshot::scroll::Stitcher stitcher(testConfig());
        stitcher.setAxis(markshot::scroll::ScrollAxis::Horizontal);

        stitcher.pushFrame(horizontalFrame(0, 80));
        const markshot::scroll::StitchResult second = stitcher.pushFrame(horizontalFrame(60, 80));

        QCOMPARE(second.status, markshot::scroll::StitchStatus::Appended);
        QCOMPARE(second.edge, markshot::scroll::StitchEdge::End);
        QCOMPARE(second.added, 60);

        const QImage full = stitcher.fullImage();
        QCOMPARE(full.size(), QSize(140, 12));
        QCOMPARE(full.pixelColor(0, 0), rowColor(0));
        QCOMPARE(full.pixelColor(139, 0), rowColor(139));

        QVERIFY(stitcher.axisLocked());
        stitcher.setAxis(markshot::scroll::ScrollAxis::Vertical);
        QCOMPARE(stitcher.axis(), markshot::scroll::ScrollAxis::Horizontal);
    }

    void rejectsFramesWithDifferentWidths()
    {
        markshot::scroll::Stitcher stitcher(testConfig());

        stitcher.pushFrame(verticalFrame(0, 80, 24));
        const markshot::scroll::StitchResult second = stitcher.pushFrame(verticalFrame(60, 80, 25));

        QCOMPARE(second.status, markshot::scroll::StitchStatus::NoMatch);
        QCOMPARE(stitcher.fullImage().size(), QSize(24, 80));
        QCOMPARE(stitcher.stats().frameCount, 1);
    }

    void keepsStickyFooterOnceAtBottom()
    {
        markshot::scroll::Stitcher stitcher(testConfig());
        constexpr int kFooter = 30;
        constexpr int kHeight = 200;
        constexpr int kContentPerFrame = kHeight - kFooter;

        // 三帧向下滚动,每帧底部带 30 行固定 footer,相邻帧内容重叠 50 行
        stitcher.pushFrame(stickyFrame(0, kFooter, 0, kHeight));
        const markshot::scroll::StitchResult second =
            stitcher.pushFrame(stickyFrame(0, kFooter, 120, kHeight));
        const markshot::scroll::StitchResult third =
            stitcher.pushFrame(stickyFrame(0, kFooter, 240, kHeight));

        QCOMPARE(second.status, markshot::scroll::StitchStatus::Appended);
        QCOMPARE(second.edge, markshot::scroll::StitchEdge::End);
        QCOMPARE(second.added, 120);
        QCOMPARE(third.status, markshot::scroll::StitchStatus::Appended);
        QCOMPARE(third.added, 120);

        // 输出应为连续内容加末尾一份 footer:240+170 内容行 + 30 footer 行
        const QImage full = stitcher.fullImage();
        QCOMPARE(full.height(), 240 + kContentPerFrame + kFooter);
        QCOMPARE(full.pixelColor(4, 0), contentColor(0));
        QCOMPARE(full.pixelColor(4, 100), contentColor(100));
        QCOMPARE(full.pixelColor(4, 200), contentColor(200));
        QCOMPARE(full.pixelColor(4, 300), contentColor(300));
        QCOMPARE(full.pixelColor(4, 409), contentColor(409));
        QCOMPARE(full.pixelColor(4, 410), kFooterColor);
        QCOMPARE(full.pixelColor(4, full.height() - 1), kFooterColor);
    }

    void keepsStickyHeaderOnceAtTopWhenPrepending()
    {
        markshot::scroll::Stitcher stitcher(testConfig());
        constexpr int kHeader = 30;
        constexpr int kHeight = 200;
        constexpr int kContentPerFrame = kHeight - kHeader;

        // 先落底部内容,再反向滚回顶部,两帧顶部都带 30 行固定 header
        stitcher.pushFrame(stickyFrame(kHeader, 0, 120, kHeight));
        const markshot::scroll::StitchResult second =
            stitcher.pushFrame(stickyFrame(kHeader, 0, 0, kHeight));

        QCOMPARE(second.status, markshot::scroll::StitchStatus::Appended);
        QCOMPARE(second.edge, markshot::scroll::StitchEdge::Start);
        QCOMPARE(second.added, 120);

        // 输出应为顶部一份 header 加连续内容:30 header 行 + 120+170 内容行
        const QImage full = stitcher.fullImage();
        QCOMPARE(full.height(), kHeader + 120 + kContentPerFrame);
        QCOMPARE(full.pixelColor(4, 0), kHeaderColor);
        QCOMPARE(full.pixelColor(4, kHeader - 1), kHeaderColor);
        QCOMPARE(full.pixelColor(4, kHeader), contentColor(0));
        QCOMPARE(full.pixelColor(4, kHeader + 150), contentColor(150));
        QCOMPARE(full.pixelColor(4, full.height() - 1), contentColor(289));
    }

    void repeatedFrameReportsNoProgress()
    {
        markshot::scroll::Stitcher stitcher(testConfig());

        stitcher.pushFrame(verticalFrame(0, 80));
        stitcher.pushFrame(verticalFrame(60, 80));
        const int height = stitcher.stats().totalHeight;

        // 重复推入同一帧只应回报 NoProgress,长图高度保持不变
        const markshot::scroll::StitchResult repeat = stitcher.pushFrame(verticalFrame(60, 80));
        QCOMPARE(repeat.status, markshot::scroll::StitchStatus::NoProgress);
        QCOMPARE(stitcher.stats().totalHeight, height);
    }
};

/// @brief Main function for the Stitcher test suite.
/// @param argc Argument count.
/// @param argv Argument vector.
/// @return Standard C++ exit code.
QTEST_APPLESS_MAIN(StitcherTest)

#include "stitcher_test.moc"
