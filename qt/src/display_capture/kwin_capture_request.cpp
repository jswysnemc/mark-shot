#include "display_capture/kwin_capture_request.h"

namespace markshot::capture {

KWinCaptureRequest buildKWinCaptureRequest(const CaptureRequest &request,
                                         const QMap<QString, QRect> &outputGeometries)
{
    // 1. 【KWin】【截图请求】严格按名称匹配输出，未知名称不得借用坐标命中的其他输出
    const QRect geometry = request.sourceGeometry.normalized();
    const auto output = outputGeometries.constFind(request.preferredOutputName);
    const bool wholeOutput = !request.allOutputs && !request.preferredOutputName.isEmpty()
        && output != outputGeometries.constEnd() && !output.value().isEmpty()
        && output.value() == geometry;

    // 2. 【KWin】【截图请求】保持原生分辨率及鼠标、自身窗口选项，不依赖整输出判定
    QVariantMap options{
        {QStringLiteral("include-cursor"), request.includeCursor},
        {QStringLiteral("native-resolution"), true},
    };
    if (!request.hideOwnWindows) {
        options.insert(QStringLiteral("hide-caller-windows"), false);
    }

    // 3. 【KWin】【截图请求】整输出交由 KWin 使用该输出的缩放，区域继续使用逻辑坐标
    if (wholeOutput) {
        return {QStringLiteral("CaptureScreen"), {request.preferredOutputName, options}};
    }
    return {QStringLiteral("CaptureArea"),
            {geometry.x(), geometry.y(), static_cast<uint>(geometry.width()),
             static_cast<uint>(geometry.height()), options}};
}

}  // namespace markshot::capture
