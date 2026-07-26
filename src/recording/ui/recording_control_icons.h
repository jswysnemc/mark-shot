#pragma once

#include <QColor>
#include <QIcon>

namespace markshot::recording::ui {

/**
 * 【录制】【控制条】绘制暂停图标。
 * @param ink 图标颜色。
 * @return 暂停图标。
 */
QIcon makeRecordingPauseIcon(const QColor &ink);

/**
 * 【录制】【控制条】绘制继续图标。
 * @param ink 图标颜色。
 * @return 继续图标。
 */
QIcon makeRecordingResumeIcon(const QColor &ink);

/**
 * 【录制】【控制条】绘制停止图标。
 * @param ink 图标颜色。
 * @return 停止图标。
 */
QIcon makeRecordingStopIcon(const QColor &ink);

}  // namespace markshot::recording::ui
