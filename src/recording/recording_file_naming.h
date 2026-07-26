#pragma once

#include "recording/recording_options.h"

namespace markshot::recording {

/**
 * 生成录制文件的默认保存路径。
 * @param mode 录制模式。
 * @param container 视频容器格式，GIF 模式忽略该参数。
 * @return 默认保存路径。
 */
QString defaultRecordingPath(RecordingMode mode,
                             RecordingContainerFormat container = RecordingContainerFormat::Mp4);

/**
 * 在指定目录生成录制文件默认保存路径。
 * @param directory 保存目录。
 * @param mode 录制模式。
 * @param container 视频容器格式，GIF 模式忽略该参数。
 * @return 默认保存路径。
 */
QString defaultRecordingPathInDirectory(const QString &directory,
                                        RecordingMode mode,
                                        RecordingContainerFormat container = RecordingContainerFormat::Mp4);

/**
 * 按录制模式与容器格式补齐输出文件扩展名。
 * @param path 用户输入路径。
 * @param mode 录制模式。
 * @param container 视频容器格式，GIF 模式忽略该参数。
 * @return 带有正确扩展名的路径。
 */
QString normalizedRecordingPath(QString path,
                                RecordingMode mode,
                                RecordingContainerFormat container = RecordingContainerFormat::Mp4);

}  // namespace markshot::recording
