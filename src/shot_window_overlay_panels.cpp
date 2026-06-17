#include "shot_window_module.h"

using namespace markshot::shot;

/// @brief 切换工具颜色浮层的显示状态
/// @param position 浮层锚点所在窗口坐标
/// @return 无返回值
void ShotWindow::toggleColorPalette(QPoint position)
{
    commitTextEditor();
    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }
    if (!m_colorPalette) {
        return;
    }

    m_colorPaletteAnchor = position;
    if (m_colorPalette->isVisible()) {
        m_colorPalette->hide();
    } else {
        updateColorPaletteGeometry(position);
        m_colorPalette->show();
        m_colorPalette->raise();
    }
    update();
}

/// @brief 更新工具颜色浮层及其环形按钮位置
/// @param anchor 浮层锚点所在窗口坐标
/// @return 无返回值
void ShotWindow::updateColorPaletteGeometry(QPoint anchor)
{
    if (!m_colorPalette) {
        return;
    }

    const QSize paletteSize(178, 178);
    int x = anchor.x() - paletteSize.width() / 2;
    int y = anchor.y() - paletteSize.height() / 2;
    x = std::clamp(x, 8, std::max(8, width() - paletteSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - paletteSize.height() - 8));
    m_colorPalette->setGeometry(x, y, paletteSize.width(), paletteSize.height());

    const QPoint center(paletteSize.width() / 2, paletteSize.height() / 2);
    const qreal radius = 68.0;
    const auto buttons = m_colorPalette->findChildren<QPushButton *>(QString(), Qt::FindDirectChildrenOnly);
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
    if (!m_colorPalettePreview) {
        return;
    }

    const int size = std::clamp(qRound(currentToolPreviewSize()), 8, 34);
    const QPoint center(89, 89);
    m_colorPalettePreview->setGeometry(center.x() - size / 2, center.y() - size / 2, size, size);
    m_colorPalettePreview->setStyleSheet(QStringLiteral(
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
    if (!m_toolbar) {
        return;
    }

    const QString active = currentToolName();
    const QString scopeAction = markshot::ui::actionName(Action::ToggleCaptureScope);
    const QString layoutAction = markshot::ui::actionName(Action::ToggleToolbarLayout);
    const auto buttons = m_toolbar->findChildren<QPushButton *>();
    for (QPushButton *button : buttons) {
        const QString action = button->property("action").toString();
        const bool isActiveTool = action == active
            || (action == scopeAction && m_fullscreenAnnotation)
            || (action == layoutAction && m_toolbarVerticalLayout);
        button->setProperty("active", isActiveTool);
        button->style()->unpolish(button);
        button->style()->polish(button);
        button->update();
    }
}
