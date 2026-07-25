#pragma once

#include <cstdint>
#include <optional>

#if __has_include(<spa/param/video/raw.h>)
#include <spa/param/video/raw.h>
#endif

// Prefer system headers when present, but always keep numeric fallbacks so
// packaging without libdrm-dev still maps SPA formats (e.g. BGRx=8 on labwc).
#if __has_include(<drm_fourcc.h>)
#include <drm_fourcc.h>
#elif __has_include(<libdrm/drm_fourcc.h>)
#include <libdrm/drm_fourcc.h>
#endif

#ifndef fourcc_code
#define fourcc_code(a, b, c, d) \
    ((std::uint32_t)(a) | ((std::uint32_t)(b) << 8) | ((std::uint32_t)(c) << 16) \
     | ((std::uint32_t)(d) << 24))
#endif

#ifndef DRM_FORMAT_ARGB8888
#define DRM_FORMAT_ARGB8888 fourcc_code('A', 'R', '2', '4')
#endif
#ifndef DRM_FORMAT_XRGB8888
#define DRM_FORMAT_XRGB8888 fourcc_code('X', 'R', '2', '4')
#endif
#ifndef DRM_FORMAT_ABGR8888
#define DRM_FORMAT_ABGR8888 fourcc_code('A', 'B', '2', '4')
#endif
#ifndef DRM_FORMAT_XBGR8888
#define DRM_FORMAT_XBGR8888 fourcc_code('X', 'B', '2', '4')
#endif
#ifndef DRM_FORMAT_BGRA8888
#define DRM_FORMAT_BGRA8888 fourcc_code('B', 'A', '2', '4')
#endif
#ifndef DRM_FORMAT_RGBA8888
#define DRM_FORMAT_RGBA8888 fourcc_code('R', 'A', '2', '4')
#endif
#ifndef DRM_FORMAT_BGRX8888
#define DRM_FORMAT_BGRX8888 fourcc_code('B', 'X', '2', '4')
#endif
#ifndef DRM_FORMAT_RGBX8888
#define DRM_FORMAT_RGBX8888 fourcc_code('R', 'X', '2', '4')
#endif
#ifndef DRM_FORMAT_RGB888
#define DRM_FORMAT_RGB888 fourcc_code('R', 'G', '2', '4')
#endif
#ifndef DRM_FORMAT_BGR888
#define DRM_FORMAT_BGR888 fourcc_code('B', 'G', '2', '4')
#endif

namespace markshot::pipewire {

/**
 * 【录制】【PipeWire DMA-BUF】将 SPA raw 格式映射为 DRM fourcc。
 * @param format SPA 像素格式。
 * @return 可用于 EGL 导入的 DRM fourcc。
 */
inline std::optional<std::uint32_t> drmFourccForSpaFormat(spa_video_format format)
{
    switch (format) {
    case SPA_VIDEO_FORMAT_BGRA:
        return DRM_FORMAT_ARGB8888;
    case SPA_VIDEO_FORMAT_BGRx:
        return DRM_FORMAT_XRGB8888;
    case SPA_VIDEO_FORMAT_RGBA:
        return DRM_FORMAT_ABGR8888;
    case SPA_VIDEO_FORMAT_RGBx:
        return DRM_FORMAT_XBGR8888;
    case SPA_VIDEO_FORMAT_ARGB:
        return DRM_FORMAT_BGRA8888;
    case SPA_VIDEO_FORMAT_ABGR:
        return DRM_FORMAT_RGBA8888;
    case SPA_VIDEO_FORMAT_xRGB:
        return DRM_FORMAT_BGRX8888;
    case SPA_VIDEO_FORMAT_xBGR:
        return DRM_FORMAT_RGBX8888;
    case SPA_VIDEO_FORMAT_RGB:
        return DRM_FORMAT_RGB888;
    case SPA_VIDEO_FORMAT_BGR:
        return DRM_FORMAT_BGR888;
    default:
        return std::nullopt;
    }
}

}  // namespace markshot::pipewire
