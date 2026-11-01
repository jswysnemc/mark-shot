#pragma once

#include "rapid_onnx_session.h"

#include <QImage>
#include <QRect>
#include <QString>
#include <QVector>

namespace markshot::ocr_rapid {

/**
 * PP-OCR DBNet 文本检测模型。
 *
 * 输出轴对齐文本行框；截图场景文字基本水平，省略旋转框以简化后处理。
 */
class RapidDetModel final {
public:
    /** 【OCR】【检测模型】将模型会话绑定到本次运行库 api，不持有跨任务的原生状态。 */
    explicit RapidDetModel(const OrtApi &api) : m_session(api) {}

    /**
     * 加载检测模型。
     * @param modelPath 模型文件路径。
     * @param error 输出错误信息。
     * @return 加载成功时返回 true。
     */
    bool load(const QString &modelPath, QString *error);

    /**
     * 释放检测会话。
     * @return 无返回值。
     */
    void release();

    /**
     * 检测图像中的文本行区域。
     * @param image 输入图像。
     * @param boxes 输出文本行框（原图像素坐标，已按 unclip 比例外扩）。
     * @param error 输出错误信息。
     * @return 检测成功时返回 true。
     */
    bool detect(const QImage &image, QVector<QRect> *boxes, QString *error);

private:
    RapidOnnxSession m_session;
};

}  // namespace markshot::ocr_rapid
