#pragma once

#include "shot_window_types.h"

#include <QPointF>

namespace markshot::ruler {

/// 尺子工具在一次截图会话中的状态
struct RulerToolState {
    // 新画尺子的测量方式
    markshot::shot::types::RulerMode mode = markshot::shot::types::RulerMode::Length;
    // 新画尺子的粗细，与其他工具的粗细分开保存
    qreal width = 3.0;
    // 悬停十字线和坐标取色说明的位置，图像像素坐标
    QPointF hoverImagePoint;
    bool hoverValid = false;
    // 由选区阶段的尺子快捷键进入时为 true，Esc 回到选区阶段而不是关闭截图
    bool enteredFromSelection = false;
};

}  // namespace markshot::ruler
