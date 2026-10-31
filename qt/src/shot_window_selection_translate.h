#pragma once

#include "ocr_result.h"
#include "selection_translate/selection_translate_session.h"

#include <QImage>
#include <QRect>
#include <QSize>
#include <QString>
#include <QVector>

#include <optional>

class ShotWindow;

namespace markshot::shot {
struct PinnedWindowConfig;
}

namespace markshot::selection_translate {
class TranslateLanguagePopup;
class TranslateResultBar;
}

namespace markshot::translation_layout {
struct TranslationSeed;
}

/// 把框选翻译接入截图窗口：工具栏翻译按钮作为翻译模式开关，模式内可换语言、在原文与译文间切换、复制译文，
/// 转贴图时交出原图与已有译文
class ShotWindowSelectionTranslate {
public:
    explicit ShotWindowSelectionTranslate(ShotWindow &window);

    /**
     * 工具栏翻译按钮的处理：不在翻译模式时打开目标语言列表，已在翻译模式时退出并还原原图。
     * @return 无返回值。
     */
    void toggleTranslationMode();

    /**
     * 打开目标语言列表；列表已打开时关闭它。没有可用选区或未配置 OCR 时只提示。
     * @param anchorToResultBar 为 true 时贴着译文浮条弹出，否则贴着工具栏翻译按钮。
     * @return 无返回值。
     */
    void openLanguagePopup(bool anchorToResultBar);

    /**
     * 切换到目标语言的译文：已缓存时直接显示；翻译模式内复用识别结果只重新翻译；
     * 不在翻译模式时先识别选区再翻译，成功后进入翻译模式。失败时保持原来的显示。
     * @param targetLanguage 实际传给翻译器的语言名称。
     * @return 无返回值。
     */
    void translateTo(const QString &targetLanguage);

    /**
     * 在原文与译文之间切换显示，保持翻译模式。
     * @param showTranslation 为 true 时显示译文。
     * @return 无返回值。
     */
    void setShowingTranslation(bool showTranslation);

    /**
     * 退出翻译模式：写回原图，丢弃识别结果与译文缓存，收起语言列表和译文浮条。
     * @return 无返回值。
     */
    void exitTranslation();

    /**
     * 把当前语言的译文复制到剪贴板，并提示复制结果。
     * @return 无返回值。
     */
    void copyTranslation();

    /**
     * 按当前选区和工具栏位置刷新译文浮条：编辑选区且选区与翻译区域相交时显示，否则隐藏。
     * @return 无返回值。
     */
    void updateResultBar();

    /**
     * 收起目标语言列表和译文浮条，翻译模式保留，选区回到翻译区域上时再次显示浮条。
     * @return 无返回值。
     */
    void hidePanels();

    /**
     * 为转贴图渲染选区：翻译模式下用原图渲染，并交出相对贴图图片的识别结果和译文；
     * 不在翻译模式时与普通渲染一致。
     * @param seed 输出贴图窗口使用的识别结果与译文，不在翻译模式时保持为空。
     * @return 渲染出的选区图片。
     */
    QImage renderSelectionForPin(markshot::translation_layout::TranslationSeed *seed);

private:
    /**
     * 识别选区并建立翻译模式状态，尚未写入冻结截图。
     * @param config OCR 配置。
     * @return 识别成功且有文字时返回新状态，否则提示原因并返回空。
     */
    std::optional<markshot::selection_translate::SelectionTranslateSession>
    startSession(const markshot::shot::PinnedWindowConfig &config) const;

    /**
     * 把译文排进原图并生成译文缓存。
     * @param session 翻译模式状态。
     * @param texts 与段落一一对应的译文。
     * @return 一种目标语言的译文。
     */
    markshot::selection_translate::SelectionTranslation
    bakeTranslation(const markshot::selection_translate::SelectionTranslateSession &session,
                    const QVector<QString> &texts) const;

    /**
     * 按翻译模式状态把原文或译文像素写回冻结截图，并刷新画布、浮条和工具栏。
     * @return 无返回值。
     */
    void applyView();

    /**
     * 识别临时选区图片中的文字，失败时提示原因。
     * @param config OCR 配置。
     * @param imagePath 选区临时图片路径。
     * @param imageSize 选区图片尺寸，用于校正识别框。
     * @return 识别到的文字；失败或没有文字时返回空。
     */
    std::optional<QVector<markshot::ocr::Token>> recognize(const markshot::shot::PinnedWindowConfig &config,
                                                           const QString &imagePath, QSize imageSize) const;

    /**
     * 把原文段落翻译成目标语言，失败时提示原因。
     * @param config 翻译配置。
     * @param blocks 原文段落。
     * @param targetLanguage 目标语言。
     * @param imageSize 选区图片尺寸，用于校正译文框。
     * @return 与段落一一对应的译文；失败或译文全空时返回空。
     */
    std::optional<QVector<QString>> translateBlocks(const markshot::shot::PinnedWindowConfig &config,
                                                    const QVector<markshot::translation_layout::SourceBlock> &blocks,
                                                    const QString &targetLanguage, QSize imageSize) const;

    /**
     * 返回目标语言列表，首次调用时创建并连接信号。
     * @return 目标语言列表。
     */
    markshot::selection_translate::TranslateLanguagePopup *languagePopup() const;

    /**
     * 返回译文浮条，首次调用时创建并连接信号。
     * @return 译文浮条。
     */
    markshot::selection_translate::TranslateResultBar *resultBar() const;

    /**
     * 查找可见的翻译按钮。
     * @param verticalToolbar 输出按钮所在工具栏是否竖向排列。
     * @return 按钮在截图窗口中的矩形，没有可见按钮时返回空矩形。
     */
    QRect translateButtonRect(bool *verticalToolbar) const;

    ShotWindow &m_window;
};
