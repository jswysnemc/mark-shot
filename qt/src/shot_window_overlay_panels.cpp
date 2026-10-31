#include "shot_window_module.h"

#include "ui/compact_toolbar.h"

#include <QSet>

using namespace markshot::shot;

/// @brief 切换工具颜色浮层的显示状态
/// @param position 浮层锚点所在窗口坐标
/// @return 无返回值
void ShotWindow::toggleColorPalette(QPoint position)
{
    commitTextEditor();
    if (m_controls.m_openWithPanel) {
        m_controls.m_openWithPanel->hide();
    }
    if (m_controls.m_extensionPanel) {
        m_controls.m_extensionPanel->hide();
    }
    if (!m_controls.m_colorPalette) {
        return;
    }

    m_controls.m_colorPaletteAnchor = position;
    if (m_controls.m_colorPalette->isVisible()) {
        m_controls.m_colorPalette->hide();
    } else {
        updateColorPaletteGeometry(position);
        m_controls.m_colorPalette->show();
        m_controls.m_colorPalette->raise();
    }
    update();
}

/// @brief 更新工具颜色浮层及其环形按钮位置
/// @param anchor 浮层锚点所在窗口坐标
/// @return 无返回值
void ShotWindow::updateColorPaletteGeometry(QPoint anchor)
{
    if (!m_controls.m_colorPalette) {
        return;
    }

    const QSize paletteSize(178, 178);
    int x = anchor.x() - paletteSize.width() / 2;
    int y = anchor.y() - paletteSize.height() / 2;
    x = std::clamp(x, 8, std::max(8, width() - paletteSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - paletteSize.height() - 8));
    m_controls.m_colorPalette->setGeometry(x, y, paletteSize.width(), paletteSize.height());

    const QPoint center(paletteSize.width() / 2, paletteSize.height() / 2);
    const qreal radius = 68.0;
    const auto buttons = m_controls.m_colorPalette->findChildren<QPushButton *>(QString(), Qt::FindDirectChildrenOnly);
    for (int i = 0; i < buttons.size(); ++i) {
        const qreal angle = -M_PI / 2.0 + (2.0 * M_PI * i / std::max<qsizetype>(1, buttons.size()));
        const QPoint pos(qRound(center.x() + std::cos(angle) * radius - 15.0),
                         qRound(center.y() + std::sin(angle) * radius - 15.0));
        buttons.at(i)->setGeometry(QRect(pos, QSize(30, 30)));
    }
    updateColorPalettePreview();
}

/// @brief 更新工具颜色浮层中央的当前工具尺寸预览
/// @return 无返回值
void ShotWindow::updateColorPalettePreview()
{
    if (!m_controls.m_colorPalettePreview) {
        return;
    }

    const int size = std::clamp(qRound(currentToolPreviewSize()), 8, 34);
    const QPoint center(89, 89);
    m_controls.m_colorPalettePreview->setGeometry(center.x() - size / 2, center.y() - size / 2, size, size);
    m_controls.m_colorPalettePreview->setStyleSheet(QStringLiteral(
        "QWidget#colorPalettePreview {"
        " background: %1;"
        " border: 0;"
        " border-radius: 3px;"
        "}").arg(m_currentColor.name()));
}

/// @brief 根据文本标注位置和选区边界更新文本编辑器几何
/// @return 无返回值
void ShotWindow::updateTextEditorGeometry()
{
    if (!m_textEditor || !m_textEditor->isVisible()) {
        return;
    }
    // 1. 【标注】【文字编辑】snow 输入框按文字内容摆放，不再使用固定尺寸的旧输入框
    if (applySnowTextEditorGeometryIfEnabled()) {
        return;
    }
    auto keepEditorInsideWindow = [this](QRect editorRect) {
        const int minLeft = 8;
        const int minTop = 8;
        const int maxLeft = std::max(minLeft, width() - editorRect.width() - 8);
        const int maxTop = std::max(minTop, height() - editorRect.height() - 8);
        editorRect.moveLeft(std::clamp(editorRect.left(), minLeft, maxLeft));
        editorRect.moveTop(std::clamp(editorRect.top(), minTop, maxTop));
        return editorRect;
    };

    if (m_editingTextAnnotationId.has_value()) {
        if (const Annotation *annotation = annotationById(*m_editingTextAnnotationId)) {
            QRect editorRect = textContentRect(*annotation, true).toAlignedRect().adjusted(0, 0, 1, 1);
            m_textEditor->setGeometry(keepEditorInsideWindow(editorRect));
        }
        return;
    }

    const QPointF topLeft = imageToWidget(m_textEditorImagePoint);
    const QRectF selection = imageRectToWidget(normalizedSelection());
    constexpr int kMinTextEditorWidth = 96;
    constexpr int kMinTextEditorHeight = 38;
    const int spaceRight = qRound(selection.right() - topLeft.x() - 12);
    const int spaceLeft = qRound(topLeft.x() - selection.left() - 12);
    const int spaceBottom = qRound(selection.bottom() - topLeft.y() - 12);
    const int spaceTop = qRound(topLeft.y() - selection.top() - 12);
    const int availableRight = std::max(kMinTextEditorWidth, std::max(spaceRight, spaceLeft));
    const int availableBottom = std::max(kMinTextEditorHeight, std::max(spaceBottom, spaceTop));
    const int editorWidth = std::clamp(220, kMinTextEditorWidth, availableRight);
    const int editorHeight = std::clamp(m_textEditor->fontMetrics().height() + 18, kMinTextEditorHeight, availableBottom);
    QPoint editorTopLeft = topLeft.toPoint();
    if (spaceRight < kMinTextEditorWidth && spaceLeft > spaceRight) {
        editorTopLeft.setX(qRound(topLeft.x()) - editorWidth);
    }
    if (spaceBottom < kMinTextEditorHeight && spaceTop > spaceBottom) {
        editorTopLeft.setY(qRound(topLeft.y()) - editorHeight);
    }
    m_textEditor->setGeometry(keepEditorInsideWindow(QRect(editorTopLeft, QSize(editorWidth, editorHeight))));
    m_textEditor->ensureCursorVisible();
}

/// @brief 根据当前工具和窗口状态刷新工具栏按钮选中态
/// @return 无返回值
void ShotWindow::updateToolbarState()
{
    if (!m_controls.m_toolbarPanel) {
        return;
    }

    // 1. 汇总当前工具、截图范围与排列方向对应的选中动作
    QSet<QString> activeActions{currentToolName()};
    if (m_fullscreenAnnotation) {
        activeActions.insert(markshot::ui::actionName(Action::ToggleCaptureScope));
    }
    if (m_controls.m_toolbarVerticalLayout) {
        activeActions.insert(markshot::ui::actionName(Action::ToggleToolbarLayout));
    }
    // 2. 主副工具栏统一应用，组件只重绘状态变化的按钮
    m_controls.m_toolbarPanel->setActiveActions(activeActions);
    if (m_controls.m_actionToolbarPanel) {
        m_controls.m_actionToolbarPanel->setActiveActions(activeActions);
    }
}


/**
 * 【标注工具栏】【形状弹层】隐藏形状网格弹层，并恢复被其遮挡的选区尺寸状态条。
 * @return 无返回值。
 */
void ShotWindow::hideShapeMarkerPopup()
{
    if (m_controls.m_shapeMarkerPopup && m_controls.m_shapeMarkerPopup->isVisible()) {
        m_controls.m_shapeMarkerPopup->hide();
        // 弹层关闭后恢复选区尺寸状态条
        updateSelectionSizePanelGeometry();
    }
}

void ShotWindow::refreshShapeMarkerToolbarButton()
{
    // 1. 工具栏主按钮图标跟随当前默认形状
    if (!m_controls.m_shapeMarkerToolbarButton) {
        return;
    }
    rememberShapeMarkerToolbarIconIfEnabled();
    m_controls.m_shapeMarkerToolbarButton->setIcon(
        markshot::ui::makeMarkerShapeIcon(m_markerShape, m_toolbarAppearance.palette.text, m_markerFilled));
    m_controls.m_shapeMarkerToolbarButton->setToolTip(
        QStringLiteral("%1 (%2)").arg(
            markshot::i18n::translate(markshot::marker::shapeLabel(m_markerShape)),
            shortcutText(Tool::Marker)));

    // 2. 同步弹层中按钮选中态；形状按钮图标跟随当前填充模式渲染
    if (!m_controls.m_shapeMarkerPopup) {
        return;
    }
    const auto buttons = m_controls.m_shapeMarkerPopup->findChildren<QPushButton *>();
    for (QPushButton *button : buttons) {
        bool selected = false;
        const QVariant shapeProperty = button->property("markerShape");
        const QVariant fillProperty = button->property("markerFillMode");
        if (shapeProperty.isValid()) {
            selected = shapeProperty.toInt() == static_cast<int>(m_markerShape);
            button->setIcon(markshot::ui::makeMarkerShapeIcon(static_cast<MarkerShape>(shapeProperty.toInt()),
                                                              m_toolbarAppearance.palette.text, m_markerFilled));
        } else if (fillProperty.isValid()) {
            selected = fillProperty.toBool() == m_markerFilled;
        }
        const QSignalBlocker blocker(button);
        button->setChecked(selected);
        button->setProperty("role", selected ? QStringLiteral("primary") : QVariant());
        button->style()->unpolish(button);
        button->style()->polish(button);
    }
}

/**
 * 【标注工具栏】【形状弹层】把形状网格弹层放在属性栏下方 6-8px，并通知选区尺寸状态条重新避让。
 * @return 无返回值。
 */
void ShotWindow::updateShapeMarkerPopupGeometry()
{
    if (!m_controls.m_shapeMarkerPopup || !m_controls.m_shapeMarkerPopup->isVisible() || !m_controls.m_shapeMarkerToolbarButton) {
        return;
    }
    m_controls.m_shapeMarkerPopup->adjustSize();
    const QSize popupSize = m_controls.m_shapeMarkerPopup->sizeHint().expandedTo(m_controls.m_shapeMarkerPopup->size());
    // 1. 左边界对齐工具栏形状按钮；纵向锚定属性栏，属性栏隐藏时锚定工具栏按钮
    const QRect buttonRect(m_controls.m_shapeMarkerToolbarButton->mapTo(this, QPoint(0, 0)),
                           m_controls.m_shapeMarkerToolbarButton->size());
    const QRect anchorRect = m_controls.m_annotationPropertyPanel && m_controls.m_annotationPropertyPanel->isVisible()
        ? m_controls.m_annotationPropertyPanel->geometry()
        : buttonRect;
    m_controls.m_shapeMarkerPopup->setGeometry(
        markshot::shot_popup::popupBelowAnchor(anchorRect, buttonRect.left(), popupSize, rect()));
    m_controls.m_shapeMarkerPopup->raise();
    // 2. 选区尺寸状态条按新弹层位置重新避让，无处可放时隐藏
    updateSelectionSizePanelGeometry();
}

void ShotWindow::toggleShapeMarkerPopup()
{
    if (!m_controls.m_shapeMarkerPopup) {
        return;
    }
    if (m_controls.m_shapeMarkerPopup->isVisible()) {
        hideShapeMarkerPopup();
        return;
    }
    hideTransientPanels();
    refreshShapeMarkerToolbarButton();
    m_controls.m_shapeMarkerPopup->show();
    updateShapeMarkerPopupGeometry();
}

namespace {

constexpr bool FEATURE_REMEMBER_SHAPE_MARKER_TOOLBAR_ICON_ENABLED = true;

}  // namespace

void ShotWindow::rememberShapeMarkerToolbarIconIfEnabled()
{
    if (!FEATURE_REMEMBER_SHAPE_MARKER_TOOLBAR_ICON_ENABLED) {
        return;
    }
    QPushButton *button = m_controls.m_shapeMarkerToolbarButton;
    if (!button) {
        return;
    }
    // 1. 【工具栏】【形状标记】把当前形状交给悬停换色，避免被固定星星图标覆盖
    button->setProperty("dynamicIcon", true);
    button->setProperty("markerShape", static_cast<int>(m_markerShape));
    button->setProperty("markerFilled", m_markerFilled);
}
