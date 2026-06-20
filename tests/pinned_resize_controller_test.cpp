#include "pinned_window/pinned_resize_controller.h"

#include <QtTest/QtTest>

class PinnedResizeControllerTest : public QObject {
    Q_OBJECT

private slots:
    void hitTestFindsCornersBeforeEdges()
    {
        const QRectF rect(0, 0, 200, 100);

        QCOMPARE(markshot::shot::pinnedResizeDirectionAt(rect, QPointF(2, 2), 8),
                 markshot::shot::PinnedResizeDirection::TopLeft);
        QCOMPARE(markshot::shot::pinnedResizeDirectionAt(rect, QPointF(198, 2), 8),
                 markshot::shot::PinnedResizeDirection::TopRight);
        QCOMPARE(markshot::shot::pinnedResizeDirectionAt(rect, QPointF(2, 98), 8),
                 markshot::shot::PinnedResizeDirection::BottomLeft);
        QCOMPARE(markshot::shot::pinnedResizeDirectionAt(rect, QPointF(198, 98), 8),
                 markshot::shot::PinnedResizeDirection::BottomRight);
    }

    void hitTestFindsSingleEdges()
    {
        const QRectF rect(0, 0, 200, 100);

        QCOMPARE(markshot::shot::pinnedResizeDirectionAt(rect, QPointF(2, 50), 8),
                 markshot::shot::PinnedResizeDirection::Left);
        QCOMPARE(markshot::shot::pinnedResizeDirectionAt(rect, QPointF(198, 50), 8),
                 markshot::shot::PinnedResizeDirection::Right);
        QCOMPARE(markshot::shot::pinnedResizeDirectionAt(rect, QPointF(100, 2), 8),
                 markshot::shot::PinnedResizeDirection::Top);
        QCOMPARE(markshot::shot::pinnedResizeDirectionAt(rect, QPointF(100, 98), 8),
                 markshot::shot::PinnedResizeDirection::Bottom);
        QCOMPARE(markshot::shot::pinnedResizeDirectionAt(rect, QPointF(100, 50), 8),
                 markshot::shot::PinnedResizeDirection::None);
    }

    void directionEdgeFlagsMatchCorners()
    {
        QCOMPARE(markshot::shot::pinnedResizeDirectionIncludesLeft(markshot::shot::PinnedResizeDirection::TopLeft), true);
        QCOMPARE(markshot::shot::pinnedResizeDirectionIncludesTop(markshot::shot::PinnedResizeDirection::TopLeft), true);
        QCOMPARE(markshot::shot::pinnedResizeDirectionIncludesRight(markshot::shot::PinnedResizeDirection::TopLeft), false);
        QCOMPARE(markshot::shot::pinnedResizeDirectionIncludesBottom(markshot::shot::PinnedResizeDirection::TopLeft), false);

        QCOMPARE(markshot::shot::pinnedResizeDirectionIncludesRight(markshot::shot::PinnedResizeDirection::BottomRight), true);
        QCOMPARE(markshot::shot::pinnedResizeDirectionIncludesBottom(markshot::shot::PinnedResizeDirection::BottomRight), true);
        QCOMPARE(markshot::shot::pinnedResizeDirectionIncludesLeft(markshot::shot::PinnedResizeDirection::BottomRight), false);
        QCOMPARE(markshot::shot::pinnedResizeDirectionIncludesTop(markshot::shot::PinnedResizeDirection::BottomRight), false);
    }

    void screenEdgeDirectionLetsMoveTakePriority()
    {
        const QRect screen(0, 0, 1920, 1080);

        QCOMPARE(markshot::shot::pinnedResizeDirectionTouchesScreenEdge(
                     markshot::shot::PinnedResizeDirection::Left, QRect(-30, 100, 240, 120), screen),
                 true);
        QCOMPARE(markshot::shot::pinnedResizeDirectionTouchesScreenEdge(
                     markshot::shot::PinnedResizeDirection::Top, QRect(100, -20, 240, 120), screen),
                 true);
        QCOMPARE(markshot::shot::pinnedResizeDirectionTouchesScreenEdge(
                     markshot::shot::PinnedResizeDirection::Right, QRect(100, 100, 240, 120), screen),
                 false);
    }

    void rightEdgeDragPreservesAspectRatio()
    {
        const QRect start(100, 80, 200, 100);
        const markshot::shot::PinnedResizeDragState state =
            markshot::shot::beginPinnedResizeDrag(markshot::shot::PinnedResizeDirection::Right,
                                                  start,
                                                  QPoint(300, 130));

        const QRect resized = markshot::shot::pinnedResizeGeometry(state, QPoint(360, 130), QSize(24, 24));

        QCOMPARE(resized.topLeft(), QPoint(100, 65));
        QCOMPARE(resized.size(), QSize(260, 130));
    }

    void topLeftDragKeepsOppositeCornerFixed()
    {
        const QRect start(100, 80, 200, 100);
        const markshot::shot::PinnedResizeDragState state =
            markshot::shot::beginPinnedResizeDrag(markshot::shot::PinnedResizeDirection::TopLeft,
                                                  start,
                                                  QPoint(100, 80));

        const QRect resized = markshot::shot::pinnedResizeGeometry(state, QPoint(70, 60), QSize(24, 24));

        QCOMPARE(resized.topLeft(), QPoint(60, 60));
        QCOMPARE(resized.size(), QSize(240, 120));
        QCOMPARE(resized.right(), start.right());
        QCOMPARE(resized.bottom(), start.bottom());
    }

    void dragRespectsMinimumSize()
    {
        const QRect start(100, 80, 200, 100);
        const markshot::shot::PinnedResizeDragState state =
            markshot::shot::beginPinnedResizeDrag(markshot::shot::PinnedResizeDirection::BottomRight,
                                                  start,
                                                  QPoint(300, 180));

        const QRect resized = markshot::shot::pinnedResizeGeometry(state, QPoint(110, 90), QSize(60, 60));

        QCOMPARE(resized.topLeft(), start.topLeft());
        QCOMPARE(resized.size(), QSize(120, 60));
    }
};

QTEST_MAIN(PinnedResizeControllerTest)
#include "pinned_resize_controller_test.moc"
