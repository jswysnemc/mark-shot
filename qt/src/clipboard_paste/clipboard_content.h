#pragma once

#include <QFont>
#include <QImage>
#include <QPalette>

class QMimeData;

namespace markshot::clipboard {

/// @brief 将剪贴板图片或纯文字转换为贴图，图片优先且文字不解析 HTML
/// @param mime 剪贴板内容
/// @param font 文字贴图字体
/// @param palette 文字贴图的背景与前景配色
/// @param error 失败原因，允许为空
/// @return 可用于贴图的图像，内容无效或过大时返回空图像
QImage imageFromContent(const QMimeData &mime, const QFont &font, const QPalette &palette,
                        QString *error = nullptr);

}
