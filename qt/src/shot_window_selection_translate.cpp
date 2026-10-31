#include "shot_window_selection_translate.h"

#include "shot_window_module.h"

#include "debug_log.h"
#include "selection_translate/translate_language_popup.h"
#include "translation_language_options.h"
#include "translation_layout/block_layout.h"
#include "translation_layout/block_painter.h"
#include "translation_layout/translation_seed.h"
#include "ui/theme.h"

#include <QPainter>
#include <QScopeGuard>
#include <QScopedValueRollback>

using namespace markshot::shot;
using markshot::selection_translate::SelectionTranslateSession;
using markshot::selection_translate::SelectionTranslation;
namespace layout = markshot::translation_layout;

namespace {

/// 关闭时框选页不提供一键翻译
inline constexpr bool FEATURE_SELECTION_TRANSLATE_ENABLED = true;

/**
 * 把保存的区域像素原样写回冻结截图。
 * @param frame 冻结截图，就地修改。
 * @param rect 区域在冻结图中的像素矩形。
 * @param pixels 区域像素。
 * @return 无返回值。
 */
void paintRegion(QImage *frame, const QRect &rect, const QImage &pixels)
{
    if (!frame || pixels.isNull()) {
        return;
    }
    QPainter painter(frame);
    painter.setCompositionMode(QPainter::CompositionMode_Source);
    painter.drawImage(rect.topLeft(), pixels);
}

}  // namespace

void ShotWindow::translateSelection()
{
    if (!FEATURE_SELECTION_TRANSLATE_ENABLED) {
        return;
    }
    ShotWindowSelectionTranslate(*this).toggleTranslationMode();
}

ShotWindowSelectionTranslate::ShotWindowSelectionTranslate(ShotWindow &window)
    : m_window(window)
{
}

void ShotWindowSelectionTranslate::toggleTranslationMode()
{
    // 1. 【框选翻译】【模式开关】语言列表开着时只关闭列表；已在翻译模式时退出
    if (m_window.m_controls.m_translateLanguagePopup && m_window.m_controls.m_translateLanguagePopup->isVisible()) {
        m_window.m_controls.m_translateLanguagePopup->dismiss();
        return;
    }
    if (m_window.m_controls.m_selectionTranslateSession) {
        exitTranslation();
        return;
    }
    openLanguagePopup(false);
}

void ShotWindowSelectionTranslate::translateTo(const QString &targetLanguage)
{
    m_window.commitTextEditor();
    if (targetLanguage.trimmed().isEmpty() || m_window.m_mode != ShotWindow::Mode::Editing) {
        return;
    }
    std::optional<SelectionTranslateSession> &active = m_window.m_controls.m_selectionTranslateSession;
    if (!active && !m_window.hasUsableSelection()) {
        return;
    }
    const PinnedWindowConfig config = pinnedWindowConfig();
    if (!config.ocrEnabled) {
        m_window.showToast(MS_TR("OCR helper not found"));
        return;
    }

    // 1. 【框选翻译】【目标语言】选中的语言作为下次默认值，保存失败不影响本次翻译
    QString error;
    if (!markshot::saveTranslationTargetLanguage(targetLanguage, &error) && !error.isEmpty()) {
        markshot::debugLog("config", "【框选翻译】【目标语言】cannot save translation.targetLanguage: %s",
                           error.toUtf8().constData());
    }

    // 2. 【框选翻译】【译文缓存】翻译过的语言直接切换，不再调用翻译器
    if (active && active->translations.contains(targetLanguage)) {
        active->language = targetLanguage;
        active->showingTranslation = true;
        applyView();
        return;
    }

    // 3. 【框选翻译】【识别】翻译模式内复用识别结果；首次翻译时识别选区
    const auto restoreCursor = qScopeGuard([this] { m_window.updateCursor(); });
    const QScopedValueRollback<bool> busy(m_window.m_operationBusy, true);
    m_window.updateCursor();
    std::optional<SelectionTranslateSession> created;
    if (!active) {
        created = startSession(config);
        if (!created) {
            return;
        }
    }
    SelectionTranslateSession &session = active ? *active : *created;

    // 4. 【框选翻译】【翻译】翻译失败时保持当前显示
    const std::optional<QVector<QString>> texts =
        translateBlocks(config, session.sourceBlocks, targetLanguage, session.originalPixels.size());
    if (!texts) {
        return;
    }

    // 5. 【框选翻译】【写回】译文排进原图后缓存，再写进冻结截图，复制和保存会带上当前显示的内容
    session.translations.insert(targetLanguage, bakeTranslation(session, *texts));
    session.language = targetLanguage;
    session.showingTranslation = true;
    if (created) {
        active = std::move(created);
    }
    applyView();
}

std::optional<SelectionTranslateSession> ShotWindowSelectionTranslate::startSession(
    const PinnedWindowConfig &config) const
{
    const QRect imageRect = m_window.normalizedSelection().toAlignedRect().intersected(m_window.m_frozenFrame.rect());
    if (imageRect.isEmpty()) {
        return std::nullopt;
    }
    // 1. 导出选区后识别文字，坐标保持在裁剪图内
    const QString tempPath = m_window.saveSelectionToTempFile();
    if (tempPath.isEmpty()) {
        return std::nullopt;
    }
    const auto removeTemp = qScopeGuard([&tempPath] { QFile::remove(tempPath); });
    const std::optional<QVector<markshot::ocr::Token>> tokens = recognize(config, tempPath, imageRect.size());
    if (!tokens) {
        return std::nullopt;
    }

    // 2. 相邻行合并成段落，翻译器按整句翻译
    SelectionTranslateSession session;
    session.sourceBlocks = layout::groupParagraphs(*tokens);
    if (session.sourceBlocks.isEmpty()) {
        m_window.showToast(MS_TR("No text to translate"));
        return std::nullopt;
    }
    session.imageRect = imageRect;
    session.originalPixels = m_window.m_frozenFrame.copy(imageRect);
    session.sourceTokens = *tokens;
    return session;
}

SelectionTranslation ShotWindowSelectionTranslate::bakeTranslation(const SelectionTranslateSession &session,
                                                                   const QVector<QString> &texts) const
{
    // 1. 在原图上取色排版，再画到原图副本上
    SelectionTranslation translation;
    translation.blocks = layout::layoutTranslatedBlocks(session.originalPixels, session.sourceBlocks, texts,
                                                        markshot::theme::uiFont());
    translation.pixels = session.originalPixels.copy();
    QPainter painter(&translation.pixels);
    layout::paintTranslatedBlocks(painter, translation.blocks);
    painter.end();
    translation.text = layout::translatedText(translation.blocks);
    return translation;
}

void ShotWindowSelectionTranslate::setShowingTranslation(bool showTranslation)
{
    std::optional<SelectionTranslateSession> &session = m_window.m_controls.m_selectionTranslateSession;
    if (!session || session->showingTranslation == showTranslation) {
        return;
    }
    session->showingTranslation = showTranslation;
    applyView();
}

void ShotWindowSelectionTranslate::exitTranslation()
{
    std::optional<SelectionTranslateSession> &session = m_window.m_controls.m_selectionTranslateSession;
    if (!session) {
        return;
    }
    // 1. 【框选翻译】【退出】写回原图后丢弃缓存，工具栏翻译按钮恢复未选中
    paintRegion(&m_window.m_frozenFrame, session->imageRect, session->originalPixels);
    session.reset();
    hidePanels();
    applyView();
}

void ShotWindowSelectionTranslate::applyView()
{
    const std::optional<SelectionTranslateSession> &session = m_window.m_controls.m_selectionTranslateSession;
    if (session) {
        const SelectionTranslation *translation = session->current();
        const bool showTranslation = session->showingTranslation && translation;
        paintRegion(&m_window.m_frozenFrame, session->imageRect,
                    showTranslation ? translation->pixels : session->originalPixels);
    }
    // 冻结截图像素变化后清掉缩放缓存，再刷新浮条与工具栏选中态
    m_window.m_sharpViewportCache = {};
    updateResultBar();
    m_window.updateToolbarState();
    m_window.update();
}

QImage ShotWindowSelectionTranslate::renderSelectionForPin(layout::TranslationSeed *seed)
{
    std::optional<SelectionTranslateSession> &session = m_window.m_controls.m_selectionTranslateSession;
    if (!session) {
        return m_window.renderedSelection();
    }

    // 1. 【框选翻译】【转贴图】贴图用原图渲染，译文交给贴图窗口自己的原文译文切换
    const SelectionTranslation *translation = session->current();
    const bool showingTranslation = session->showingTranslation && translation;
    if (showingTranslation) {
        paintRegion(&m_window.m_frozenFrame, session->imageRect, session->originalPixels);
    }
    const QImage output = m_window.renderedSelection();
    if (showingTranslation) {
        paintRegion(&m_window.m_frozenFrame, session->imageRect, translation->pixels);
    }
    if (!seed || output.isNull()) {
        return output;
    }

    // 2. 识别结果与译文从翻译区域坐标换到贴图图片坐标，落在贴图外的丢弃
    const QRect selectionRect =
        m_window.normalizedSelection().toAlignedRect().intersected(m_window.m_frozenFrame.rect());
    const QPointF offset = session->imageRect.topLeft() - selectionRect.topLeft();
    const QRectF bounds(QPointF(0.0, 0.0), QSizeF(output.size()));
    for (markshot::ocr::Token token : session->sourceTokens) {
        token.imageRect = token.imageRect.translated(offset).intersected(bounds);
        if (!token.imageRect.isEmpty()) {
            seed->sourceTokens.append(token);
        }
    }
    if (translation) {
        seed->blocks = layout::offsetBlocks(translation->blocks, offset, bounds);
    }
    seed->showTranslation = showingTranslation && !seed->blocks.isEmpty();
    return output;
}
