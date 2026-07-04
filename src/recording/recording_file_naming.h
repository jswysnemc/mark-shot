#pragma once

#include "recording/recording_options.h"

namespace markshot::recording {

/**
 * 生成录制文件的默认保存路径。
 * @param mode 录制模式。
 * @return 默认保存路径。
 */
QString defaultRecordingPath(RecordingMode mode);

/**
 * 在指定目录生成录制文件默认保存路径。
 * @param directory 保存目录。
 * @param mode 录制模式。
 * @return 默认保存路径。
 */
QString defaultRecordingPathInDirectory(const QString &directory, RecordingMode mode);

/**
 * 按录制模式补齐输出文件扩展名。
 * @param path 用户输入路径。
 * @param mode 录制模式。
 * @return 带有正确扩展名的路径。
 */
QString normalizedRecordingPath(QString path, RecordingMode mode);

}  // namespace markshot::recording
