#pragma once

#include "recording/recording_options.h"

#include <QVector>

namespace markshot::recording {

/**
 * 读取当前可用于录制的显示器来源。
 * @return 显示器来源列表，多屏时包含全部显示器来源。
 */
QVector<DisplaySource> availableDisplaySources();

}  // namespace markshot::recording
