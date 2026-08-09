#include "window_hover_selection.h"

#include <QtTest/QtTest>

class WindowHoverSelectionTest final : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证真实负层级仍然高于缺少层级信息的窗口。
     * @return 无返回值。
     */
    void negativeZOrderBeatsMissingZOrder()
    {
        const QVector<markshot::WindowInfo> windows{
            {QRect(0, 0, 80, 80), std::nullopt},
            {QRect(10, 10, 40, 40), -5},
        };

        QCOMPARE(markshot::window_selection::topmostWindowRectAt(windows, QPoint(20, 20)),
                 std::optional<QRect>(QRect(10, 10, 40, 40)));
    }

    /**
     * 验证所有窗口缺少层级信息时选择面积较小的嵌套窗口。
     * @return 无返回值。
     */
    void missingZOrderFallsBackToSmallestArea()
    {
        const QVector<markshot::WindowInfo> windows{
            {QRect(0, 0, 100, 100), std::nullopt},
            {QRect(10, 10, 30, 30), std::nullopt},
        };

        QCOMPARE(markshot::window_selection::topmostWindowRectAt(windows, QPoint(20, 20)),
                 std::optional<QRect>(QRect(10, 10, 30, 30)));
    }

    /**
     * 验证没有窗口覆盖目标坐标时返回空值。
     * @return 无返回值。
     */
    void noMatchingWindowReturnsEmptyResult()
    {
        const QVector<markshot::WindowInfo> windows{
            {QRect(0, 0, 20, 20), 1},
        };

        QVERIFY(!markshot::window_selection::topmostWindowRectAt(windows, QPoint(30, 30)).has_value());
    }
};

QTEST_APPLESS_MAIN(WindowHoverSelectionTest)

#include "window_hover_selection_test.moc"
