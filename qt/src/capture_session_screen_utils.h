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
 * 判断普通区域截图是否需要冻结全部显示器。
 * @param allOutputs 是否已经显式请求全部输出。
 * @param fullscreenAnnotation 是否直接进入全屏标注。
 * @param freezeScope 配置的冻结范围。
 * @param screenCount 当前有效显示器数量。
 * @return 需要冻结全部显示器时返回 true。
 */
bool shouldFreezeAllScreens(bool allOutputs,
                            bool fullscreenAnnotation,
                            CaptureFreezeScope freezeScope,
                            int screenCount);

/**
 * 【截图会话】【窗口放置】判断跨屏冻结覆盖层是否需要绕过窗口管理器。
 * 原生 X11 窗口管理器（如 Muffin、Mutter）会把非全屏的普通窗口约束到扣除面板 strut 后的工作区，
 * 跨越整个虚拟桌面的覆盖层因此被下移或缩小，冻结画面与实时面板同时可见（issue #148）。
 * @param nativeX11Session 当前是否为原生 X11 会话（非 XWayland）。
 * @param spansAllOutputs 覆盖层是否以单个窗口跨越整个虚拟桌面。
 * @param useRegularWindow 是否通过 --xdg-window 显式要求普通窗口。
 * @return 需要设置 Qt::X11BypassWindowManagerHint 时返回 true。
 */
bool shouldBypassWindowManagerForSpanningOverlay(bool nativeX11Session,
                                                 bool spansAllOutputs,
                                                 bool useRegularWindow);

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
