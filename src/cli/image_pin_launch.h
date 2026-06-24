#pragma once

#include <QString>

class QWidget;

namespace markshot::cli {

/// @brief 从图片路径创建并显示贴图窗口。
/// @param imagePath 图片文件路径。
/// @param error 失败时写入错误信息。
/// @return 成功时返回贴图窗口，失败时返回 nullptr。
QWidget *launchPinnedImageFromPath(const QString &imagePath, QString *error = nullptr);

}  // namespace markshot::cli
