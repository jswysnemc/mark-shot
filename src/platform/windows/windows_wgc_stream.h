#pragma once

#include "platform/windows/windows_wgc_frame.h"
#include "screen_capture.h"

#include <functional>
#include <memory>

namespace markshot::platform::windows {

class WindowsWgcStream final {
public:
    using FrameCallback = std::function<void(WindowsWgcFrame)>;
    using ErrorCallback = std::function<void(QString)>;

    /**
     * 创建 Windows Graphics Capture 流式采集器。
     */
    WindowsWgcStream();

    /**
     * 销毁 Windows Graphics Capture 流式采集器。
     */
    ~WindowsWgcStream();

    /**
     * 启动 Windows Graphics Capture 长生命周期帧流。
     * @param request 捕获请求。
     * @param onFrame 帧回调。
     * @param onError 错误回调。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    bool start(const CaptureRequest &request,
               FrameCallback onFrame,
               ErrorCallback onError,
               QString *error);

    /**
     * 停止 Windows Graphics Capture 长生命周期帧流。
     * @return 无返回值。
     */
    void stop();

private:
    class Private;
    std::unique_ptr<Private> d;
};

/**
 * 判断当前构建和系统是否支持 Windows Graphics Capture 流式采集。
 * @param error 输出错误信息。
 * @return 支持时返回 true。
 */
bool windowsWgcStreamSupported(QString *error = nullptr);

}  // namespace markshot::platform::windows
