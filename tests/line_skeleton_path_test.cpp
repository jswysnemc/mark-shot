#include "line_skeleton_path.h"

#include <QtTest/QtTest>

#include <cmath>
#include <optional>

/// @brief 验证直线类工具骨架路径的几何计算。
class LineSkeletonPathTest : public QObject
{
    Q_OBJECT

private slots:
    /**
     * 验证骨架路由点保持起点、骨架点、终点顺序。
     * @return 无返回值。
     */
    void routePointsKeepEndpointStorageCompatible()
    {
        const QVector<QPointF> points = {
            QPointF(0.0, 0.0),
            QPointF(10.0, 0.0),
            QPointF(4.0, 6.0),
            QPointF(7.0, 3.0),
        };

        const QVector<QPointF> routePoints = markshot::shot::lineSkeletonRoutePoints(points);

        QCOMPARE(routePoints.size(), 4);
        QCOMPARE(routePoints.at(0), points.at(0));
        QCOMPARE(routePoints.at(1), points.at(2));
        QCOMPARE(routePoints.at(2), points.at(3));
        QCOMPARE(routePoints.at(3), points.at(1));
    }

    /**
     * 验证直线段可在路径附近插入第一个骨架点。
     * @return 无返回值。
     */
    void insertionOnStraightLineUsesFirstSkeletonIndex()
    {
        const QVector<QPointF> points = {
            QPointF(0.0, 0.0),
            QPointF(10.0, 0.0),
        };

        const std::optional<markshot::shot::LineSkeletonInsertion> insertion =
            markshot::shot::lineSkeletonInsertionAt(points, QPointF(5.0, 0.5), 1.0);

        QVERIFY(insertion.has_value());
        QCOMPARE(insertion->pointIndex, markshot::shot::kLineSkeletonFirstPointIndex);
        QVERIFY(std::abs(insertion->point.x() - 5.0) < 0.001);
        QVERIFY(std::abs(insertion->point.y()) < 0.001);
    }

    /**
     * 验证只有中间骨架点允许删除。
     * @return 无返回值。
     */
    void onlyInternalSkeletonPointsCanBeRemoved()
    {
        const QVector<QPointF> points = {
            QPointF(0.0, 0.0),
            QPointF(10.0, 0.0),
            QPointF(5.0, 5.0),
        };

        QVERIFY(!markshot::shot::canRemoveLineSkeletonPoint(points, 0));
        QVERIFY(!markshot::shot::canRemoveLineSkeletonPoint(points, 1));
        QVERIFY(markshot::shot::canRemoveLineSkeletonPoint(points, 2));
    }

    /**
     * 验证单骨架点曲线会生成多点中心线采样。
     * @return 无返回值。
     */
    void singleSkeletonPointProducesCurvedSamples()
    {
        const QVector<QPointF> points = {
            QPointF(0.0, 0.0),
            QPointF(10.0, 0.0),
            QPointF(5.0, 8.0),
        };

        const QVector<QPointF> samples = markshot::shot::sampledLineSkeletonCenterLine(points, 12);

        QVERIFY(samples.size() > 2);
        QCOMPARE(samples.first(), points.first());
        QCOMPARE(samples.last(), points.at(1));
    }
};

/// @brief 运行 LineSkeletonPathTest 测试套件。
/// @param argc 命令行参数数量。
/// @param argv 命令行参数数组。
/// @return 测试进程退出状态。
QTEST_APPLESS_MAIN(LineSkeletonPathTest)

#include "line_skeleton_path_test.moc"
