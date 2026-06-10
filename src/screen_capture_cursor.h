#pragma once

#include "screen_capture.h"

#include <QImage>
#include <QPoint>

namespace markshot::capture {

struct CursorFrame {
    QImage image;
    QPoint hotSpot;
    QPoint globalPosition;
};

/// @brief 创建用于后端无法提供原生鼠标图像时的箭头光标。
/// @return 带透明背景的箭头光标图像。
QImage fallbackCursorImage();

/// @brief 将指定鼠标图像绘制到捕获结果中。
/// @param capture 捕获结果，函数会直接修改其中的 image。
/// @param cursor 鼠标图像、热点和全局位置。
/// @return 成功绘制鼠标时返回 true。
bool paintCursorFrameIntoCapture(CaptureResult *capture, const CursorFrame &cursor);

/// @brief 将当前鼠标绘制到捕获结果中。
/// @param capture 捕获结果，函数会直接修改其中的 image。
/// @return 成功绘制鼠标时返回 true。
bool paintCurrentCursorIntoCapture(CaptureResult *capture);

}  // namespace markshot::capture
