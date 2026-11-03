#pragma once

#include <QImage>

namespace markshot::canvas {

/** 【画布】【图像输入】接收 image 的共享引用或所有权，返回 DPR 为 1 的可绘制图像，保留像素与元数据。 */
QImage canvasImageInput(QImage image);

}  // namespace markshot::canvas
