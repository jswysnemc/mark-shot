#pragma once

#include "capture_freeze_scope.h"

#include <QList>
#include <QRect>
#include <QString>

class QScreen;

namespace markshot::capture_session {

/**
 * 返回冻结范围日志名称。
 * @param scope 冻结范围枚举值。
 * @return 日志使用的固定字符串。
 */
const char *freezeScopeDebugName(CaptureFreezeScope scope);

/**
 * 判断当前窗口系统是否为 Wayland。
 * @return 当前 Qt 平台为 Wayland 时返回 true。
 */
bool isWaylandPlatform();

/**
 * 判断屏幕列表是否包含不同的设备像素比例。
 * @param screens 当前屏幕列表。
 * @return 存在两个以上不同设备像素比例时返回 true。
 */
bool hasMixedDevicePixelRatios(const QList<QScreen *> &screens);

/**
 * 判断多屏冻结是否需要逐屏捕获。
 * @param screens 当前屏幕列表。
 * @return Wayland 多屏场景返回 true。
 */
bool shouldCaptureScreensIndividually(const QList<QScreen *> &screens);

/**
 * 按平台和屏幕数量判断是否需要逐屏捕获。
 * @param waylandPlatform 当前窗口系统是否为 Wayland。
 * @param screenCount 当前有效显示器数量。
 * @return Wayland 多屏场景返回 true。
 */
bool shouldCaptureScreensIndividually(bool waylandPlatform, int screenCount);

/**
 * 记录截图会话中的屏幕缩放诊断信息。
 * @param screens 当前屏幕列表。
 * @return 无返回值。
 */
void logCaptureSessionScreens(const QList<QScreen *> &screens);

/**
 * 计算全部显示器组成的虚拟桌面几何。
 * @return 虚拟桌面几何。
 */
QRect virtualScreensGeometry();

/**
 * 按显示器名称查找屏幕。
 * @param screenName 显示器名称。
 * @return 匹配到的屏幕；未找到时返回 nullptr。
 */
QScreen *screenByName(const QString &screenName);

}  // namespace markshot::capture_session
