#pragma once

#include "screen_capture.h"

#include <QMap>
#include <QVariant>

namespace markshot::capture {

struct KWinCaptureRequest {
    QString method;
    QVariantList arguments;
};

/**
 * 【KWin】【截图请求】根据当前输出名称和几何区域构造 ScreenShot2 请求。
 * @param request 捕获区域、输出名称及鼠标和自身窗口策略。
 * @param outputGeometries 当前输出名称到逻辑几何区域的映射，不包含坐标回退结果。
 * @return 方法名称及文件描述符之前的参数；只有名称严格匹配的整输出请求使用 CaptureScreen。
 */
KWinCaptureRequest buildKWinCaptureRequest(const CaptureRequest &request,
                                         const QMap<QString, QRect> &outputGeometries);

}  // namespace markshot::capture
