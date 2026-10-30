#pragma once

#include <QPointF>
#include <QVector>

namespace markshot::ruler {

/// 角度尺草稿在一次松开后的去向
enum class AngleDraftStep {
    // 继续定点：最后一点跟随指针，等待下一次按下
    Continue,
    // 三点齐全，可以提交
    Commit,
};

/**
 * 角度尺三次定点：按下放顶点并拖出第一条边，松开后移动指针，再次单击放下第二条边的端点。
 * 草稿的最后一点始终是正在跟随指针的点。
 */
class AngleDraft {
public:
    /**
     * 处理一次按下：草稿为空时在指针处放下顶点并拉出第一条边，否则把跟随点移到指针处。
     * @param points 草稿点列表，原地修改。
     * @param point 指针位置，图像像素坐标。
     * @return 无返回值。
     */
    static void press(QVector<QPointF> &points, QPointF point);

    /**
     * 跟随指针移动最后一点。
     * @param points 草稿点列表，原地修改。
     * @param point 指针位置，图像像素坐标。
     * @return 无返回值。
     */
    static void follow(QVector<QPointF> &points, QPointF point);

    /**
     * 处理一次松开：跟随点离顶点和上一点都够远时定下该点，不够远时继续跟随，避免单击产生零长度的边。
     * @param points 草稿点列表，原地修改；定下第一条边后追加第二条边的跟随点。
     * @param point 指针位置，图像像素坐标。
     * @param minimumArm 一条边的最短长度，图像像素。
     * @return 三点齐全时返回 Commit，否则返回 Continue。
     */
    static AngleDraftStep release(QVector<QPointF> &points, QPointF point, qreal minimumArm);
};

}  // namespace markshot::ruler
