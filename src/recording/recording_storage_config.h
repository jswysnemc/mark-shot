#pragma once

#include "recording/recording_options.h"

#include <QJsonObject>
#include <QString>

namespace markshot::recording {

struct RecordingStorageConfig {
    QString videoDirectory;
    QString gifDirectory;
};

/**
 * 生成录制存储默认配置。
 * @return 默认录制存储配置。
 */
RecordingStorageConfig defaultRecordingStorageConfig();

/**
 * 从应用配置根对象读取录制存储配置。
 * @param root 应用配置根对象。
 * @return 录制存储配置。
 */
RecordingStorageConfig recordingStorageConfigFromRoot(const QJsonObject &root);

/**
 * 读取当前应用配置中的录制存储配置。
 * @return 录制存储配置。
 */
RecordingStorageConfig configuredRecordingStorageConfig();

/**
 * 按录制模式返回配置后的输出目录。
 * @param mode 录制模式。
 * @return 输出目录路径。
 */
QString recordingDirectoryForMode(RecordingMode mode);

/**
 * 规范化用户输入的录制输出目录。
 * @param path 用户输入路径。
 * @param fallback 默认路径。
 * @return 规范化后的目录路径。
 */
QString normalizedRecordingDirectory(QString path, const QString &fallback);

}  // namespace markshot::recording
