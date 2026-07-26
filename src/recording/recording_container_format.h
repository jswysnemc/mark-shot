#pragma once

#include <QString>

namespace markshot::recording {

enum class RecordingContainerFormat {
    Mp4,
    Mkv,
};

/**
 * 【录制】【容器格式】读取容器对应的文件扩展名。
 * @param format 容器格式。
 * @return 不含点号的扩展名。
 */
QString recordingContainerExtension(RecordingContainerFormat format);

/**
 * 【录制】【容器格式】读取 FFmpeg 使用的容器名称。
 * @param format 容器格式。
 * @return FFmpeg muxer 名称。
 */
QString recordingContainerMuxerName(RecordingContainerFormat format);

/**
 * 【录制】【容器格式】读取容器的界面显示名称。
 * @param format 容器格式。
 * @return 显示名称。
 */
QString recordingContainerLabel(RecordingContainerFormat format);

/**
 * 【录制】【容器格式】从配置文本解析容器格式。
 * @param text 配置文本。
 * @return 容器格式，无法识别时返回 Mp4。
 */
RecordingContainerFormat recordingContainerFromName(QString text);

/**
 * 【录制】【容器格式】读取容器的配置文本。
 * @param format 容器格式。
 * @return 配置文本。
 */
QString recordingContainerName(RecordingContainerFormat format);

}  // namespace markshot::recording
