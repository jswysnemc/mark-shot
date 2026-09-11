#include "selection_adjustment.h"
#include "selection_cursor_nudge.h"

#include <QtTest/QtTest>

using namespace markshot::shot;
using Handle = types::SelectionDrag;

class SelectionAdjustmentTest : public QObject {
    Q_OBJECT

private slots:
    /// @brief 验证选区完成后的方向键按一个像素移动，Shift 按十个像素移动
    /// @return 无返回值
    void movesWithFineAndCoarseKeys()
    {
        const QRectF before(20, 30, 80, 60);
        const QPointF pointer(40, 40);
        QCOMPARE(adjustedSelectionRect(before, Handle::Move, pointer,
                                       pointer + selectionCursorNudgeDelta(Qt::Key_Right, false),
                                       QSize(200, 150)), QRectF(21, 30, 80, 60));
        QCOMPARE(adjustedSelectionRect(before, Handle::Move, pointer,
                                       pointer + selectionCursorNudgeDelta(Qt::Key_Up, true),
                                       QSize(200, 150)), QRectF(20, 20, 80, 60));
    }

    /// @brief 验证八个边角均可用方向键调整且保留另一侧边界
    /// @return 无返回值
    void resizesEveryHandle()
    {
        const QRectF before(20, 30, 80, 60);
        const QSize imageSize(200, 150);
        const Handle handles[] = {Handle::Left, Handle::Right, Handle::Top, Handle::Bottom,
                                   Handle::TopLeft, Handle::TopRight,
                                   Handle::BottomLeft, Handle::BottomRight};
        const QRectF expected[] = {{21, 30, 79, 60}, {20, 30, 81, 60},
                                    {20, 31, 80, 59}, {20, 30, 80, 61},
                                    {21, 31, 79, 59}, {20, 31, 81, 59},
                                    {21, 30, 79, 61}, {20, 30, 81, 61}};
        for (int index = 0; index < 8; ++index) {
            const QPointF point = selectionAdjustmentPoint(before, handles[index], QPointF(23, 33));
            QCOMPARE(adjustedSelectionRect(before, handles[index], point,
                                           point + QPointF(1, 1), imageSize), expected[index]);
        }
    }

    /// @brief 验证连续微调和鼠标拖动使用相同起点时不会重置已移动的距离
    /// @return 无返回值
    void continuesAnActiveDrag()
    {
        const QRectF before(20, 30, 80, 60);
        const QPointF anchor(100, 90);
        QCOMPARE(adjustedSelectionRect(before, Handle::BottomRight, anchor,
                                       QPointF(107, 96), QSize(200, 150)), QRectF(20, 30, 87, 66));
        QCOMPARE(adjustedSelectionRect(before, Handle::BottomRight, anchor,
                                       QPointF(108, 96), QSize(200, 150)), QRectF(20, 30, 88, 66));
    }

    /// @brief 验证拖到边界或最小尺寸后可立即反向微调并继续拖动
    /// @return 无返回值
    void reversesFromClampedDrag()
    {
        const QSize size(200, 150);
        const QRectF original(20, 30, 80, 60);
        const QPointF pointer(199, 40);
        const QRectF clamped = adjustedSelectionRect(original, Handle::Move, QPointF(40, 40), pointer, size);
        QCOMPARE(clamped, QRectF(120, 30, 80, 60));
        const QPointF nudgedPointer = pointer - QPointF(1, 0);
        const QRectF nudged = adjustedSelectionRect(clamped, Handle::Move, pointer, nudgedPointer, size);
        QCOMPARE(nudged, QRectF(119, 30, 80, 60));
        QCOMPARE(adjustedSelectionRect(nudged, Handle::Move, nudgedPointer,
                                       nudgedPointer - QPointF(5, 0), size), QRectF(114, 30, 80, 60));

        const QRectF minimum = adjustedSelectionRect(original, Handle::Left, {}, QPointF(199, 40), size);
        QCOMPARE(minimum, QRectF(92, 30, 8, 60));
        const QPointF edge = selectionAdjustmentPoint(minimum, Handle::Left, QPointF(199, 40));
        QCOMPARE(adjustedSelectionRect(minimum, Handle::Left, edge, edge - QPointF(1, 0), size),
                 QRectF(91, 30, 9, 60));
    }

    /// @brief 验证边界夹紧、最小尺寸和无效输入
    /// @return 无返回值
    void clampsBoundsAndMinimumSize()
    {
        const QRectF before(20, 30, 80, 60);
        QCOMPARE(adjustedSelectionRect(before, Handle::Move, QPointF(40, 40),
                                       QPointF(400, -100), QSize(200, 150)), QRectF(120, 0, 80, 60));
        QCOMPARE(adjustedSelectionRect(before, Handle::TopLeft, {}, QPointF(200, 200),
                                       QSize(200, 150)), QRectF(92, 82, 8, 8));
        QCOMPARE(adjustedSelectionRect(before, Handle::BottomRight, {}, QPointF(999, 999),
                                       QSize(200, 150)), QRectF(20, 30, 180, 120));
        QCOMPARE(adjustedSelectionRect(before, Handle::Left, {}, {}, QSize()), before);
        QCOMPARE(adjustedSelectionRect(before, Handle::None, {}, {}, QSize(200, 150)), before);
    }
};

QTEST_APPLESS_MAIN(SelectionAdjustmentTest)

#include "selection_adjustment_test.moc"
