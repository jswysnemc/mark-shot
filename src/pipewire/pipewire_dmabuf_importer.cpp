#include "pipewire/pipewire_dmabuf_importer.h"

#ifdef HAVE_PIPEWIRE

#include "debug_log.h"
#include "pipewire/pipewire_dmabuf_importer_gl_helpers.h"

#include <QByteArray>

#include <array>
#include <cstdlib>
#include <cstring>
#include <vector>

namespace markshot {

#ifdef HAVE_PIPEWIRE_DMABUF_IMPORT
using namespace markshot::dmabuf_detail;
#endif

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
     * 【录制】【PipeWire DMA-BUF】导入 DMA-BUF buffer 为 CPU 可读图像。
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
        const QRect fullFrame(0, 0,
                              static_cast<int>(videoInfo.size.width),
                              static_cast<int>(videoInfo.size.height));
        if (!importAndRender(buffer, videoInfo, fullFrame, error)) {
            return {};
        }
        return readbackToImage(fullFrame.size(), error);
#endif
    }

    /**
     * 【录制】【PipeWire DMA-BUF】导入 DMA-BUF buffer 并裁剪直读为 BGRA 字节。
     * @param buffer PipeWire 提供的 SPA buffer。
     * @param videoInfo 已协商的视频格式信息。
     * @param crop 帧内裁剪区域。
     * @param bgra 输出缓冲，成功时按 crop 尺寸填充自底向上的 BGRA 数据。
     * @param error 输出错误信息。
     * @return 读取成功时返回 true。
     */
    bool importBufferToBgra(const spa_buffer *buffer,
                            const spa_video_info_raw &videoInfo,
                            const QRect &crop,
                            QByteArray *bgra,
                            QString *error)
    {
#ifndef HAVE_PIPEWIRE_DMABUF_IMPORT
        Q_UNUSED(buffer);
        Q_UNUSED(videoInfo);
        Q_UNUSED(crop);
        Q_UNUSED(bgra);
        if (error) {
            *error = QStringLiteral("PipeWire DMA-BUF import is not built because EGL/GLESv2 is unavailable");
        }
        return false;
#else
        if (!bgra || crop.isEmpty()) {
            if (error) {
                *error = QStringLiteral("PipeWire DMA-BUF BGRA readback arguments are invalid");
            }
            return false;
        }
        if (!importAndRender(buffer, videoInfo, crop, error)) {
            return false;
        }

        // 1. 直接把离屏帧读进目标缓冲，着色器已输出 BGRA 字节序，免去 QImage 转换链
        const qsizetype rowBytes = static_cast<qsizetype>(crop.width()) * 4;
        bgra->resize(rowBytes * crop.height());
        glReadPixels(0, 0, crop.width(), crop.height(), GL_RGBA, GL_UNSIGNED_BYTE, bgra->data());
        if (glGetError() != GL_NO_ERROR) {
            if (error) {
                *error = glErrorText(QStringLiteral("failed to read PipeWire DMA-BUF pixels"));
            }
            return false;
        }
        // 2. glReadPixels 自底向上返回行序，由调用方以 yInverted 标记交给编码器处理
        return true;
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
        QString displaySource = QStringLiteral("default");
        if (getPlatformDisplay
            && (hasExtension(clientExtensions, "EGL_EXT_platform_wayland")
                || hasExtension(clientExtensions, "EGL_KHR_platform_wayland"))) {
            void *waylandDisplay = qtWaylandDisplay();
            if (waylandDisplay) {
                m_display = getPlatformDisplay(EGL_PLATFORM_WAYLAND_EXT,
                                               waylandDisplay,
                                               nullptr);
                displaySource = QStringLiteral("wayland");
            }
        }
        if (m_display == EGL_NO_DISPLAY && getPlatformDisplay
            && hasExtension(clientExtensions, "EGL_MESA_platform_surfaceless")) {
            m_display = getPlatformDisplay(EGL_PLATFORM_SURFACELESS_MESA,
                                           EGL_DEFAULT_DISPLAY,
                                           nullptr);
            displaySource = QStringLiteral("surfaceless");
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
        markshot::debugLog("screencast",
                           "【录制】【PipeWire DMA-BUF】egl-display source=%s vendor=%s version=%s modifier_import=%d",
                           displaySource.toUtf8().constData(),
                           eglQueryString(m_display, EGL_VENDOR),
                           eglQueryString(m_display, EGL_VERSION),
                           m_hasModifierImport ? 1 : 0);

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
        // 交换红蓝通道输出，使 GL_RGBA 读回字节序恰好是 BGRA（即 ARGB32 小端布局）
        static constexpr const char *kFragmentShader =
            "precision mediump float;\n"
            "uniform sampler2D uTexture;\n"
            "varying vec2 vTexCoord;\n"
            "void main() {\n"
            "    vec4 color = texture2D(uTexture, vTexCoord);\n"
            "    gl_FragColor = vec4(color.b, color.g, color.r, 1.0);\n"
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
     * 【录制】【PipeWire DMA-BUF】校验 buffer 并把裁剪区域渲染到离屏帧。
     * @param buffer PipeWire 提供的 SPA buffer。
     * @param videoInfo 已协商的视频格式信息。
     * @param crop 帧内裁剪区域。
     * @param error 输出错误信息。
     * @return 渲染成功后 FBO 保持绑定，可直接读回。
     */
    bool importAndRender(const spa_buffer *buffer,
                         const spa_video_info_raw &videoInfo,
                         const QRect &crop,
                         QString *error)
    {
        if (!initialize(error)) {
            return false;
        }
        if (!buffer || buffer->n_datas == 0) {
            if (error) {
                *error = QStringLiteral("PipeWire DMA-BUF buffer is empty");
            }
            return false;
        }
        const spa_data &data = buffer->datas[0];
        if (data.type != SPA_DATA_DmaBuf || data.fd < 0 || !data.chunk) {
            if (error) {
                *error = QStringLiteral("PipeWire buffer is not a usable DMA-BUF (type=%1 fd=%2)")
                             .arg(static_cast<uint>(data.type))
                             .arg(static_cast<qint64>(data.fd));
            }
            return false;
        }

        const int width = static_cast<int>(videoInfo.size.width);
        const int height = static_cast<int>(videoInfo.size.height);
        const int bpp = bytesPerPixel(videoInfo.format);
        if (width <= 0 || height <= 0 || bpp <= 0 || crop.isEmpty()
            || !QRect(0, 0, width, height).contains(crop)) {
            if (error) {
                *error = QStringLiteral("PipeWire DMA-BUF frame format is invalid");
            }
            return false;
        }

        const std::optional<EGLint> fourcc = drmFourccForFormat(videoInfo.format);
        if (!fourcc.has_value()) {
            if (error) {
                *error = QStringLiteral("PipeWire DMA-BUF format %1 has no DRM fourcc mapping")
                             .arg(static_cast<int>(videoInfo.format));
            }
            return false;
        }

        const EGLint pitch = data.chunk->stride != 0
            ? static_cast<EGLint>(std::abs(static_cast<int>(data.chunk->stride)))
            : static_cast<EGLint>(width * bpp);
        if (pitch <= 0) {
            if (error) {
                *error = QStringLiteral("PipeWire DMA-BUF pitch is invalid");
            }
            return false;
        }

        if (!ensureOutputTarget(crop.width(), crop.height(), error)) {
            return false;
        }

        const std::uint64_t modifier = videoInfo.modifier;
#ifdef HAVE_PIPEWIRE_VIDEO_INFO_RAW_FLAGS
        const bool hasModifier = (videoInfo.flags & SPA_VIDEO_FLAG_MODIFIER) != 0
            && modifier != kDrmFormatModInvalid;
#else
        const bool hasModifier = modifier != 0 && modifier != kDrmFormatModInvalid;
#endif
        if (hasModifier && !m_hasModifierImport) {
            if (error) {
                *error = QStringLiteral("EGL does not support DMA-BUF modifier import");
            }
            return false;
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
            return false;
        }

        GLuint importedTexture = 0;
        glGenTextures(1, &importedTexture);
        glBindTexture(GL_TEXTURE_2D, importedTexture);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
        m_glEGLImageTargetTexture2DOES(GL_TEXTURE_2D, image);
        const GLenum bindError = glGetError();
        if (bindError != GL_NO_ERROR) {
            glDeleteTextures(1, &importedTexture);
            m_eglDestroyImageKHR(m_display, image);
            if (error) {
                *error = glErrorText(QStringLiteral("failed to bind PipeWire DMA-BUF EGLImage"),
                                     bindError);
            }
            return false;
        }

        const bool rendered = renderTexture(importedTexture, crop, QSize(width, height), error);
        glDeleteTextures(1, &importedTexture);
        m_eglDestroyImageKHR(m_display, image);
        return rendered;
    }

    /**
     * 【录制】【PipeWire DMA-BUF】把导入纹理的裁剪区域绘制到离屏帧。
     * @param texture 已导入的 DMA-BUF 纹理。
     * @param crop 帧内裁剪区域。
     * @param frameSize 完整帧尺寸。
     * @param error 输出错误信息。
     * @return 绘制成功时返回 true。
     */
    bool renderTexture(GLuint texture, const QRect &crop, const QSize &frameSize, QString *error)
    {
        // 1. 按裁剪区域计算纹理坐标，FBO 底行对应图像底部，读回时行序自底向上
        const GLfloat u0 = static_cast<GLfloat>(crop.x()) / frameSize.width();
        const GLfloat u1 = static_cast<GLfloat>(crop.x() + crop.width()) / frameSize.width();
        const GLfloat vTop = static_cast<GLfloat>(crop.y()) / frameSize.height();
        const GLfloat vBottom = static_cast<GLfloat>(crop.y() + crop.height()) / frameSize.height();
        const std::array<GLfloat, 16> vertices = {
            -1.0f, -1.0f, u0, vBottom,
             1.0f, -1.0f, u1, vBottom,
            -1.0f,  1.0f, u0, vTop,
             1.0f,  1.0f, u1, vTop,
        };

        glBindFramebuffer(GL_FRAMEBUFFER, m_fbo);
        glViewport(0, 0, crop.width(), crop.height());
        glUseProgram(m_program);
        glActiveTexture(GL_TEXTURE0);
        glBindTexture(GL_TEXTURE_2D, texture);
        glUniform1i(m_textureUniform, 0);
        glVertexAttribPointer(0,
                              2,
                              GL_FLOAT,
                              GL_FALSE,
                              4 * sizeof(GLfloat),
                              vertices.data());
        glVertexAttribPointer(1,
                              2,
                              GL_FLOAT,
                              GL_FALSE,
                              4 * sizeof(GLfloat),
                              vertices.data() + 2);
        glEnableVertexAttribArray(0);
        glEnableVertexAttribArray(1);
        glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
        if (glGetError() != GL_NO_ERROR) {
            if (error) {
                *error = glErrorText(QStringLiteral("failed to render PipeWire DMA-BUF"));
            }
            return false;
        }
        return true;
    }

    /**
     * 【录制】【PipeWire DMA-BUF】把离屏帧读回为 QImage。
     * @param size 离屏帧尺寸。
     * @param error 输出错误信息。
     * @return 读回成功时返回图像。
     */
    QImage readbackToImage(const QSize &size, QString *error)
    {
        const int width = size.width();
        const int height = size.height();
        std::vector<uchar> pixels(static_cast<size_t>(width) * height * 4);
        glReadPixels(0, 0, width, height, GL_RGBA, GL_UNSIGNED_BYTE, pixels.data());
        if (glGetError() != GL_NO_ERROR) {
            if (error) {
                *error = glErrorText(QStringLiteral("failed to read PipeWire DMA-BUF pixels"));
            }
            return {};
        }

        // 着色器已输出 BGRA 字节序，逐行倒序拷贝即得到 top-down ARGB32 图像
        QImage image(width, height, QImage::Format_ARGB32);
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
        return image;
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

bool PipeWireDmaBufImporter::importBufferToBgra(const spa_buffer *buffer,
                                                const spa_video_info_raw &videoInfo,
                                                const QRect &crop,
                                                QByteArray *bgra,
                                                QString *error)
{
    return m_impl->importBufferToBgra(buffer, videoInfo, crop, bgra, error);
}

}  // namespace markshot

#endif
