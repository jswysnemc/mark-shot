#include "pipewire/pipewire_dmabuf_importer.h"

#ifdef HAVE_PIPEWIRE

#include <QByteArray>

#include <algorithm>
#include <array>
#include <cstring>
#include <optional>
#include <vector>

#ifdef HAVE_PIPEWIRE_DMABUF_IMPORT
#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES2/gl2.h>
#include <GLES2/gl2ext.h>

#if __has_include(<drm_fourcc.h>)
#include <drm_fourcc.h>
#define MARKSHOT_HAS_DRM_FOURCC 1
#elif __has_include(<libdrm/drm_fourcc.h>)
#include <libdrm/drm_fourcc.h>
#define MARKSHOT_HAS_DRM_FOURCC 1
#endif
#endif

namespace markshot {

namespace {

#ifdef HAVE_PIPEWIRE_DMABUF_IMPORT

constexpr std::uint64_t kDrmFormatModInvalid =
#ifdef DRM_FORMAT_MOD_INVALID
    DRM_FORMAT_MOD_INVALID;
#else
    0x00ffffffffffffffULL;
#endif

#ifndef EGL_PLATFORM_SURFACELESS_MESA
#define EGL_PLATFORM_SURFACELESS_MESA 0x31DD
#endif

/**
 * 【录制】【PipeWire DMA-BUF】判断 EGL 扩展列表是否包含指定扩展。
 * @param extensions EGL 扩展列表。
 * @param name 扩展名称。
 * @return 包含扩展时返回 true。
 */
bool hasExtension(const char *extensions, const char *name)
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
QString eglErrorText(const QString &prefix)
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
QString glErrorText(const QString &prefix)
{
    return QStringLiteral("%1 (GL error 0x%2)")
        .arg(prefix)
        .arg(static_cast<uint>(glGetError()), 0, 16);
}

/**
 * 【录制】【PipeWire DMA-BUF】读取 SPA 像素格式每像素字节数。
 * @param format SPA 像素格式。
 * @return 每像素字节数，不支持时返回 0。
 */
int bytesPerPixel(spa_video_format format)
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
std::optional<EGLint> drmFourccForFormat(spa_video_format format)
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
GLuint compileShader(GLenum type, const char *source, QString *error)
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

#endif

}  // namespace

class PipeWireDmaBufImporter::Impl {
public:
    /**
     * 【录制】【PipeWire DMA-BUF】释放 EGL 和 GLES 资源。
     */
    ~Impl()
    {
#ifdef HAVE_PIPEWIRE_DMABUF_IMPORT
        if (m_display != EGL_NO_DISPLAY) {
            eglMakeCurrent(m_display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
            if (m_outputTexture != 0) {
                glDeleteTextures(1, &m_outputTexture);
            }
            if (m_fbo != 0) {
                glDeleteFramebuffers(1, &m_fbo);
            }
            if (m_program != 0) {
                glDeleteProgram(m_program);
            }
            if (m_context != EGL_NO_CONTEXT) {
                eglDestroyContext(m_display, m_context);
            }
            if (m_surface != EGL_NO_SURFACE) {
                eglDestroySurface(m_display, m_surface);
            }
            eglTerminate(m_display);
        }
#endif
    }

    /**
     * 【录制】【PipeWire DMA-BUF】导入 DMA-BUF buffer。
     * @param buffer PipeWire 提供的 SPA buffer。
     * @param videoInfo 已协商的视频格式信息。
     * @param error 输出错误信息。
     * @return 导入成功时返回图像，失败时返回空图像。
     */
    QImage importBuffer(const spa_buffer *buffer,
                        const spa_video_info_raw &videoInfo,
                        QString *error)
    {
#ifndef HAVE_PIPEWIRE_DMABUF_IMPORT
        Q_UNUSED(buffer);
        Q_UNUSED(videoInfo);
        if (error) {
            *error = QStringLiteral("PipeWire DMA-BUF import is not built because EGL/GLESv2 is unavailable");
        }
        return {};
#else
        if (!initialize(error)) {
            return {};
        }
        if (!buffer || buffer->n_datas == 0) {
            if (error) {
                *error = QStringLiteral("PipeWire DMA-BUF buffer is empty");
            }
            return {};
        }
        const spa_data &data = buffer->datas[0];
        if (data.type != SPA_DATA_DmaBuf || data.fd < 0 || !data.chunk) {
            if (error) {
                *error = QStringLiteral("PipeWire buffer is not a usable DMA-BUF (type=%1 fd=%2)")
                             .arg(static_cast<uint>(data.type))
                             .arg(static_cast<qint64>(data.fd));
            }
            return {};
        }

        const int width = static_cast<int>(videoInfo.size.width);
        const int height = static_cast<int>(videoInfo.size.height);
        const int bpp = bytesPerPixel(videoInfo.format);
        if (width <= 0 || height <= 0 || bpp <= 0) {
            if (error) {
                *error = QStringLiteral("PipeWire DMA-BUF frame format is invalid");
            }
            return {};
        }

        const std::optional<EGLint> fourcc = drmFourccForFormat(videoInfo.format);
        if (!fourcc.has_value()) {
            if (error) {
                *error = QStringLiteral("PipeWire DMA-BUF format %1 has no DRM fourcc mapping")
                             .arg(static_cast<int>(videoInfo.format));
            }
            return {};
        }

        const EGLint pitch = data.chunk->stride != 0
            ? static_cast<EGLint>(std::abs(static_cast<int>(data.chunk->stride)))
            : static_cast<EGLint>(width * bpp);
        if (pitch <= 0) {
            if (error) {
                *error = QStringLiteral("PipeWire DMA-BUF pitch is invalid");
            }
            return {};
        }

        if (!ensureOutputTarget(width, height, error)) {
            return {};
        }

        const std::uint64_t modifier = videoInfo.modifier;
        const bool hasModifier = (videoInfo.flags & SPA_VIDEO_FLAG_MODIFIER) != 0
            && modifier != kDrmFormatModInvalid;
        if (hasModifier && !m_hasModifierImport) {
            if (error) {
                *error = QStringLiteral("EGL does not support DMA-BUF modifier import");
            }
            return {};
        }

        const EGLint offset = static_cast<EGLint>(data.mapoffset + data.chunk->offset);
        std::vector<EGLint> attributes = {
            EGL_WIDTH, width,
            EGL_HEIGHT, height,
            EGL_LINUX_DRM_FOURCC_EXT, fourcc.value(),
            EGL_DMA_BUF_PLANE0_FD_EXT, static_cast<EGLint>(data.fd),
            EGL_DMA_BUF_PLANE0_OFFSET_EXT, offset,
            EGL_DMA_BUF_PLANE0_PITCH_EXT, pitch,
        };
        if (hasModifier) {
            attributes.push_back(EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT);
            attributes.push_back(static_cast<EGLint>(modifier & 0xffffffffULL));
            attributes.push_back(EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT);
            attributes.push_back(static_cast<EGLint>(modifier >> 32));
        }
        attributes.push_back(EGL_NONE);

        EGLImageKHR image = m_eglCreateImageKHR(m_display,
                                                EGL_NO_CONTEXT,
                                                EGL_LINUX_DMA_BUF_EXT,
                                                nullptr,
                                                attributes.data());
        if (image == EGL_NO_IMAGE_KHR) {
            if (error) {
                *error = eglErrorText(QStringLiteral("failed to create EGLImage from PipeWire DMA-BUF"));
            }
            return {};
        }

        GLuint importedTexture = 0;
        glGenTextures(1, &importedTexture);
        glBindTexture(GL_TEXTURE_2D, importedTexture);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
        m_glEGLImageTargetTexture2DOES(GL_TEXTURE_2D, image);
        if (glGetError() != GL_NO_ERROR) {
            glDeleteTextures(1, &importedTexture);
            m_eglDestroyImageKHR(m_display, image);
            if (error) {
                *error = glErrorText(QStringLiteral("failed to bind PipeWire DMA-BUF EGLImage"));
            }
            return {};
        }

        const QImage result = renderImportedTexture(importedTexture, width, height, error);
        glDeleteTextures(1, &importedTexture);
        m_eglDestroyImageKHR(m_display, image);
        return result;
#endif
    }

private:
#ifdef HAVE_PIPEWIRE_DMABUF_IMPORT
    /**
     * 【录制】【PipeWire DMA-BUF】初始化 EGL 和 GLES 状态。
     * @param error 输出错误信息。
     * @return 初始化成功时返回 true。
     */
    bool initialize(QString *error)
    {
        if (m_initialized) {
            return true;
        }
        const char *clientExtensions = eglQueryString(EGL_NO_DISPLAY, EGL_EXTENSIONS);
        auto getPlatformDisplay =
            reinterpret_cast<PFNEGLGETPLATFORMDISPLAYEXTPROC>(
                eglGetProcAddress("eglGetPlatformDisplayEXT"));
        if (getPlatformDisplay
            && hasExtension(clientExtensions, "EGL_MESA_platform_surfaceless")) {
            m_display = getPlatformDisplay(EGL_PLATFORM_SURFACELESS_MESA,
                                           EGL_DEFAULT_DISPLAY,
                                           nullptr);
        }
        if (m_display == EGL_NO_DISPLAY) {
            m_display = eglGetDisplay(EGL_DEFAULT_DISPLAY);
        }
        if (m_display == EGL_NO_DISPLAY) {
            if (error) {
                *error = eglErrorText(QStringLiteral("failed to get EGL display"));
            }
            return false;
        }

        EGLint major = 0;
        EGLint minor = 0;
        if (eglInitialize(m_display, &major, &minor) != EGL_TRUE) {
            if (error) {
                *error = eglErrorText(QStringLiteral("failed to initialize EGL"));
            }
            return false;
        }
        if (eglBindAPI(EGL_OPENGL_ES_API) != EGL_TRUE) {
            if (error) {
                *error = eglErrorText(QStringLiteral("failed to bind OpenGL ES API"));
            }
            return false;
        }

        const char *displayExtensions = eglQueryString(m_display, EGL_EXTENSIONS);
        if (!hasExtension(displayExtensions, "EGL_EXT_image_dma_buf_import")) {
            if (error) {
                *error = QStringLiteral("EGL_EXT_image_dma_buf_import is unavailable");
            }
            return false;
        }
        m_hasModifierImport =
            hasExtension(displayExtensions, "EGL_EXT_image_dma_buf_import_modifiers");

        EGLConfig config = nullptr;
        EGLint configCount = 0;
        const EGLint configAttributes[] = {
            EGL_SURFACE_TYPE, EGL_PBUFFER_BIT,
            EGL_RENDERABLE_TYPE, EGL_OPENGL_ES2_BIT,
            EGL_RED_SIZE, 8,
            EGL_GREEN_SIZE, 8,
            EGL_BLUE_SIZE, 8,
            EGL_ALPHA_SIZE, 8,
            EGL_NONE,
        };
        if (eglChooseConfig(m_display, configAttributes, &config, 1, &configCount) != EGL_TRUE
            || configCount <= 0) {
            if (error) {
                *error = eglErrorText(QStringLiteral("failed to choose EGL config"));
            }
            return false;
        }

        const EGLint surfaceAttributes[] = {
            EGL_WIDTH, 1,
            EGL_HEIGHT, 1,
            EGL_NONE,
        };
        m_surface = eglCreatePbufferSurface(m_display, config, surfaceAttributes);
        if (m_surface == EGL_NO_SURFACE) {
            if (error) {
                *error = eglErrorText(QStringLiteral("failed to create EGL pbuffer"));
            }
            return false;
        }

        const EGLint contextAttributes[] = {
            EGL_CONTEXT_CLIENT_VERSION, 2,
            EGL_NONE,
        };
        m_context = eglCreateContext(m_display, config, EGL_NO_CONTEXT, contextAttributes);
        if (m_context == EGL_NO_CONTEXT) {
            if (error) {
                *error = eglErrorText(QStringLiteral("failed to create GLES context"));
            }
            return false;
        }
        if (eglMakeCurrent(m_display, m_surface, m_surface, m_context) != EGL_TRUE) {
            if (error) {
                *error = eglErrorText(QStringLiteral("failed to activate GLES context"));
            }
            return false;
        }

        m_eglCreateImageKHR =
            reinterpret_cast<PFNEGLCREATEIMAGEKHRPROC>(eglGetProcAddress("eglCreateImageKHR"));
        m_eglDestroyImageKHR =
            reinterpret_cast<PFNEGLDESTROYIMAGEKHRPROC>(eglGetProcAddress("eglDestroyImageKHR"));
        m_glEGLImageTargetTexture2DOES =
            reinterpret_cast<PFNGLEGLIMAGETARGETTEXTURE2DOESPROC>(
                eglGetProcAddress("glEGLImageTargetTexture2DOES"));
        if (!m_eglCreateImageKHR || !m_eglDestroyImageKHR || !m_glEGLImageTargetTexture2DOES) {
            if (error) {
                *error = QStringLiteral("required EGL/GLES DMA-BUF import functions are unavailable");
            }
            return false;
        }

        if (!createProgram(error)) {
            return false;
        }
        glGenFramebuffers(1, &m_fbo);
        glGenTextures(1, &m_outputTexture);
        m_initialized = true;
        return true;
    }

    /**
     * 【录制】【PipeWire DMA-BUF】创建纹理采样程序。
     * @param error 输出错误信息。
     * @return 创建成功时返回 true。
     */
    bool createProgram(QString *error)
    {
        static constexpr const char *kVertexShader =
            "attribute vec2 aPosition;\n"
            "attribute vec2 aTexCoord;\n"
            "varying vec2 vTexCoord;\n"
            "void main() {\n"
            "    gl_Position = vec4(aPosition, 0.0, 1.0);\n"
            "    vTexCoord = aTexCoord;\n"
            "}\n";
        static constexpr const char *kFragmentShader =
            "precision mediump float;\n"
            "uniform sampler2D uTexture;\n"
            "varying vec2 vTexCoord;\n"
            "void main() {\n"
            "    vec4 color = texture2D(uTexture, vTexCoord);\n"
            "    gl_FragColor = vec4(color.rgb, 1.0);\n"
            "}\n";

        const GLuint vertex = compileShader(GL_VERTEX_SHADER, kVertexShader, error);
        if (vertex == 0) {
            return false;
        }
        const GLuint fragment = compileShader(GL_FRAGMENT_SHADER, kFragmentShader, error);
        if (fragment == 0) {
            glDeleteShader(vertex);
            return false;
        }

        m_program = glCreateProgram();
        glAttachShader(m_program, vertex);
        glAttachShader(m_program, fragment);
        glBindAttribLocation(m_program, 0, "aPosition");
        glBindAttribLocation(m_program, 1, "aTexCoord");
        glLinkProgram(m_program);
        glDeleteShader(vertex);
        glDeleteShader(fragment);

        GLint ok = GL_FALSE;
        glGetProgramiv(m_program, GL_LINK_STATUS, &ok);
        if (ok != GL_TRUE) {
            GLint length = 0;
            glGetProgramiv(m_program, GL_INFO_LOG_LENGTH, &length);
            QByteArray log(std::max(1, length), '\0');
            glGetProgramInfoLog(m_program, log.size(), nullptr, log.data());
            if (error) {
                *error = QStringLiteral("failed to link DMA-BUF shader: %1")
                             .arg(QString::fromUtf8(log.constData()).trimmed());
            }
            glDeleteProgram(m_program);
            m_program = 0;
            return false;
        }
        m_textureUniform = glGetUniformLocation(m_program, "uTexture");
        return true;
    }

    /**
     * 【录制】【PipeWire DMA-BUF】确保离屏输出纹理尺寸可用。
     * @param width 输出宽度。
     * @param height 输出高度。
     * @param error 输出错误信息。
     * @return 输出目标可用时返回 true。
     */
    bool ensureOutputTarget(int width, int height, QString *error)
    {
        glBindTexture(GL_TEXTURE_2D, m_outputTexture);
        if (m_outputWidth != width || m_outputHeight != height) {
            glTexImage2D(GL_TEXTURE_2D,
                         0,
                         GL_RGBA,
                         width,
                         height,
                         0,
                         GL_RGBA,
                         GL_UNSIGNED_BYTE,
                         nullptr);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
            m_outputWidth = width;
            m_outputHeight = height;
        }
        glBindFramebuffer(GL_FRAMEBUFFER, m_fbo);
        glFramebufferTexture2D(GL_FRAMEBUFFER,
                               GL_COLOR_ATTACHMENT0,
                               GL_TEXTURE_2D,
                               m_outputTexture,
                               0);
        if (glCheckFramebufferStatus(GL_FRAMEBUFFER) != GL_FRAMEBUFFER_COMPLETE) {
            if (error) {
                *error = glErrorText(QStringLiteral("DMA-BUF output framebuffer is incomplete"));
            }
            return false;
        }
        return true;
    }

    /**
     * 【录制】【PipeWire DMA-BUF】渲染导入纹理并读回 CPU 图像。
     * @param texture 已导入的 DMA-BUF 纹理。
     * @param width 图像宽度。
     * @param height 图像高度。
     * @param error 输出错误信息。
     * @return 读回成功时返回图像。
     */
    QImage renderImportedTexture(GLuint texture, int width, int height, QString *error)
    {
        static constexpr std::array<GLfloat, 16> kVertices = {
            -1.0f, -1.0f, 0.0f, 1.0f,
             1.0f, -1.0f, 1.0f, 1.0f,
            -1.0f,  1.0f, 0.0f, 0.0f,
             1.0f,  1.0f, 1.0f, 0.0f,
        };

        glBindFramebuffer(GL_FRAMEBUFFER, m_fbo);
        glViewport(0, 0, width, height);
        glUseProgram(m_program);
        glActiveTexture(GL_TEXTURE0);
        glBindTexture(GL_TEXTURE_2D, texture);
        glUniform1i(m_textureUniform, 0);
        glVertexAttribPointer(0,
                              2,
                              GL_FLOAT,
                              GL_FALSE,
                              4 * sizeof(GLfloat),
                              kVertices.data());
        glVertexAttribPointer(1,
                              2,
                              GL_FLOAT,
                              GL_FALSE,
                              4 * sizeof(GLfloat),
                              kVertices.data() + 2);
        glEnableVertexAttribArray(0);
        glEnableVertexAttribArray(1);
        glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
        if (glGetError() != GL_NO_ERROR) {
            if (error) {
                *error = glErrorText(QStringLiteral("failed to render PipeWire DMA-BUF"));
            }
            return {};
        }

        std::vector<uchar> pixels(static_cast<size_t>(width) * height * 4);
        glReadPixels(0, 0, width, height, GL_RGBA, GL_UNSIGNED_BYTE, pixels.data());
        if (glGetError() != GL_NO_ERROR) {
            if (error) {
                *error = glErrorText(QStringLiteral("failed to read PipeWire DMA-BUF pixels"));
            }
            return {};
        }

        QImage image(width, height, QImage::Format_RGBA8888);
        if (image.isNull()) {
            if (error) {
                *error = QStringLiteral("failed to allocate PipeWire DMA-BUF readback image");
            }
            return {};
        }
        const int rowBytes = width * 4;
        for (int y = 0; y < height; ++y) {
            const uchar *source = pixels.data() + static_cast<size_t>(height - 1 - y) * rowBytes;
            std::memcpy(image.scanLine(y), source, static_cast<size_t>(rowBytes));
        }
        return image.convertToFormat(QImage::Format_ARGB32);
    }

    bool m_initialized = false;
    bool m_hasModifierImport = false;
    EGLDisplay m_display = EGL_NO_DISPLAY;
    EGLContext m_context = EGL_NO_CONTEXT;
    EGLSurface m_surface = EGL_NO_SURFACE;
    GLuint m_program = 0;
    GLuint m_fbo = 0;
    GLuint m_outputTexture = 0;
    GLint m_textureUniform = -1;
    int m_outputWidth = 0;
    int m_outputHeight = 0;
    PFNEGLCREATEIMAGEKHRPROC m_eglCreateImageKHR = nullptr;
    PFNEGLDESTROYIMAGEKHRPROC m_eglDestroyImageKHR = nullptr;
    PFNGLEGLIMAGETARGETTEXTURE2DOESPROC m_glEGLImageTargetTexture2DOES = nullptr;
#endif
};

PipeWireDmaBufImporter::PipeWireDmaBufImporter()
    : m_impl(std::make_unique<Impl>())
{
}

PipeWireDmaBufImporter::~PipeWireDmaBufImporter() = default;

QImage PipeWireDmaBufImporter::importBuffer(const spa_buffer *buffer,
                                            const spa_video_info_raw &videoInfo,
                                            QString *error)
{
    return m_impl->importBuffer(buffer, videoInfo, error);
}

}  // namespace markshot

#endif
