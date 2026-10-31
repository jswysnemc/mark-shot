#include "shot_window_selection_translate.h"

#include "shot_window_module.h"

#include "selection_translate/selection_translate_placement.h"
#include "selection_translate/translate_language_popup.h"
#include "selection_translate/translate_result_bar.h"
#include "translation_language_options.h"
#include "ui/compact_toolbar.h"

using namespace markshot::shot;
namespace translate = markshot::selection_translate;

namespace {

/**
 * 查找目标语言的界面名称。
 * @param value 实际传给翻译器的语言名称。
 * @return 内置语言返回当前界面语言的名称，自定义语言原样返回。
 */
QString languageLabel(const QString &value)
{
    for (const markshot::TranslationLanguageOption &option : markshot::translationLanguageOptions()) {
        if (option.value == value) {
            return option.label;
        }
    }
    return value;
}

}  // namespace

void ShotWindowSelectionTranslate::openLanguagePopup(bool anchorToResultBar)
{
    m_window.commitTextEditor();
    const std::optional<translate::SelectionTranslateSession> &session =
        m_window.m_controls.m_selectionTranslateSession;
    if (m_window.m_mode != ShotWindow::Mode::Editing || (!session && !m_window.hasUsableSelection())) {
        return;
    }
    translate::TranslateLanguagePopup *popup = languagePopup();
    if (popup->isVisible()) {
        popup->dismiss();
        return;
    }
    // 1. 【框选翻译】【目标语言】未配置 OCR 时直接提示，不让用户选完语言才失败
    const PinnedWindowConfig config = pinnedWindowConfig();
    if (!config.ocrEnabled) {
        m_window.showToast(MS_TR("OCR helper not found"));
        return;
    }

    // 2. 收起同一位置的其他面板，避免与语言列表叠在一起
    for (QWidget *panel : {m_window.m_controls.m_openWithPanel, m_window.m_controls.m_extensionPanel,
                           m_window.m_controls.m_colorPalette}) {
        if (panel) {
            panel->hide();
        }
    }

    // 3. 当前目标语言打勾：翻译模式内取正在显示的语言，否则取配置
    popup->setLanguages(markshot::translationLanguageOptions(),
                        session ? session->language : config.translationTargetLanguage);
    const QSize popupSize = popup->sizeHint();

    // 4. 从译文浮条打开时贴着浮条上下弹出；否则贴着翻译按钮，按钮不可见时退回侧边面板位置
    const translate::TranslateResultBar *bar = m_window.m_controls.m_translateResultBar;
    if (anchorToResultBar && bar && bar->isVisible()) {
        popup->popup(translate::languagePopupGeometry(bar->geometry(), false, popupSize, m_window.rect()));
        return;
    }
    bool verticalToolbar = true;
    const QRect anchor = translateButtonRect(&verticalToolbar);
    const QRect geometry = anchor.isEmpty()
        ? m_window.sidePanelGeometry(popupSize)
        : translate::languagePopupGeometry(anchor, verticalToolbar, popupSize, m_window.rect());
    popup->popup(geometry);
}

void ShotWindowSelectionTranslate::copyTranslation()
{
    const std::optional<translate::SelectionTranslateSession> &session =
        m_window.m_controls.m_selectionTranslateSession;
    const translate::SelectionTranslation *translation = session ? session->current() : nullptr;
    if (!translation) {
        return;
    }
    m_window.showToast(markshot::copyTextToClipboard(translation->text) ? MS_TR("Translation copied")
                                                                       : MS_TR("Copy failed"));
}

void ShotWindowSelectionTranslate::updateResultBar()
{
    const std::optional<translate::SelectionTranslateSession> &session =
        m_window.m_controls.m_selectionTranslateSession;
    const bool visible = session && m_window.m_mode == ShotWindow::Mode::Editing && m_window.hasUsableSelection()
        && m_window.normalizedSelection().intersects(QRectF(session->imageRect));
    if (!visible) {
        if (m_window.m_controls.m_translateResultBar) {
            m_window.m_controls.m_translateResultBar->hide();
        }
        return;
    }

    // 1. 【框选翻译】【译文浮条】浮条贴在译文区域旁边，避开工具栏、属性栏和选区尺寸提示
    translate::TranslateResultBar *bar = resultBar();
    bar->setLanguageLabel(languageLabel(session->language));
    bar->setShowingTranslation(session->showingTranslation && session->current());
    QVector<QRect> obstacles;
    for (const QWidget *panel : {m_window.m_controls.m_toolbar, m_window.m_controls.m_actionToolbar,
                                 m_window.m_controls.m_annotationPropertyPanel, m_window.m_selectionSizePanel}) {
        if (panel && panel->isVisible()) {
            obstacles.append(panel->geometry());
        }
    }
    const QRect region = m_window.imageRectToWidget(QRectF(session->imageRect)).toAlignedRect();
    bar->setGeometry(translate::resultBarGeometry(region, bar->sizeHint(), m_window.rect(), obstacles));
    bar->show();
    bar->raise();
}

void ShotWindowSelectionTranslate::hidePanels()
{
    if (m_window.m_controls.m_translateLanguagePopup) {
        m_window.m_controls.m_translateLanguagePopup->dismiss();
    }
    if (m_window.m_controls.m_translateResultBar) {
        m_window.m_controls.m_translateResultBar->hide();
    }
}

translate::TranslateLanguagePopup *ShotWindowSelectionTranslate::languagePopup() const
{
    translate::TranslateLanguagePopup *&popup = m_window.m_controls.m_translateLanguagePopup;
    if (!popup) {
        // 1. 弹层挂在截图窗口下，关闭后把键盘焦点还给截图窗口
        ShotWindow *window = &m_window;
        popup = new translate::TranslateLanguagePopup(window);
        popup->setToolbarPalette(window->m_toolbarAppearance.palette);
        QObject::connect(popup, &translate::TranslateLanguagePopup::closed, window,
                         [window] { window->setFocus(Qt::OtherFocusReason); });
        // 2. 选中语言后立即翻译，信号发出前弹层已关闭
        QObject::connect(popup, &translate::TranslateLanguagePopup::languageChosen, window,
                         [window](const QString &value) { ShotWindowSelectionTranslate(*window).translateTo(value); });
    }
    return popup;
}

translate::TranslateResultBar *ShotWindowSelectionTranslate::resultBar() const
{
    translate::TranslateResultBar *&bar = m_window.m_controls.m_translateResultBar;
    if (!bar) {
        ShotWindow *window = &m_window;
        bar = new translate::TranslateResultBar(window->m_toolbarAppearance.palette, window);
        // 1. 浮条上的语言、原文译文切换、复制和退出分别交给翻译模式处理
        QObject::connect(bar, &translate::TranslateResultBar::languageRequested, window,
                         [window] { ShotWindowSelectionTranslate(*window).openLanguagePopup(true); });
        QObject::connect(bar, &translate::TranslateResultBar::viewChanged, window, [window](bool showTranslation) {
            ShotWindowSelectionTranslate(*window).setShowingTranslation(showTranslation);
        });
        QObject::connect(bar, &translate::TranslateResultBar::copyRequested, window,
                         [window] { ShotWindowSelectionTranslate(*window).copyTranslation(); });
        QObject::connect(bar, &translate::TranslateResultBar::exitRequested, window,
                         [window] { ShotWindowSelectionTranslate(*window).exitTranslation(); });
    }
    return bar;
}

QRect ShotWindowSelectionTranslate::translateButtonRect(bool *verticalToolbar) const
{
    // 翻译按钮可能在副工具栏，也可能在全屏标注时的主工具栏，取第一个可见的
    const QString actionKey = markshot::ui::actionName(ShotWindow::Action::Translate);
    for (markshot::ui::CompactToolbar *toolbar :
         {m_window.m_controls.m_actionToolbarPanel, m_window.m_controls.m_toolbarPanel}) {
        QPushButton *button = toolbar ? toolbar->button(actionKey) : nullptr;
        if (!button || !button->isVisibleTo(&m_window) || !toolbar->isVisible()) {
            continue;
        }
        if (verticalToolbar) {
            *verticalToolbar = toolbar->height() > toolbar->width();
        }
        return QRect(button->mapTo(&m_window, QPoint(0, 0)), button->size());
    }
    return {};
}
