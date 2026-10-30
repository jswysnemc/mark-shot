#include "ruler/angle_draft.h"

#include <QLineF>

namespace markshot::ruler {

void AngleDraft::press(QVector<QPointF> &points, QPointF point)
{
    if (points.isEmpty()) {
        points = {point, point};
        return;
    }
    points.last() = point;
}

void AngleDraft::follow(QVector<QPointF> &points, QPointF point)
{
    if (!points.isEmpty()) {
        points.last() = point;
    }
}

AngleDraftStep AngleDraft::release(QVector<QPointF> &points, QPointF point, qreal minimumArm)
{
    follow(points, point);
    if (points.size() < 2) {
        return AngleDraftStep::Continue;
    }

    // 1. 跟随点离顶点或上一点太近时视为单击，继续跟随
    const QPointF previous = points.at(points.size() - 2);
    if (QLineF(points.first(), points.last()).length() < minimumArm
        || QLineF(previous, points.last()).length() < minimumArm) {
        return AngleDraftStep::Continue;
    }

    // 2. 定下第一条边后追加第二条边的跟随点，第二条边定下后三点齐全
    if (points.size() < 3) {
        points.append(points.last());
        return AngleDraftStep::Continue;
    }
    return AngleDraftStep::Commit;
}

}  // namespace markshot::ruler
