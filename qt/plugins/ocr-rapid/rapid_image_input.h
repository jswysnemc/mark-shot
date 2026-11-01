#pragma once

#include <QImage>

namespace markshot::ocr_rapid {

/**
 * 【OCR】【输入像素】保留原有 ARGB32 处理语义，兼容时共享不透明 RGB32 像素。
 * @param image 本次识别的输入；外部缓冲在共享期间不得绕过 QImage 修改像素。
 * @return 持有源图寿命的 ARGB32 图像；写入结果时独立复制，不修改输入。
 */
QImage rapidImageInput(const QImage &image);

}  // namespace markshot::ocr_rapid
