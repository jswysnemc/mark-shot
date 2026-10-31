#include "selection_translate/selection_translate_placement.h"

#include <algorithm>
#include <cstdlib>
#include <optional>

namespace markshot::selection_translate {

namespace {

/// 弹层、浮条与锚点之间的间距
constexpr int kGap = 6;
/// 弹层、浮条与窗口边缘保持的最小留白
constexpr int kEdgeMargin = 8;
/// 浮条与障碍物之间至少留出的间距
constexpr int kObstaclePadding = 4;

/**
 * 把矩形平移到窗口留白以内，尺寸不变。
 * @param rect 待调整矩形。
 * @param bounds 截图窗口矩形。
 * @return 平移后的矩形。
 */
QRect clampedInto(QRect rect, const QRect &bounds)
{
    const int maxLeft = std::max(bounds.left() + kEdgeMargin, bounds.right() + 1 - kEdgeMargin - rect.width());
    const int maxTop = std::max(bounds.top() + kEdgeMargin, bounds.bottom() + 1 - kEdgeMargin - rect.height());
    rect.moveLeft(std::clamp(rect.left(), bounds.left() + kEdgeMargin, maxLeft));
    rect.moveTop(std::clamp(rect.top(), bounds.top() + kEdgeMargin, maxTop));
    return rect;
}

/**
 * 判断矩形是否与任一障碍物重叠，障碍物外扩 kObstaclePadding 留出呼吸空间。
 * @param rect 候选矩形。
 * @param obstacles 障碍物矩形。
 * @return 有重叠时返回 true。
 */
bool overlapsAny(const QRect &rect, const QVector<QRect> &obstacles)
{
    return std::any_of(obstacles.cbegin(), obstacles.cend(), [&rect](const QRect &obstacle) {
        return !obstacle.isEmpty()
            && rect.intersects(obstacle.adjusted(-kObstaclePadding, -kObstaclePadding, kObstaclePadding, kObstaclePadding));
    });
}

/**
 * 在译文区域上方、下方或内侧顶部的一行里找浮条位置。
 * 先试与区域右对齐，允许平移时再贴到各障碍物的左右两侧，离右对齐位置近的优先。
 * @param region 译文区域在截图窗口中的矩形。
 * @param top 这一行的顶边。
 * @param alignedLeft 右对齐时浮条的左边。
 * @param barSize 浮条尺寸。
 * @param bounds 截图窗口矩形。
 * @param obstacles 需要避让的工具栏和面板矩形。
 * @param slide 是否允许横向平移。
 * @param outside 是否要求浮条不压住区域，内侧顶部一行为 false。
 * @return 找到时返回浮条矩形，这一行放不下时返回空。
 */
std::optional<QRect> placeInRow(const QRect &region, int top, int alignedLeft, const QSize &barSize,
                                const QRect &bounds, const QVector<QRect> &obstacles, bool slide, bool outside)
{
    // 1. 候选左边：右对齐位置，以及紧贴每个障碍物左右两侧的位置
    QVector<int> lefts{alignedLeft};
    if (slide) {
        for (const QRect &obstacle : obstacles) {
            if (!obstacle.isEmpty()) {
                lefts.append(obstacle.right() + 1 + kObstaclePadding);
                lefts.append(obstacle.left() - kObstaclePadding - barSize.width());
            }
        }
        std::stable_sort(lefts.begin() + 1, lefts.end(), [alignedLeft](int left, int right) {
            return std::abs(left - alignedLeft) < std::abs(right - alignedLeft);
        });
    }

    // 2. 平移进窗口后不能压住区域和障碍物，还要与区域横向重叠，看得出浮条属于这块译文
    for (int left : lefts) {
        const QRect candidate = clampedInto(QRect(QPoint(left, top), barSize), bounds);
        const bool besideRegion = candidate.left() <= region.right() && candidate.right() >= region.left();
        if (besideRegion && (!outside || !candidate.intersects(region)) && !overlapsAny(candidate, obstacles)) {
            return candidate;
        }
    }
    return std::nullopt;
}

}  // namespace

QRect languagePopupGeometry(const QRect &anchor, bool verticalToolbar, const QSize &popupSize, const QRect &bounds)
{
    QRect popup(QPoint(0, 0), popupSize);
    if (verticalToolbar) {
        // 1. 竖向工具栏：按钮在窗口右半边时向左弹出，否则向右弹出，顶边与按钮对齐
        const bool openLeft = anchor.center().x() > bounds.center().x();
        popup.moveLeft(openLeft ? anchor.left() - kGap - popupSize.width() : anchor.right() + 1 + kGap);
        popup.moveTop(anchor.top());
    } else {
        // 2. 横向工具栏：按钮在窗口下半边时向上弹出，否则向下弹出，右边与按钮对齐
        const bool openUp = anchor.center().y() > bounds.center().y();
        popup.moveTop(openUp ? anchor.top() - kGap - popupSize.height() : anchor.bottom() + 1 + kGap);
        popup.moveLeft(anchor.right() + 1 - popupSize.width());
    }
    return clampedInto(popup, bounds);
}

QRect resultBarGeometry(const QRect &region, const QSize &barSize, const QRect &bounds,
                        const QVector<QRect> &obstacles)
{
    // 1. 先在区域上方、下方右对齐摆放，都被挡住时再在两行里横向平移避开障碍物
    const int outsideLeft = region.right() + 1 - barSize.width();
    for (bool slide : {false, true}) {
        for (int top : {region.top() - kGap - barSize.height(), region.bottom() + 1 + kGap}) {
            if (const std::optional<QRect> placed =
                    placeInRow(region, top, outsideLeft, barSize, bounds, obstacles, slide, true)) {
                return *placed;
            }
        }
    }

    // 2. 外侧都放不下时放进区域右上角内侧，只遮住译文的一角，同样避开工具栏
    const int insideLeft = outsideLeft - kGap;
    const int insideTop = region.top() + kGap;
    if (const std::optional<QRect> placed =
            placeInRow(region, insideTop, insideLeft, barSize, bounds, obstacles, true, false)) {
        return *placed;
    }
    return clampedInto(QRect(QPoint(insideLeft, insideTop), barSize), bounds);
}

}  // namespace markshot::selection_translate
