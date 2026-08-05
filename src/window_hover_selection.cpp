#include "window_hover_selection.h"

#include <limits>

namespace markshot::window_selection {

std::optional<QRect> topmostWindowRectAt(const QVector<WindowInfo> &windows,
                                         const QPoint &point)
{
    QVector<const WindowInfo *> candidates;
    bool hasZOrder = false;

    // 1. 收集覆盖目标坐标的窗口，并判断是否能够使用层级信息
    for (const WindowInfo &window : windows) {
        if (!window.rect.contains(point)) {
            continue;
        }
        candidates.append(&window);
        hasZOrder = hasZOrder || window.zOrder.has_value();
    }

    if (candidates.isEmpty()) {
        return std::nullopt;
    }

    const WindowInfo *best = candidates.first();
    if (hasZOrder) {
        // 2. 存在层级信息时，将缺少层级的窗口统一视为最底层
        const int bottomZOrder = std::numeric_limits<int>::min();
        for (const WindowInfo *candidate : std::as_const(candidates)) {
            if (candidate->zOrder.value_or(bottomZOrder) > best->zOrder.value_or(bottomZOrder)) {
                best = candidate;
            }
        }
    } else {
        // 3. 完全缺少层级信息时，使用较小面积作为窗口嵌套关系的回退判断
        for (const WindowInfo *candidate : std::as_const(candidates)) {
            const qint64 candidateArea = static_cast<qint64>(candidate->rect.width())
                * candidate->rect.height();
            const qint64 bestArea = static_cast<qint64>(best->rect.width())
                * best->rect.height();
            if (candidateArea < bestArea) {
                best = candidate;
            }
        }
    }

    return best->rect;
}

} // namespace markshot::window_selection
