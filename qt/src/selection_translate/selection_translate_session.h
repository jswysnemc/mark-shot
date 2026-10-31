#pragma once

#include "ocr_result.h"
#include "translation_layout/paragraph_blocks.h"
#include "translation_layout/translated_block.h"

#include <QHash>
#include <QImage>
#include <QRect>
#include <QString>
#include <QVector>

namespace markshot::selection_translate {

/// @brief 【框选翻译】【译文缓存】一种目标语言的译文
struct SelectionTranslation {
    // 写入译文后的区域像素
    QImage pixels;
    // 按段落换行拼接的译文，供复制
    QString text;
    // 排好的译文段落，坐标相对区域左上角
    QVector<markshot::translation_layout::TranslatedBlock> blocks;
};

/// @brief 【框选翻译】【翻译模式】一次框选翻译的状态：原图、识别结果和各语言译文的缓存
///
/// 原文与译文之间切换只交换区域像素；换一种语言时复用识别结果，只重新翻译
struct SelectionTranslateSession {
    // 翻译区域，冻结截图像素坐标
    QRect imageRect;
    // 写入译文前的区域像素
    QImage originalPixels;
    // 识别结果，坐标相对区域左上角
    QVector<markshot::ocr::Token> sourceTokens;
    // 由识别结果合并出的原文段落，坐标相对区域左上角
    QVector<markshot::translation_layout::SourceBlock> sourceBlocks;
    // 按目标语言缓存的译文
    QHash<QString, SelectionTranslation> translations;
    // 当前目标语言
    QString language;
    // 区域当前显示译文；为 false 时显示原文，仍处于翻译模式
    bool showingTranslation = true;

    /** 返回当前语言的译文，参数无，尚未翻译时返回空指针。 */
    const SelectionTranslation *current() const
    {
        const auto it = translations.constFind(language);
        return it == translations.constEnd() ? nullptr : &it.value();
    }
};

}  // namespace markshot::selection_translate
