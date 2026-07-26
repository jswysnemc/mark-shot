#pragma once

#include <QString>

#ifdef HAVE_LIBAV_RECORDING

#include <QByteArray>

#include <mutex>

extern "C" {
#include <libavformat/avformat.h>
}

namespace markshot::recording {

/**
 * 【录制】【容器封装】管理输出容器的打开、写包与关闭。
 *
 * 视频编码线程与音频编码线程共用同一个容器，写包必须串行，
 * 因此写锁由本类持有并对外暴露给音频编码器复用。
 */
class LibavMuxer final {
public:
    LibavMuxer() = default;
    ~LibavMuxer();
    LibavMuxer(const LibavMuxer &) = delete;
    LibavMuxer &operator=(const LibavMuxer &) = delete;

    /**
     * 打开输出容器。
     * @param outputPath 输出文件路径。
     * @param formatName 强制容器名称，传空时按扩展名推断。
     * @param error 输出错误信息。
     * @return 打开成功时返回 true。
     */
    bool open(const QString &outputPath, const QString &formatName, QString *error);

    /**
     * 写出容器头。
     * @param error 输出错误信息。
     * @return 写出成功时返回 true。
     */
    bool writeHeader(QString *error);

    /**
     * 加锁写出一个已完成时间基换算的 packet。
     * @param packet 待写出的 packet。
     * @param error 输出错误信息。
     * @return 写出成功时返回 true。
     */
    bool writePacket(AVPacket *packet, QString *error);

    /**
     * 写出容器尾。
     * @param error 输出错误信息。
     * @return 写出成功时返回 true。
     */
    bool writeTrailer(QString *error);

    /**
     * 关闭容器并释放资源。
     * @return 无返回值。
     */
    void close();

    /**
     * 读取底层容器上下文。
     * @return 容器上下文，未打开时返回空。
     */
    AVFormatContext *formatContext() const
    {
        return m_formatContext;
    }

    /**
     * 判断容器是否已打开。
     * @return 已打开时返回 true。
     */
    bool isOpen() const
    {
        return m_formatContext != nullptr;
    }

    /**
     * 判断容器是否要求编码器输出全局头。
     * @return 需要全局头时返回 true。
     */
    bool needsGlobalHeader() const;

    /**
     * 读取容器写锁，供音频编码器共用。
     * @return 写锁引用。
     */
    std::mutex &writeMutex()
    {
        return m_writeMutex;
    }

private:
    AVFormatContext *m_formatContext = nullptr;
    QByteArray m_outputPathBytes;
    std::mutex m_writeMutex;
};

}  // namespace markshot::recording

#endif  // HAVE_LIBAV_RECORDING
