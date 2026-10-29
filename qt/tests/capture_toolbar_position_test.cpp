#include "capture_toolbar_position.h"
#include "toolbar_dock_layout.h"

#include <QJsonObject>
#include <QtTest/QtTest>

namespace {

constexpr int kEdge = 12;
constexpr int kSpacing = 10;
using Edge = markshot::ToolbarDockEdge;

}  // namespace

class CaptureToolbarPositionTest : public QObject {
    Q_OBJECT

private slots:
    /** 验证规范名称与常见别名都能解析，非法值返回空，无参数和返回值。 */
    void parsePositionAliases()
    {
        using markshot::CaptureToolbarPosition;
        QCOMPARE(markshot::captureToolbarPositionFromText(QStringLiteral("selection")).value(),
                 CaptureToolbarPosition::FollowSelection);
        QCOMPARE(markshot::captureToolbarPositionFromText(QStringLiteral("follow-selection")).value(),
                 CaptureToolbarPosition::FollowSelection);
        QCOMPARE(markshot::captureToolbarPositionFromText(QStringLiteral("Bottom")).value(),
                 CaptureToolbarPosition::ScreenBottom);
        QCOMPARE(markshot::captureToolbarPositionFromText(QStringLiteral("screen_bottom")).value(),
                 CaptureToolbarPosition::ScreenBottom);
        QCOMPARE(markshot::captureToolbarPositionFromText(QStringLiteral("screen right")).value(),
                 CaptureToolbarPosition::ScreenRight);
        QCOMPARE(markshot::captureToolbarPositionFromText(QStringLiteral("Top")).value(),
                 CaptureToolbarPosition::ScreenTop);
        QCOMPARE(markshot::captureToolbarPositionFromText(QStringLiteral("screen-top")).value(),
                 CaptureToolbarPosition::ScreenTop);
        QVERIFY(!markshot::captureToolbarPositionFromText(QStringLiteral("left")).has_value());
        QVERIFY(!markshot::captureToolbarPositionFromText(QString()).has_value());
    }

    /** 验证配置根对象读取 capture.toolbarPosition，缺失或非法时回落到跟随选区，无参数和返回值。 */
    void configRootReadsCaptureValue()
    {
        QJsonObject capture;
        capture.insert(QStringLiteral("toolbarPosition"), QStringLiteral("right"));
        QJsonObject root;
        root.insert(QStringLiteral("capture"), capture);
        QCOMPARE(markshot::captureToolbarPositionFromConfigRoot(root), markshot::CaptureToolbarPosition::ScreenRight);

        capture.insert(QStringLiteral("toolbarPosition"), QStringLiteral("middle"));
        root.insert(QStringLiteral("capture"), capture);
        QCOMPARE(markshot::captureToolbarPositionFromConfigRoot(root),
                 markshot::CaptureToolbarPosition::FollowSelection);
        QCOMPARE(markshot::captureToolbarPositionFromConfigRoot(QJsonObject()),
                 markshot::CaptureToolbarPosition::FollowSelection);
    }

    /** 验证写入名称能被重新解析为同一摆放方式，无参数和返回值。 */
    void namesRoundTrip()
    {
        using markshot::CaptureToolbarPosition;
        for (CaptureToolbarPosition position : {CaptureToolbarPosition::FollowSelection,
                                                CaptureToolbarPosition::ScreenBottom,
                                                CaptureToolbarPosition::ScreenRight,
                                                CaptureToolbarPosition::ScreenTop}) {
            QCOMPARE(markshot::captureToolbarPositionFromText(markshot::captureToolbarPositionName(position)).value(),
                     position);
        }
    }

    /** 验证底部停靠一行放得下时两条工具栏整体居中、贴底且间距固定，无参数和返回值。 */
    void bottomDockCentersOneRow()
    {
        const QRect bounds(0, 0, 1000, 600);
        const auto layout = markshot::dockedToolbarLayout(Edge::Bottom, bounds, QSize(500, 40), QSize(300, 36),
                                                          kEdge, kSpacing);
        QCOMPARE(layout.toolbar, QRect(95, 548, 500, 40));
        QCOMPARE(layout.actionToolbar, QRect(605, 550, 300, 36));
        QCOMPARE(bounds.bottom() - layout.toolbar.bottom(), kEdge);
    }

    /** 验证底部宽度不足时副工具栏移到主工具栏上方并右对齐，两者不重叠，无参数和返回值。 */
    void bottomDockStacksWhenNarrow()
    {
        const auto layout = markshot::dockedToolbarLayout(Edge::Bottom, QRect(0, 0, 800, 600), QSize(500, 40),
                                                          QSize(300, 36), kEdge, kSpacing);
        QCOMPARE(layout.toolbar, QRect(150, 548, 500, 40));
        QCOMPARE(layout.actionToolbar, QRect(350, 502, 300, 36));
        QVERIFY(!layout.toolbar.intersects(layout.actionToolbar));
    }

    /** 验证顶部停靠一行放得下时两条工具栏整体居中、贴顶且间距固定，无参数和返回值。 */
    void topDockCentersOneRow()
    {
        const QRect bounds(0, 0, 1000, 600);
        const auto layout = markshot::dockedToolbarLayout(Edge::Top, bounds, QSize(500, 40), QSize(300, 36),
                                                          kEdge, kSpacing);
        QCOMPARE(layout.toolbar, QRect(95, 12, 500, 40));
        QCOMPARE(layout.actionToolbar, QRect(605, 14, 300, 36));
        QCOMPARE(layout.toolbar.top() - bounds.top(), kEdge);
    }

    /** 验证顶部宽度不足时主工具栏贴顶，副工具栏移到其下方并右对齐，两者不重叠，无参数和返回值。 */
    void topDockStacksWhenNarrow()
    {
        const auto layout = markshot::dockedToolbarLayout(Edge::Top, QRect(0, 0, 800, 600), QSize(500, 40),
                                                          QSize(300, 36), kEdge, kSpacing);
        QCOMPARE(layout.toolbar, QRect(150, 12, 500, 40));
        QCOMPARE(layout.actionToolbar, QRect(350, 62, 300, 36));
        QVERIFY(!layout.toolbar.intersects(layout.actionToolbar));
    }

    /** 验证右侧停靠一列放得下时两条工具栏整体垂直居中、贴右，无参数和返回值。 */
    void rightDockCentersOneColumn()
    {
        const QRect bounds(0, 0, 1000, 800);
        const auto layout = markshot::dockedToolbarLayout(Edge::Right, bounds, QSize(40, 500), QSize(36, 200),
                                                          kEdge, kSpacing);
        QCOMPARE(layout.toolbar, QRect(948, 45, 40, 500));
        QCOMPARE(layout.actionToolbar, QRect(950, 555, 36, 200));
        QCOMPARE(bounds.right() - layout.toolbar.right(), kEdge);
    }

    /** 验证右侧高度不足时副工具栏移到主工具栏左侧并底对齐，两者不重叠，无参数和返回值。 */
    void rightDockSideBySideWhenShort()
    {
        const auto layout = markshot::dockedToolbarLayout(Edge::Right, QRect(0, 0, 1000, 400), QSize(40, 300),
                                                          QSize(36, 200), kEdge, kSpacing);
        QCOMPARE(layout.toolbar, QRect(948, 50, 40, 300));
        QCOMPARE(layout.actionToolbar, QRect(902, 150, 36, 200));
        QVERIFY(!layout.toolbar.intersects(layout.actionToolbar));
    }

    /** 验证没有副工具栏时只摆放主工具栏，副工具栏结果为空，无参数和返回值。 */
    void dockWithoutActionToolbar()
    {
        const auto layout = markshot::dockedToolbarLayout(Edge::Bottom, QRect(0, 0, 1000, 600), QSize(600, 40),
                                                          QSize(), kEdge, kSpacing);
        QCOMPARE(layout.toolbar, QRect(200, 548, 600, 40));
        QVERIFY(layout.actionToolbar.isNull());
    }

    /** 验证工具栏比可用区域还大时贴靠左上边距，不移出窗口原点，无参数和返回值。 */
    void oversizedToolbarStaysAtOrigin()
    {
        const auto layout = markshot::dockedToolbarLayout(Edge::Bottom, QRect(0, 0, 300, 200), QSize(500, 40),
                                                          QSize(), kEdge, kSpacing);
        QCOMPARE(layout.toolbar.topLeft(), QPoint(kEdge, 148));
    }
};

QTEST_GUILESS_MAIN(CaptureToolbarPositionTest)

#include "capture_toolbar_position_test.moc"
