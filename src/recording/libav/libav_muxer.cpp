#include "recording/libav/libav_muxer.h"

#ifdef HAVE_LIBAV_RECORDING

#include "recording/libav_error.h"

#include <QFile>

namespace markshot::recording {
namespace {

/**
 * 向调用方写入错误文本。
 * @param error 输出错误信息。
 * @param text 错误文本。
 * @return 固定返回 false，便于调用处直接返回。
 */
bool failWith(QString *error, const QString &text)
{
    if (error) {
        *error = text;
    }
    return false;
}

}  // namespace

LibavMuxer::~LibavMuxer()
{
    close();
}

bool LibavMuxer::open(const QString &outputPath, const QString &formatName, QString *error)
{
    if (error) {
        error->clear();
    }
    close();

    // 1. 按显式容器名或输出扩展名分配容器上下文
    m_outputPathBytes = QFile::encodeName(outputPath);
    const QByteArray formatBytes = formatName.trimmed().toUtf8();
    int result = avformat_alloc_output_context2(&m_formatContext,
                                                nullptr,
                                                formatBytes.isEmpty() ? nullptr : formatBytes.constData(),
                                                m_outputPathBytes.constData());
    if (result < 0 || !m_formatContext) {
        return failWith(error,
                        QStringLiteral("Failed to allocate libav output context: %1")
                            .arg(libavErrorText(result)));
    }

    // 2. 需要落盘的容器才打开 IO 上下文
    if (!(m_formatContext->oformat->flags & AVFMT_NOFILE)) {
        result = avio_open(&m_formatContext->pb, m_outputPathBytes.constData(), AVIO_FLAG_WRITE);
        if (result < 0) {
            return failWith(error,
                            QStringLiteral("Failed to open libav output file: %1")
                                .arg(libavErrorText(result)));
        }
    }
    return true;
}

bool LibavMuxer::writeHeader(QString *error)
{
    if (error) {
        error->clear();
    }
    if (!m_formatContext) {
        return failWith(error, QStringLiteral("libav muxer is not open"));
    }
    const int result = avformat_write_header(m_formatContext, nullptr);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to write libav output header: %1")
                            .arg(libavErrorText(result)));
    }
    return true;
}

bool LibavMuxer::writePacket(AVPacket *packet, QString *error)
{
    if (error) {
        error->clear();
    }
    if (!m_formatContext || !packet) {
        return failWith(error, QStringLiteral("libav muxer is not open"));
    }

    int result = 0;
    {
        std::lock_guard<std::mutex> lock(m_writeMutex);
        result = av_interleaved_write_frame(m_formatContext, packet);
    }
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to write libav packet: %1")
                            .arg(libavErrorText(result)));
    }
    return true;
}

bool LibavMuxer::writeTrailer(QString *error)
{
    if (error) {
        error->clear();
    }
    if (!m_formatContext) {
        return failWith(error, QStringLiteral("libav muxer is not open"));
    }
    const int result = av_write_trailer(m_formatContext);
    if (result < 0) {
        return failWith(error,
                        QStringLiteral("Failed to write libav output trailer: %1")
                            .arg(libavErrorText(result)));
    }
    return true;
}

void LibavMuxer::close()
{
    if (!m_formatContext) {
        return;
    }
    if (m_formatContext->pb) {
        avio_closep(&m_formatContext->pb);
    }
    avformat_free_context(m_formatContext);
    m_formatContext = nullptr;
}

bool LibavMuxer::needsGlobalHeader() const
{
    return m_formatContext && (m_formatContext->oformat->flags & AVFMT_GLOBALHEADER);
}

}  // namespace markshot::recording

#endif  // HAVE_LIBAV_RECORDING
