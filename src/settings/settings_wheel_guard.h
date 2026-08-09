#pragma once

#include <QObject>

class QWidget;

namespace markshot::settings {

/// @brief 为设置对话框安装滚轮防护。
///
/// 防止在设置页面上滚动时，悬停于下拉框/数值框等控件上导致其内容被误改：
/// 未聚焦的 QComboBox / QAbstractSpinBox 收到的滚轮事件会被换算为对外层
/// QScrollArea 滚动条的滚动，让页面正常翻页；控件聚焦时仍保留滚轮调整值
/// 的能力。
/// 返回的过滤器对象由 dialog 拥有，随对话框销毁自动卸载。
/// @param dialog 设置对话框。
/// @return 安装的事件过滤器对象。
QObject *installSettingsWheelGuard(QWidget *dialog);

}  // namespace markshot::settings
