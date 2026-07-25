#pragma once

#include <QPoint>
#include <QRect>
#include <QSize>

namespace markshot::shot {

struct OcrResultWindowPlacement {
    QSize size;
    QPoint topLeft;
};

/**
 * 【OCR】【结果窗口放置】计算 OCR 结果窗口的初始尺寸与位置。
 * @param targetAvailableGeometry 截图目标屏幕的可用区域。
 * @param fallbackAvailableGeometry 目标屏幕不可用时采用的主屏幕可用区域。
 * @return OCR 结果窗口的初始尺寸与左上角位置。
 */
OcrResultWindowPlacement ocrResultWindowPlacement(const QRect &targetAvailableGeometry,
                                                  const QRect &fallbackAvailableGeometry);

}  // namespace markshot::shot
