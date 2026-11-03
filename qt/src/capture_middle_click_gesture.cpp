#include "capture_middle_click_gesture.h"

#include <algorithm>

namespace markshot {

void CaptureMiddleClickGesture::setAction(CaptureClickAction action)
{
    m_action = action;
}

CaptureClickAction CaptureMiddleClickGesture::action() const
{
    return m_action;
}

void CaptureMiddleClickGesture::press(QPointF position, int dragThreshold)
{
    m_pressPosition = position;
    m_dragThreshold = std::max(1, dragThreshold);
    m_pending = true;
}

bool CaptureMiddleClickGesture::track(QPointF position)
{
    // 1. 超过阈值后本次按下已转为平移，即使指针移回原处也不再视为单击
    if (m_pending && !withinThreshold(position)) {
        m_pending = false;
    }
    return m_pending;
}

bool CaptureMiddleClickGesture::release(QPointF position)
{
    // 1. 释放前未经移动事件上报的位移同样按阈值判定
    const bool clicked = track(position);
    m_pending = false;
    return clicked;
}

void CaptureMiddleClickGesture::cancel()
{
    m_pending = false;
}

bool CaptureMiddleClickGesture::pending() const
{
    return m_pending;
}

bool CaptureMiddleClickGesture::withinThreshold(QPointF position) const
{
    return (position - m_pressPosition).manhattanLength() < m_dragThreshold;
}

}  // namespace markshot
