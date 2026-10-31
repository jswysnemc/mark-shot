#pragma once

#include "ocr_result.h"
#include "translation_layout/translated_block.h"

#include <QVector>

namespace markshot::translation_layout {

/// @brief 【贴图】【译文交接】截图窗口在翻译模式下转贴图时交给贴图窗口的识别结果与译文，坐标相对贴图图片
struct TranslationSeed {
    // 原文识别结果，贴图窗口直接复用，不再重新识别
    QVector<markshot::ocr::Token> sourceTokens;
    // 已排好的译文，贴图窗口的“显示译文”直接使用
    QVector<TranslatedBlock> blocks;
    // 转贴图时截图窗口正在显示译文
    bool showTranslation = false;
};

}  // namespace markshot::translation_layout
