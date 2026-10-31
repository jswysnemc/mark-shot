#pragma once

#include "ocr_result.h"

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

/// 把框选翻译接入截图窗口：选择目标语言、识别并翻译选区文字、写回译文，以及复制译文和还原原文
class ShotWindowSelectionTranslate {
public:
    explicit ShotWindowSelectionTranslate(ShotWindow &window);

    /**
     * 打开目标语言弹层；弹层已打开时关闭它。没有可用选区或未配置 OCR 时只提示。
     * @return 无返回值。
     */
    void openLanguagePopup();

    /**
     * 记住目标语言，识别选区文字后翻译，并把译文写回冻结截图。
     * 已有译文时先还原原文再识别；本次失败时恢复上一次译文。
     * @param targetLanguage 实际传给翻译器的语言名称。
     * @return 无返回值。
     */
    void translateTo(const QString &targetLanguage);

    /**
     * 把译文区域写回原始像素，并隐藏译文浮条。
     * @return 无返回值。
     */
    void restoreOriginal();

    /**
     * 把译文复制到剪贴板，并提示复制结果。
     * @return 无返回值。
     */
    void copyTranslation();

    /**
     * 按当前选区和工具栏位置刷新译文浮条：编辑选区且选区与译文区域相交时显示，否则隐藏。
     * @return 无返回值。
     */
    void updateResultBar();

    /**
     * 收起目标语言弹层和译文浮条，已写回的译文保留，可在选区回到译文上时再次显示浮条。
     * @return 无返回值。
     */
    void hidePanels();

private:
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
     * 把识别结果翻译成目标语言，失败时提示原因。
     * @param config 翻译配置。
     * @param tokens 识别到的文字。
     * @param targetLanguage 目标语言。
     * @param imageSize 选区图片尺寸，用于校正译文框。
     * @return 译文；失败或译文为空时返回空。
     */
    std::optional<QVector<markshot::ocr::Token>> translate(const markshot::shot::PinnedWindowConfig &config,
                                                           const QVector<markshot::ocr::Token> &tokens,
                                                           const QString &targetLanguage, QSize imageSize) const;

    /**
     * 返回目标语言弹层，首次调用时创建并连接信号。
     * @return 目标语言弹层。
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
