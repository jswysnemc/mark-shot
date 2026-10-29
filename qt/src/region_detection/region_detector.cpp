#include "region_detection/region_detector.h"

#include "region_detection/region_detection_internal.h"

#include <algorithm>
#include <cmath>
#include <utility>

namespace markshot::region_detection {
namespace {

/**
 * 可靠边界先进入优先级，再按尺度选择。
 * @param candidate 候选。
 * @return 越小越优先。
 */
float candidateRank(const RegionCandidate &candidate)
{
    const float tier = candidate.quality >= 0.80f ? 0.0f : 10000.0f;
    return tier + std::sqrt(std::sqrt(static_cast<float>(rectArea(candidate.rect))))
        / std::pow(qMax(candidate.quality, 0.1f), 2.0f);
}

/**
 * 判断候选是否来自合成器窗口注入。
 * @param candidate 候选。
 * @return 含 window 来源时返回 true。
 */
bool isWindowCandidate(const RegionCandidate &candidate)
{
    return candidate.sources.contains(QStringLiteral("window"));
}

/// 图像候选越出所在窗口时允许的边界误差，覆盖阴影与描边的抗锯齿像素
constexpr int kWindowContainmentTolerance = 4;

/**
 * 判断图像候选是否落在合成器窗口内。
 * @param window 合成器窗口矩形。
 * @param candidate 图像分析得到的矩形。
 * @return 候选未越出窗口边界（含容差）时返回 true。
 */
bool insideWindow(QRect window, QRect candidate)
{
    const QRect relaxed = window.adjusted(-kWindowContainmentTolerance, -kWindowContainmentTolerance,
                                          kWindowContainmentTolerance, kWindowContainmentTolerance);
    return containsRect(relaxed, candidate);
}

} // namespace

/**
 * 计算矩形面积。
 * @param rect 目标矩形。
 * @return 面积。
 */
qint64 rectArea(QRect rect)
{
    return static_cast<qint64>(rect.width()) * rect.height();
}

/**
 * 判断父矩形是否完整包含子矩形。
 * @param parent 外层矩形。
 * @param child 内层矩形。
 * @return 完整包含时返回 true。
 */
bool containsRect(QRect parent, QRect child)
{
    return parent.x() <= child.x()
        && parent.y() <= child.y()
        && parent.x() + parent.width() >= child.x() + child.width()
        && parent.y() + parent.height() >= child.y() + child.height();
}

/**
 * 从冻结截图构造区域检测器。
 * @param image 冻结截图。
 */
RegionDetector::RegionDetector(const QImage &image)
{
    m_imageWidth = image.width();
    m_imageHeight = image.height();
    m_candidates = detectRegions(image);
    std::sort(m_candidates.begin(), m_candidates.end(),
              [](const RegionCandidate &a, const RegionCandidate &b) {
                  if (a.quality != b.quality) {
                      return a.quality > b.quality;
                  }
                  return rectArea(a.rect) < rectArea(b.rect);
              });
    if (m_candidates.size() > kMaxCandidates) {
        m_candidates.resize(kMaxCandidates);
    }
    std::sort(m_candidates.begin(), m_candidates.end(),
              [](const RegionCandidate &a, const RegionCandidate &b) {
                  return rectArea(a.rect) < rectArea(b.rect);
              });
}

/**
 * 用已有候选构造检测器，供测试注入固定层级。
 * @param imageWidth 图像宽度。
 * @param imageHeight 图像高度。
 * @param candidates 候选列表。
 */
RegionDetector::RegionDetector(int imageWidth, int imageHeight, QVector<RegionCandidate> candidates)
    : m_imageWidth(imageWidth)
    , m_imageHeight(imageHeight)
    , m_candidates(std::move(candidates))
{
}

/**
 * 返回图像宽度。
 * @return 宽度。
 */
int RegionDetector::imageWidth() const
{
    return m_imageWidth;
}

/**
 * 返回图像高度。
 * @return 高度。
 */
int RegionDetector::imageHeight() const
{
    return m_imageHeight;
}

/**
 * 返回已缓存候选。
 * @return 候选列表。
 */
const QVector<RegionCandidate> &RegionDetector::candidates() const
{
    return m_candidates;
}

/**
 * 返回整图边界。
 * @return 整图矩形。
 */
QRect RegionDetector::bounds() const
{
    return QRect(0, 0, m_imageWidth, m_imageHeight);
}

/**
 * 把合成器窗口矩形注入候选，供滚轮切换父层级；悬停仍优先 UI 控件。
 * @param windows 已换算到图像坐标的窗口矩形。
 * @return 无返回值。
 */
void RegionDetector::addWindowRects(const QVector<QRect> &windows)
{
    for (const QRect &window : windows) {
        const QRect clipped = window.intersected(bounds());
        if (clipped.width() < 8 || clipped.height() < 8) {
            continue;
        }
        RegionCandidate candidate;
        candidate.rect = clipped;
        candidate.quality = 0.92f;
        candidate.sides = {0.92f, 0.92f, 0.92f, 0.92f};
        candidate.sources.append(QStringLiteral("window"));
        mergeCandidate(&m_candidates, std::move(candidate), 2);
    }
    std::sort(m_candidates.begin(), m_candidates.end(),
              [](const RegionCandidate &a, const RegionCandidate &b) {
                  return rectArea(a.rect) < rectArea(b.rect);
              });
}

/**
 * 把归一化坐标转换成图像像素。
 * @param x 水平归一化坐标。
 * @param y 垂直归一化坐标。
 * @return 图像像素坐标。
 */
QPoint RegionDetector::pointFromNormalized(float x, float y) const
{
    const int px = static_cast<int>(qBound(0.0f, x, 1.0f) * static_cast<float>(m_imageWidth));
    const int py = static_cast<int>(qBound(0.0f, y, 1.0f) * static_cast<float>(m_imageHeight));
    return QPoint(qMin(px, qMax(0, m_imageWidth - 1)), qMin(py, qMax(0, m_imageHeight - 1)));
}

/**
 * 返回覆盖指定图像坐标的最佳候选；没有命中时回退到整图。
 * @param point 图像像素坐标。
 * @return 候选矩形。
 */
QRect RegionDetector::regionAt(QPoint point) const
{
    const RegionCandidate *bestUi = nullptr;
    const RegionCandidate *bestWindow = nullptr;
    float bestUiRank = 0.0f;
    float bestWindowRank = 0.0f;
    for (const RegionCandidate &candidate : m_candidates) {
        if (!candidate.rect.contains(point)) {
            continue;
        }
        const float rank = candidateRank(candidate);
        if (isWindowCandidate(candidate)) {
            if (!bestWindow || rank < bestWindowRank) {
                bestWindow = &candidate;
                bestWindowRank = rank;
            }
            continue;
        }
        if (!bestUi || rank < bestUiRank) {
            bestUi = &candidate;
            bestUiRank = rank;
        }
    }
    // 1. 【截图】【区域识别】越出所在窗口的图像候选来自深色内容、面板与壁纸连成一片，不能压过合成器窗口
    if (bestUi && bestWindow && !insideWindow(bestWindow->rect, bestUi->rect)) {
        bestUi = nullptr;
        bestUiRank = 0.0f;
        for (const RegionCandidate &candidate : m_candidates) {
            if (!candidate.rect.contains(point) || isWindowCandidate(candidate)
                || !insideWindow(bestWindow->rect, candidate.rect)) {
                continue;
            }
            const float rank = candidateRank(candidate);
            if (!bestUi || rank < bestUiRank) {
                bestUi = &candidate;
                bestUiRank = rank;
            }
        }
    }
    if (bestUi) {
        return bestUi->rect;
    }
    if (bestWindow) {
        return bestWindow->rect;
    }
    return bounds();
}

/**
 * 返回覆盖归一化坐标的最佳候选。
 * @param x 水平归一化坐标，范围 [0, 1]。
 * @param y 垂直归一化坐标，范围 [0, 1]。
 * @return 候选矩形。
 */
QRect RegionDetector::regionAtNormalized(float x, float y) const
{
    return regionAt(pointFromNormalized(x, y));
}

/**
 * 从小到大返回一条真正包含关系的链。
 * @param point 图像像素坐标。
 * @return 子区域到整图的矩形链。
 */
QVector<QRect> RegionDetector::regionsAt(QPoint point) const
{
    QRect current = regionAt(point);
    QVector<QRect> chain{current};
    while (true) {
        const RegionCandidate *child = nullptr;
        for (const RegionCandidate &candidate : m_candidates) {
            if (!candidate.rect.contains(point) || candidate.rect == chain.first()
                || !containsRect(chain.first(), candidate.rect)) {
                continue;
            }
            if (!child || rectArea(candidate.rect) > rectArea(child->rect)) {
                child = &candidate;
            }
        }
        if (!child) {
            break;
        }
        chain.prepend(child->rect);
    }
    while (true) {
        const RegionCandidate *parent = nullptr;
        for (const RegionCandidate &candidate : m_candidates) {
            if (!candidate.rect.contains(point) || candidate.rect == chain.last()
                || !containsRect(candidate.rect, chain.last())) {
                continue;
            }
            if (!parent || rectArea(candidate.rect) < rectArea(parent->rect)) {
                parent = &candidate;
            }
        }
        if (!parent) {
            break;
        }
        chain.append(parent->rect);
    }
    if (chain.last() != bounds()) {
        chain.append(bounds());
    }
    return chain;
}

/**
 * 根据指针位置更新当前候选。
 * @param detector 已缓存的区域检测器。
 * @param point 图像像素坐标。
 * @return 当前候选矩形。
 */
QRect SelectionCursor::hover(const RegionDetector &detector, QPoint point)
{
    const float tolerance = m_manualLevel ? 6.0f : 3.0f;
    if (m_current.has_value() && m_current->contains(point) && m_anchor.has_value()) {
        const float distance = std::hypot(static_cast<float>(point.x()) - static_cast<float>(m_anchor->x()),
                                          static_cast<float>(point.y()) - static_cast<float>(m_anchor->y()));
        if (distance <= tolerance) {
            return *m_current;
        }
    }
    const QRect rect = detector.regionAt(point);
    m_current = rect;
    m_anchor = QPointF(point);
    m_manualLevel = false;
    return rect;
}

/**
 * 正方向扩大到父区域，负方向缩小到子区域。
 * @param detector 已缓存的区域检测器。
 * @param point 图像像素坐标。
 * @param direction 正值扩大，负值缩小。
 * @return 切换后的候选矩形。
 */
QRect SelectionCursor::step(const RegionDetector &detector, QPoint point, int direction)
{
    const QRect current = hover(detector, point);
    QRect rect = current;
    if (direction > 0) {
        const RegionCandidate *parent = nullptr;
        for (const RegionCandidate &candidate : detector.candidates()) {
            if (!candidate.rect.contains(point) || candidate.rect == current
                || !containsRect(candidate.rect, current)) {
                continue;
            }
            if (!parent || rectArea(candidate.rect) < rectArea(parent->rect)) {
                parent = &candidate;
            }
        }
        rect = parent ? parent->rect : detector.bounds();
    } else if (direction < 0) {
        const RegionCandidate *child = nullptr;
        for (const RegionCandidate &candidate : detector.candidates()) {
            if (!candidate.rect.contains(point) || candidate.rect == current
                || !containsRect(current, candidate.rect)) {
                continue;
            }
            if (!child || rectArea(candidate.rect) > rectArea(child->rect)) {
                child = &candidate;
            }
        }
        rect = child ? child->rect : current;
    }
    m_current = rect;
    m_anchor = QPointF(point);
    m_manualLevel = true;
    return rect;
}

/**
 * 判断当前框是否来自滚轮手动切换。
 * @return 手动层级时返回 true。
 */
bool SelectionCursor::manualLevel() const
{
    return m_manualLevel;
}

/**
 * 返回当前缓存的矩形。
 * @return 当前矩形；尚未悬停时为空。
 */
std::optional<QRect> SelectionCursor::current() const
{
    return m_current;
}

/**
 * 清空悬停状态。
 * @return 无返回值。
 */
void SelectionCursor::reset()
{
    m_current.reset();
    m_anchor.reset();
    m_manualLevel = false;
}

} // namespace markshot::region_detection
