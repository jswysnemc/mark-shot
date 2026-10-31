#pragma once

#include <QImage>
#include <QRect>
#include <QString>

namespace markshot::selection_translate {

/// @brief 【框选翻译】【翻译结果】写回冻结截图的一次译文，保存还原原文所需的像素
struct SelectionTranslateResult {
    // 译文所在区域，冻结截图像素坐标
    QRect imageRect;
    // 写入译文前的区域像素，还原时原样写回
    QImage originalPixels;
    // 写入译文后的区域像素，重新翻译失败时用来恢复上一次译文
    QImage translatedPixels;
    // 按行拼接的译文，供复制
    QString translatedText;
    // 目标语言的界面名称
    QString languageLabel;
};

}  // namespace markshot::selection_translate
