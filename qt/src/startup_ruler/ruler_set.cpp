#include "startup_ruler/ruler_set.h"

#include <cmath>

namespace markshot::startup_ruler {

bool rulerMeasures(const QLineF &ruler)
{
    return std::abs(ruler.dx()) >= 1.0 || std::abs(ruler.dy()) >= 1.0;
}

bool dropUnmeasuredLastRuler(QVector<QLineF> &rulers)
{
    if (rulers.isEmpty() || rulerMeasures(rulers.last())) {
        return false;
    }
    rulers.removeLast();
    return true;
}

}  // namespace markshot::startup_ruler
