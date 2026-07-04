#pragma once

// 【录制】【PipeWire DMA-BUF】导入器内部 GL/EGL 辅助函数，仅供 importer 实现文件使用

#ifdef HAVE_PIPEWIRE_DMABUF_IMPORT

#include <QByteArray>
#include <QGuiApplication>
#include <QString>
#include <QtGlobal>
#include <QtGui/qguiapplication_platform.h>

#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES2/gl2.h>
#include <GLES2/gl2ext.h>

#include <spa/param/video/raw.h>

#include <algorithm>
#include <optional>

#if __has_include(<drm_fourcc.h>)
#include <drm_fourcc.h>
#define MARKSHOT_HAS_DRM_FOURCC 1
#elif __has_include(<libdrm/drm_fourcc.h>)
#include <libdrm/drm_fourcc.h>
#define MARKSHOT_HAS_DRM_FOURCC 1
#endif

#ifndef EGL_PLATFORM_SURFACELESS_MESA
#define EGL_PLATFORM_SURFACELESS_MESA 0x31DD
#endif

#ifndef EGL_PLATFORM_WAYLAND_EXT
#define EGL_PLATFORM_WAYLAND_EXT 0x31D8
#endif

namespace markshot::dmabuf_detail {

constexpr std::uint64_t kDrmFormatModInvalid =
#ifdef DRM_FORMAT_MOD_INVALID
    DRM_FORMAT_MOD_INVALID;
#else
    0x00ffffffffffffffULL;
#endif

/**
 * 【录制】【PipeWire DMA-BUF】判断 EGL 扩展列表是否包含指定扩展。
 * @param extensions EGL 扩展列表。
 * @param name 扩展名称。
 * @return 包含扩展时返回 true。
 */
inline bool hasExtension(const char *extensions, const char *name)
{
    if (!extensions || !name || !*name) {
        return false;
    }
    const QByteArray list(extensions);
    const QList<QByteArray> parts = list.split(' ');
    return std::any_of(parts.cbegin(), parts.cend(), [name](const QByteArray &part) {
        return part == name;
    });
}

/**
 * 【录制】【PipeWire DMA-BUF】生成 EGL 错误文本。
 * @param prefix 错误前缀。
 * @return 带 EGL 错误码的文本。
 */
inline QString eglErrorText(const QString &prefix)
{
    return QStringLiteral("%1 (EGL error 0x%2)")
        .arg(prefix)
        .arg(static_cast<uint>(eglGetError()), 0, 16);
}

/**
 * 【录制】【PipeWire DMA-BUF】生成 OpenGL 错误文本。
 * @param prefix 错误前缀。
 * @return 带 OpenGL 错误码的文本。
 */
inline QString glErrorText(const QString &prefix)
{
    return QStringLiteral("%1 (GL error 0x%2)")
        .arg(prefix)
        .arg(static_cast<uint>(glGetError()), 0, 16);
}

/**
 * 【录制】【PipeWire DMA-BUF】生成 OpenGL 错误文本。
 * @param prefix 错误前缀。
 * @param code OpenGL 错误码。
 * @return 带 OpenGL 错误码的文本。
 */
inline QString glErrorText(const QString &prefix, GLenum code)
{
    return QStringLiteral("%1 (GL error 0x%2)")
        .arg(prefix)
        .arg(static_cast<uint>(code), 0, 16);
}

/**
 * 【录制】【PipeWire DMA-BUF】读取 Qt Wayland 平台插件持有的 Wayland display。
 * @return 支持该 native interface 时返回 display 指针，否则返回空指针。
 */
inline void *qtWaylandDisplay()
{
#if QT_VERSION >= QT_VERSION_CHECK(6, 5, 0)
    auto *waylandApp = qGuiApp
        ? qGuiApp->nativeInterface<QNativeInterface::QWaylandApplication>()
        : nullptr;
    return waylandApp ? waylandApp->display() : nullptr;
#else
    return nullptr;
#endif
}

/**
 * 【录制】【PipeWire DMA-BUF】读取 SPA 像素格式每像素字节数。
 * @param format SPA 像素格式。
 * @return 每像素字节数，不支持时返回 0。
 */
inline int bytesPerPixel(spa_video_format format)
{
    switch (format) {
    case SPA_VIDEO_FORMAT_BGRA:
    case SPA_VIDEO_FORMAT_BGRx:
    case SPA_VIDEO_FORMAT_xBGR:
    case SPA_VIDEO_FORMAT_RGBA:
    case SPA_VIDEO_FORMAT_RGBx:
    case SPA_VIDEO_FORMAT_ARGB:
    case SPA_VIDEO_FORMAT_ABGR:
    case SPA_VIDEO_FORMAT_xRGB:
        return 4;
    case SPA_VIDEO_FORMAT_RGB:
    case SPA_VIDEO_FORMAT_BGR:
        return 3;
    default:
        return 0;
    }
}

/**
 * 【录制】【PipeWire DMA-BUF】将 SPA raw 格式映射为 DRM fourcc。
 * @param format SPA 像素格式。
 * @return 可用于 EGL 导入的 DRM fourcc。
 */
inline std::optional<EGLint> drmFourccForFormat(spa_video_format format)
{
#ifdef MARKSHOT_HAS_DRM_FOURCC
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
#else
    Q_UNUSED(format);
    return std::nullopt;
#endif
}

/**
 * 【录制】【PipeWire DMA-BUF】编译 GLES 着色器。
 * @param type 着色器类型。
 * @param source 着色器源码。
 * @param error 输出错误信息。
 * @return 成功时返回着色器对象，失败时返回 0。
 */
inline GLuint compileShader(GLenum type, const char *source, QString *error)
{
    const GLuint shader = glCreateShader(type);
    glShaderSource(shader, 1, &source, nullptr);
    glCompileShader(shader);
    GLint ok = GL_FALSE;
    glGetShaderiv(shader, GL_COMPILE_STATUS, &ok);
    if (ok == GL_TRUE) {
        return shader;
    }
    GLint length = 0;
    glGetShaderiv(shader, GL_INFO_LOG_LENGTH, &length);
    QByteArray log(std::max(1, length), '\0');
    glGetShaderInfoLog(shader, log.size(), nullptr, log.data());
    if (error) {
        *error = QStringLiteral("failed to compile DMA-BUF shader: %1")
                     .arg(QString::fromUtf8(log.constData()).trimmed());
    }
    glDeleteShader(shader);
    return 0;
}

}  // namespace markshot::dmabuf_detail

#endif
