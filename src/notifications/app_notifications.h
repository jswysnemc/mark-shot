#pragma once

#include <QString>

namespace markshot::recording {
struct RecordingOptions;
}

namespace markshot::notifications {

/**
 * 发送桌面通知。
 * @param summary 通知标题。
 * @param body 通知内容。
 * @param timeoutMs 通知超时时间，单位为毫秒。
 * @return 成功提交通知时返回 true。
 */
bool sendDesktopNotification(const QString &summary, const QString &body, int timeoutMs = 2500);

/**
 * 发送截图保存成功通知。
 * @param path 截图保存路径。
 * @return 成功提交通知时返回 true。
 */
bool notifyScreenshotSaved(const QString &path);

/**
 * 发送录制开始通知。
 * @param options 录制配置。
 * @return 成功提交通知时返回 true。
 */
bool notifyRecordingStarted(const recording::RecordingOptions &options);

/**
 * 发送录制保存成功通知。
 * @param path 录制文件保存路径。
 * @return 成功提交通知时返回 true。
 */
bool notifyRecordingSaved(const QString &path);

/**
 * 发送录制失败通知。
 * @param message 失败原因。
 * @return 成功提交通知时返回 true。
 */
bool notifyRecordingFailed(const QString &message);

}  // namespace markshot::notifications
