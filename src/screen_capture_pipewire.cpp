#include "screen_capture_pipewire_screencast.h"

#ifdef HAVE_PIPEWIRE

namespace {

/**
 * 读取当前复用的 PipeWire 截屏会话。
 * @return 截屏会话实例。
 */
std::unique_ptr<PortalPipeWireScreencast> &activeScreencast()
{
    static std::unique_ptr<PortalPipeWireScreencast> screencast;
    return screencast;
}

}  // namespace

CaptureResult captureWithPortalScreencast(const CaptureRequest &request)
{
    std::unique_ptr<PortalPipeWireScreencast> &screencast = activeScreencast();
    if (!screencast) {
        screencast = std::make_unique<PortalPipeWireScreencast>();
    }
    return screencast->capture(request);
}

void stopPortalScreencast()
{
    activeScreencast().reset();
}

#else

CaptureResult captureWithPortalScreencast(const CaptureRequest &request)
{
    return {{}, QStringLiteral("PipeWire support was not enabled at build time"), {}, request.sourceGeometry};
}

void stopPortalScreencast()
{
}

#endif
