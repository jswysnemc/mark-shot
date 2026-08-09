#include "scroll/stitcher_fixed_regions.h"

#include <QImage>
#include <QPainter>

#include <QtTest/QtTest>

namespace {

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
QImage stickyFrame(int headerRows, int footerRows, int firstContent, int height = 240, int width = 200)
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

/// @brief FixedRegionDetector 的单元测试。
class StitcherFixedRegionsTest : public QObject
{
    Q_OBJECT

private slots:
    void detectsHeaderAndFooterBands()
    {
        markshot::scroll::FixedRegionDetector detector;

        const QImage prev = stickyFrame(24, 16, 0);
        const QImage cur = stickyFrame(24, 16, 37);
        const auto obs = detector.observe(prev, cur, 37);

        QVERIFY(obs.valid);
        QCOMPARE(obs.header, 24);
        QCOMPARE(obs.footer, 16);
        QCOMPARE(detector.matchTopIgnore(), 24);
        QCOMPARE(detector.matchBottomIgnore(), 16);
    }

    void ignoresSmallScrollOffsets()
    {
        markshot::scroll::FixedRegionDetector detector;

        // 位移不足时两帧几乎重合,不能作为固定带的证据
        const auto obs = detector.observe(stickyFrame(24, 16, 0), stickyFrame(24, 16, 37), 5);
        QVERIFY(!obs.valid);
        QCOMPARE(detector.matchTopIgnore(), 0);
        QCOMPARE(detector.matchBottomIgnore(), 0);
    }

    void rejectsMismatchedFrameSizes()
    {
        markshot::scroll::FixedRegionDetector detector;

        const auto obs = detector.observe(stickyFrame(24, 16, 0, 240), stickyFrame(24, 16, 37, 220), 37);
        QVERIFY(!obs.valid);
    }

    void lowersCommittedEstimateImmediately()
    {
        markshot::scroll::FixedRegionDetector detector;

        detector.observe(stickyFrame(24, 16, 0), stickyFrame(24, 16, 37), 37);
        QCOMPARE(detector.matchTopIgnore(), 24);

        // 后续观察发现固定带更矮时立即下调,防止把偶然相同的内容行算进去
        detector.observe(stickyFrame(10, 16, 74), stickyFrame(10, 16, 111), 37);
        QCOMPARE(detector.matchTopIgnore(), 10);
        QCOMPARE(detector.matchBottomIgnore(), 16);
    }

    void raisesCommittedEstimateOnlyAfterPersistence()
    {
        markshot::scroll::FixedRegionDetector detector;

        detector.observe(stickyFrame(10, 0, 0), stickyFrame(10, 0, 37), 37);
        QCOMPARE(detector.matchTopIgnore(), 10);

        // 更高的候选需要连续两次观察确认才会上调
        detector.observe(stickyFrame(24, 0, 74), stickyFrame(24, 0, 111), 37);
        QCOMPARE(detector.matchTopIgnore(), 10);
        detector.observe(stickyFrame(24, 0, 148), stickyFrame(24, 0, 185), 37);
        QCOMPARE(detector.matchTopIgnore(), 24);
    }

    void resetClearsAllState()
    {
        markshot::scroll::FixedRegionDetector detector;

        detector.observe(stickyFrame(24, 16, 0), stickyFrame(24, 16, 37), 37);
        detector.reset();
        QCOMPARE(detector.matchTopIgnore(), 0);
        QCOMPARE(detector.matchBottomIgnore(), 0);
    }
};

/// @brief 固定区域检测器测试套件入口。
/// @param argc 参数个数。
/// @param argv 参数数组。
/// @return 标准进程退出码。
QTEST_APPLESS_MAIN(StitcherFixedRegionsTest)

#include "stitcher_fixed_regions_test.moc"
