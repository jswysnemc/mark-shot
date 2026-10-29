#include "shot_window_module.h"

#include "selection_aspect/selection_aspect_ratio.h"
#include "selection_aspect/selection_hud_layout.h"
#include "selection_aspect/selection_size_panel.h"

using namespace markshot::shot;

namespace aspect = markshot::selection_aspect;

namespace {

/**
 * 返回截图窗口直接子控件在可见时的矩形，用于 HUD 避让。
 * @param owner 截图窗口。
 * @param widget 待检查控件。
 * @return 可见的直接子控件返回其几何，否则返回空矩形。
 */
QRect visibleChildRect(const QWidget *owner, const QWidget *widget)
{
    if (!widget || !widget->isVisible() || widget->parentWidget() != owner) {
        return {};
    }
    return widget->geometry();
}

}  // namespace

QRectF ShotWindow::creationSelectionRect(QPointF anchor, QPointF pointer, bool squareModifier) const
{
    // 锁定比例优先；未锁定时按住 Shift 拖出正方形
    const qreal ratio = m_selectionAspectRatio > 0.0 ? m_selectionAspectRatio : (squareModifier ? 1.0 : 0.0);
    if (ratio <= 0.0) {
        return normalizedRect(anchor, pointer);
    }
    return aspect::constrainedCreationRect(anchor, pointer, ratio, m_frozenFrame.size());
}

QRectF ShotWindow::aspectLockedSelection(QRectF before, SelectionDrag handle, QRectF adjusted) const
{
    if (m_selectionAspectRatio <= 0.0) {
        return adjusted;
    }
    return aspect::constrainedAdjustedRect(before, handle, adjusted, m_selectionAspectRatio, m_frozenFrame.size());
}

aspect::SelectionSizePanel *ShotWindow::ensureSelectionSizePanel()
{
    if (auto *panel = qobject_cast<aspect::SelectionSizePanel *>(m_selectionSizePanel)) {
        return panel;
    }

    // 1. 【截图】【选区尺寸】首次使用时创建 HUD 并连接修改信号
    auto *panel = new aspect::SelectionSizePanel(this);
    panel->hide();
    connect(panel, &aspect::SelectionSizePanel::sizeApplied, this, &ShotWindow::applySelectionSize);
    connect(panel, &aspect::SelectionSizePanel::aspectRatioChanged, this, &ShotWindow::setSelectionAspectRatio);
    connect(panel, &aspect::SelectionSizePanel::focusReleaseRequested, this, [this] { setFocus(Qt::OtherFocusReason); });
    connect(panel, &aspect::SelectionSizePanel::layoutRequested, this, &ShotWindow::updateSelectionSizePanelGeometry);

    // 2. 【截图】【选区尺寸】标注文本输入框与工具栏变化时重新避让
    panel->watchObstacle(m_textEditor);
    panel->watchObstacle(m_controls.m_toolbar);
    panel->watchObstacle(m_controls.m_actionToolbar);
    panel->watchObstacle(m_controls.m_annotationPropertyPanel);
    m_selectionSizePanel = panel;
    return panel;
}

void ShotWindow::focusSelectionSizeInput()
{
    updateSelectionSizePanelGeometry();
    auto *panel = qobject_cast<aspect::SelectionSizePanel *>(m_selectionSizePanel);
    if (panel && panel->isVisible()) {
        panel->focusWidth();
    }
}

void ShotWindow::updateSelectionSizePanelGeometry()
{
    // 1. 只在编辑普通选区时常驻；创建选区和全屏标注阶段隐藏
    const bool wanted = m_mode == Mode::Editing && !m_fullscreenAnnotation && hasUsableSelection();
    if (!wanted) {
        if (m_selectionSizePanel) {
            m_selectionSizePanel->hide();
        }
        return;
    }
    aspect::SelectionSizePanel *panel = ensureSelectionSizePanel();

    // 2. 同步实时像素尺寸与锁定比例
    const QRectF selection = normalizedSelection();
    panel->setValues(QSize(qRound(selection.width()), qRound(selection.height())), m_frozenFrame.size(),
                     m_selectionAspectRatio);

    // 3. 优先避让标注文本输入框和工具栏，放不下时只避让文本输入框，仍冲突则隐藏
    aspect::SelectionHudLayoutInput input;
    input.selection = imageRectToWidget(selection).toAlignedRect();
    input.hudSize = panel->sizeHint();
    input.bounds = rect();
    const QRect textEditorRect = visibleChildRect(this, m_textEditor);
    // 形状网格弹层与文本输入框同属必须避让项，冲突时宁可隐藏状态条
    const QRect shapePopupRect = visibleChildRect(this, m_controls.m_shapeMarkerPopup);
    input.obstacles = {textEditorRect, shapePopupRect, visibleChildRect(this, m_controls.m_toolbar),
                       visibleChildRect(this, m_controls.m_actionToolbar),
                       visibleChildRect(this, m_controls.m_annotationPropertyPanel)};
    std::optional<QRect> placed = aspect::placeSelectionHud(input);
    const bool overlapsToolbars = !placed.has_value();
    if (overlapsToolbars) {
        input.obstacles = {textEditorRect, shapePopupRect};
        placed = aspect::placeSelectionHud(input);
    }
    if (!placed.has_value()) {
        panel->hide();
        return;
    }
    panel->setGeometry(*placed);
    if (!panel->isVisible()) {
        panel->show();
        panel->raise();
    }
    if (overlapsToolbars && m_controls.m_toolbar) {
        // 4. 只能与工具栏重叠时把状态条压到工具栏下层，选区贴着停靠的工具栏时按钮仍可点击
        panel->stackUnder(m_controls.m_toolbar);
    }
}

void ShotWindow::setSelectionAspectRatio(qreal ratio)
{
    m_selectionAspectRatio = std::max<qreal>(0.0, ratio);
    if (m_selectionAspectRatio > 0.0 && hasUsableSelection()) {
        // 以左上角为基准保持宽度按新比例改高度，放不下时再按高度收缩宽度
        const QRectF current = normalizedSelection();
        const QSize target = aspect::linkedSize(QSize(qRound(current.width()), qRound(current.height())),
                                                aspect::SizeField::Width, m_selectionAspectRatio,
                                                m_frozenFrame.size());
        applySelectionSize(target);
    }
    // 尺寸未变化或被拒绝时也要刷新 HUD 上的锁定状态
    updateSelectionSizePanelGeometry();
    update();
}

void ShotWindow::applySelectionSize(QSize size)
{
    if (!hasUsableSelection() || size.isEmpty()) {
        return;
    }
    const QRectF resized = aspect::resizedSelection(normalizedSelection(), size, m_frozenFrame.size());
    if (resized.width() < kMinSelectionSize || resized.height() < kMinSelectionSize) {
        showToast(MS_TR("Selection is too small"), 1400);
        updateSelectionSizePanelGeometry();
        return;
    }
    m_selection = resized;
    refreshAdjustedSelection();
}
