#pragma once

#include <QString>

class QWidget;

namespace markshot::clipboard {

/// @brief 从剪贴板创建贴图，不启动截图且不修改剪贴板内容
/// @param error 读取或转换失败时写入原因，允许为空
/// @return 已显示的贴图窗口，失败时为空
QWidget *pinContent(QString *error = nullptr);

/// @brief 从界面入口创建剪贴板贴图，失败时通过桌面通知反馈
/// @return 无返回值
void pinContentOrNotify();

}
