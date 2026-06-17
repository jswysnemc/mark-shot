#include "shot_window_module.h"

namespace cfg = markshot::config;
namespace shortcuts = markshot::shortcut;
using namespace markshot::shot;

void ShotWindow::setSelectedAnnotationOpacity(int opacity)
{
    opacity = std::clamp(opacity, 0, 100);
    const int alpha = qRound(opacity * 255.0 / 100.0);
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        bool changed = false;
        for (int id : selectedIds) {
            const Annotation *annotation = annotationById(id);
            if (annotation && annotation->color.alpha() != alpha) {
                changed = true;
                break;
            }
        }
        if (!changed) {
            return;
        }
        pushHistorySnapshot();
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id)) {
                annotation->color.setAlpha(alpha);
            }
        }
    } else {
        if (m_currentColor.alpha() == alpha) {
            return;
        }
        m_currentColor.setAlpha(alpha);
    }

    if (m_draft.has_value()) {
        m_draft->color.setAlpha(alpha);
    }
    if (m_laserDraft.has_value()) {
        m_laserDraft->color.setAlpha(alpha);
    }
    if (m_propertyColorPicker && m_propertyColorDialogPanel && m_propertyColorDialogPanel->isVisible()) {
        const QSignalBlocker blocker(m_propertyColorPicker);
        m_propertyColorPicker->setColor(selectedIds.isEmpty() ? m_currentColor : annotationById(selectedIds.first())->color);
    }
    updateColorPalettePreview();
    updateAnnotationPropertyPanel();
    update();
    persistAnnotationState();
}

void ShotWindow::setSelectedAnnotationFilled(bool filled)
{
    if (m_selectedAnnotationId.has_value()) {
        Annotation *annotation = annotationById(*m_selectedAnnotationId);
        if (!annotation || annotation->filled == filled) {
            return;
        }
        if (annotation->tool != Tool::Rectangle && annotation->tool != Tool::Ellipse) {
            return;
        }
        pushHistorySnapshot();
        annotation->filled = filled;
    } else {
        if (m_tool != Tool::Rectangle && m_tool != Tool::Ellipse) {
            return;
        }
        m_shapeFilled = filled;
    }
    updateAnnotationPropertyPanel();
    update();
    persistAnnotationState();
}

void ShotWindow::setSelectedAnnotationCornerRadius(int radius)
{
    if (m_selectedAnnotationId.has_value()) {
        Annotation *annotation = annotationById(*m_selectedAnnotationId);
        if (!annotation || annotation->tool != Tool::Rectangle || qRound(annotation->cornerRadius) == radius) {
            return;
        }
        pushHistorySnapshot();
        annotation->cornerRadius = radius;
    } else {
        if (m_tool != Tool::Rectangle || qRound(m_rectangleCornerRadius) == radius) {
            return;
        }
        m_rectangleCornerRadius = radius;
    }
    updateAnnotationPropertyPanel();
    update();
    persistAnnotationState();
}

void ShotWindow::setSelectedAnnotationArrowStyle(ArrowStyle style)
{
    if (m_selectedAnnotationId.has_value()) {
        Annotation *annotation = annotationById(*m_selectedAnnotationId);
        if (!annotation || annotation->tool != Tool::Arrow || annotation->arrowStyle == style) {
            return;
        }
        pushHistorySnapshot();
        annotation->arrowStyle = style;
    } else {
        if (m_tool != Tool::Arrow || m_arrowStyle == style) {
            return;
        }
        m_arrowStyle = style;
    }
    updateAnnotationPropertyPanel();
    update();
    persistAnnotationState();
}

void ShotWindow::setSelectedRectangleStyle(RectangleStyle style)
{
    // 切换矩形风格(描边/高亮/反色)。多选与单选共用同一逻辑,仅作用于
    // Tool::Rectangle 标注。无矩形选中时只更新工具默认值。
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        bool changed = false;
        // 1. 检测是否真的需要修改,避免无意义的历史快照
        for (int id : selectedIds) {
            const Annotation *annotation = annotationById(id);
            if (annotation && annotation->tool == Tool::Rectangle
                && annotation->rectangleStyle != style) {
                changed = true;
                break;
            }
        }
        if (!changed) {
            return;
        }
        // 2. 写入快照后批量改风格
        pushHistorySnapshot();
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id);
                annotation && annotation->tool == Tool::Rectangle) {
                annotation->rectangleStyle = style;
            }
        }
    } else {
        if (m_tool != Tool::Rectangle || m_rectangleStyle == style) {
            return;
        }
        m_rectangleStyle = style;
    }

    if (m_draft.has_value() && m_draft->tool == Tool::Rectangle) {
        m_draft->rectangleStyle = style;
    }
    updateAnnotationPropertyPanel();
    update();
    persistAnnotationState();
}

void ShotWindow::setSelectedHighlighterStyle(HighlighterStyle style)
{
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        bool changed = false;
        for (int id : selectedIds) {
            const Annotation *annotation = annotationById(id);
            if (annotation && annotation->tool == Tool::Highlighter
                && annotation->highlighterStyle != style) {
                changed = true;
                break;
            }
        }
        if (!changed) {
            return;
        }
        pushHistorySnapshot();
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id);
                annotation && annotation->tool == Tool::Highlighter) {
                annotation->highlighterStyle = style;
            }
        }
    } else {
        if (m_tool != Tool::Highlighter || m_highlighterStyle == style) {
            return;
        }
        m_highlighterStyle = style;
    }

    if (m_draft.has_value() && m_draft->tool == Tool::Highlighter) {
        m_draft->highlighterStyle = style;
    }
    updateAnnotationPropertyPanel();
    update();
    persistAnnotationState();
}

void ShotWindow::setSelectedNumberStyle(NumberStyle style)
{
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        bool changed = false;
        for (int id : selectedIds) {
            const Annotation *annotation = annotationById(id);
            if (annotation && annotation->tool == Tool::Number
                && annotation->numberStyle != style) {
                changed = true;
                break;
            }
        }
        if (!changed) {
            return;
        }
        pushHistorySnapshot();
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id);
                annotation && annotation->tool == Tool::Number) {
                annotation->numberStyle = style;
            }
        }
    } else {
        if (m_tool != Tool::Number || m_numberStyle == style) {
            return;
        }
        m_numberStyle = style;
    }

    if (m_draft.has_value() && m_draft->tool == Tool::Number) {
        m_draft->numberStyle = style;
    }
    updateAnnotationPropertyPanel();
    update();
    persistAnnotationState();
}

void ShotWindow::resetNumberSequence()
{
    if (m_nextNumber == 1) {
        return;
    }

    pushHistorySnapshot();
    m_nextNumber = 1;
    if (m_draft.has_value() && m_draft->tool == Tool::Number) {
        m_draft->number = m_nextNumber;
    }
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::setSelectedMagnifierScale(int scaleValue)
{
    const qreal scale = magnifierScaleFromSliderValue(scaleValue);
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        bool changed = false;
        for (int id : selectedIds) {
            const Annotation *annotation = annotationById(id);
            if (annotation && annotation->tool == Tool::Magnifier
                && !qFuzzyCompare(clampedMagnifierScale(annotation->magnifierScale), scale)) {
                changed = true;
                break;
            }
        }
        if (!changed) {
            return;
        }
        pushHistorySnapshot();
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id);
                annotation && annotation->tool == Tool::Magnifier) {
                annotation->magnifierScale = scale;
            }
        }
    } else {
        if (m_tool != Tool::Magnifier || qFuzzyCompare(m_magnifierScale, scale)) {
            return;
        }
        m_magnifierScale = scale;
    }

    if (m_draft.has_value() && m_draft->tool == Tool::Magnifier) {
        m_draft->magnifierScale = scale;
    }
    updateAnnotationPropertyPanel();
    update();
    persistAnnotationState();
}

void ShotWindow::setSelectedMagnifierShape(MagnifierShape shape)
{
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        bool changed = false;
        for (int id : selectedIds) {
            const Annotation *annotation = annotationById(id);
            if (annotation && annotation->tool == Tool::Magnifier
                && annotation->magnifierShape != shape) {
                changed = true;
                break;
            }
        }
        if (!changed) {
            return;
        }
        pushHistorySnapshot();
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id);
                annotation && annotation->tool == Tool::Magnifier) {
                annotation->magnifierShape = shape;
            }
        }
    } else {
        if (m_tool != Tool::Magnifier || m_magnifierShape == shape) {
            return;
        }
        m_magnifierShape = shape;
    }

    if (m_draft.has_value() && m_draft->tool == Tool::Magnifier) {
        m_draft->magnifierShape = shape;
    }
    updateAnnotationPropertyPanel();
    update();
    persistAnnotationState();
}

void ShotWindow::toggleMagnifierShape()
{
    setSelectedMagnifierShape(m_magnifierShape == MagnifierShape::Circle
                                  ? MagnifierShape::Rectangle
                                  : MagnifierShape::Circle);
}

void ShotWindow::deleteSelectedAnnotation()
{
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (selectedIds.isEmpty()) {
        return;
    }
    pushHistorySnapshot();
    for (int i = m_annotations.size() - 1; i >= 0; --i) {
        if (selectedIds.contains(m_annotations.at(i).id)) {
            m_annotations.removeAt(i);
        }
    }
    setSelectedAnnotations({});
    m_annotationDrag = SelectionDrag::None;
    updateAnnotationPropertyPanel();
    updateCursor();
    update();
}

void ShotWindow::openSelectedAnnotationColorPalette()
{
    if (!m_propertyColorDialogPanel || !m_propertyColorPicker || !m_annotationPropertyPanel) {
        return;
    }
    const bool wasEditingTextBackground = m_propertyColorEditingTextBackground;
    m_propertyColorEditingTextBackground = false;

    if (m_propertyColorDialogPanel->isVisible() && !wasEditingTextBackground) {
        m_propertyColorDialogPanel->hide();
        return;
    }

    if (m_colorPalette) {
        m_colorPalette->hide();
    }
    QColor color = m_currentColor;
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        if (const Annotation *annotation = annotationById(selectedIds.first())) {
            color = annotation->color;
        }
    }
    m_propertyColorEditHistoryCaptured = false;
    {
        const QSignalBlocker blocker(m_propertyColorPicker);
        m_propertyColorPicker->setColor(color);
    }
    updateAnnotationPropertyPanel();
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->show();
        m_annotationPropertyPanel->raise();
        if (QLayout *panelLayout = m_annotationPropertyPanel->layout()) {
            panelLayout->activate();
        }
        updateAnnotationPropertyPanelGeometry();
    }
    if (QLayout *colorLayout = m_propertyColorDialogPanel->layout()) {
        colorLayout->activate();
    }
    updatePropertyColorDialogGeometry();
    m_propertyColorDialogPanel->show();
    updatePropertyColorDialogGeometry();
    m_propertyColorDialogPanel->raise();
    QTimer::singleShot(0, this, [this] {
        if (m_propertyColorDialogPanel && m_propertyColorDialogPanel->isVisible()) {
            updatePropertyColorDialogGeometry();
            m_propertyColorDialogPanel->raise();
        }
    });
}

void ShotWindow::openSelectedTextBackgroundColorPalette()
{
    if (!m_propertyColorDialogPanel || !m_propertyColorPicker || !m_annotationPropertyPanel) {
        return;
    }
    const bool wasEditingTextBackground = m_propertyColorEditingTextBackground;
    m_propertyColorEditingTextBackground = true;

    if (m_propertyColorDialogPanel->isVisible() && wasEditingTextBackground) {
        m_propertyColorDialogPanel->hide();
        return;
    }

    if (m_colorPalette) {
        m_colorPalette->hide();
    }
    QColor color = m_textBackgroundColor;
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (selectedIds.size() == 1) {
        if (const Annotation *annotation = annotationById(selectedIds.first());
            annotation && annotation->tool == Tool::Text) {
            color = annotation->backgroundColor;
        }
    }
    m_propertyColorEditHistoryCaptured = false;
    {
        const QSignalBlocker blocker(m_propertyColorPicker);
        m_propertyColorPicker->setColor(color);
    }
    updateAnnotationPropertyPanel();
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->show();
        m_annotationPropertyPanel->raise();
        if (QLayout *panelLayout = m_annotationPropertyPanel->layout()) {
            panelLayout->activate();
        }
        updateAnnotationPropertyPanelGeometry();
    }
    if (QLayout *colorLayout = m_propertyColorDialogPanel->layout()) {
        colorLayout->activate();
    }
    updatePropertyColorDialogGeometry();
    m_propertyColorDialogPanel->show();
    updatePropertyColorDialogGeometry();
    m_propertyColorDialogPanel->raise();
    QTimer::singleShot(0, this, [this] {
        if (m_propertyColorDialogPanel && m_propertyColorDialogPanel->isVisible()) {
            updatePropertyColorDialogGeometry();
            m_propertyColorDialogPanel->raise();
        }
    });
}

void ShotWindow::toggleSelectedTextFontPanel()
{
    if (!m_propertyFontPanel || !m_propertyFontList || !m_propertyFontButton) {
        return;
    }

    if (m_propertyFontPanel->isVisible()) {
        m_propertyFontPanel->hide();
        return;
    }

    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    updateAnnotationPropertyPanel();
    if (QLayout *fontLayout = m_propertyFontPanel->layout()) {
        fontLayout->activate();
    }
    updatePropertyFontPanelGeometry();
    m_propertyFontPanel->show();
    updatePropertyFontPanelGeometry();
    m_propertyFontPanel->raise();
}

void ShotWindow::applyPropertyColor(QColor color)
{
    if (!color.isValid()) {
        return;
    }
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (m_propertyColorEditingTextBackground) {
        if (!selectedIds.isEmpty()) {
            if (!m_propertyColorEditHistoryCaptured) {
                pushHistorySnapshot();
                m_propertyColorEditHistoryCaptured = true;
            }
            for (int id : selectedIds) {
                if (Annotation *annotation = annotationById(id);
                    annotation && annotation->tool == Tool::Text) {
                    annotation->backgroundColor = color;
                }
            }
        } else if (m_tool == Tool::Text) {
            m_textBackgroundColor = color;
        }
    } else if (!selectedIds.isEmpty()) {
        if (!m_propertyColorEditHistoryCaptured) {
            pushHistorySnapshot();
            m_propertyColorEditHistoryCaptured = true;
        }
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id)) {
                annotation->color = color;
            }
        }
    } else {
        m_currentColor = color;
    }
    if (m_draft.has_value()) {
        m_draft->color = color;
    }
    if (m_textEditor && m_textEditor->isVisible()) {
        QColor editorColor = m_currentColor;
        QColor editorBackgroundColor = m_textBackgroundColor;
        qreal editorWidth = m_textSize;
        if (m_editingTextAnnotationId.has_value()) {
            if (const Annotation *annotation = annotationById(*m_editingTextAnnotationId)) {
                editorColor = annotation->color;
                editorBackgroundColor = annotation->backgroundColor;
                editorWidth = annotation->width;
            }
        }
        m_textEditor->setStyleSheet(markshot::theme::textEditorStyleSheet(editorColor, editorBackgroundColor, qRound(20.0 + editorWidth)));
    }
    updateColorPalettePreview();
    updateAnnotationPropertyPanel();
    update();
    persistAnnotationState();
}

void ShotWindow::clearAnnotations()
{
    commitTextEditor();
    if (m_annotations.isEmpty() && !m_draft.has_value() && m_laserStrokes.isEmpty() && !m_laserDraft.has_value()) {
        return;
    }

    pushHistorySnapshot();
    m_annotations.clear();
    m_draft.reset();
    m_laserStrokes.clear();
    m_laserDraft.reset();
    setSelectedAnnotations({});
    m_annotationDrag = SelectionDrag::None;
    m_annotationSelectionBoxActive = false;
    m_annotationHistoryCaptured = false;
    m_nextNumber = 1;
    m_nextAnnotationId = 1;
    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    if (m_propertyFontPanel) {
        m_propertyFontPanel->hide();
    }
    updateAnnotationPropertyPanel();
    updateCursor();
    update();
}

void ShotWindow::setSelectedTextFontFamily(const QString &fontFamily)
{
    if (fontFamily.isEmpty()) {
        return;
    }

    if (m_selectedAnnotationId.has_value()) {
        Annotation *annotation = annotationById(*m_selectedAnnotationId);
        if (!annotation || annotation->tool != Tool::Text || annotation->fontFamily == fontFamily) {
            return;
        }
        pushHistorySnapshot();
        annotation->fontFamily = fontFamily;
    } else {
        if (m_tool != Tool::Text || m_textFontFamily == fontFamily) {
            return;
        }
        m_textFontFamily = fontFamily;
        if (m_textEditor && m_textEditor->isVisible() && !m_editingTextAnnotationId.has_value()) {
            m_textEditor->setFont(markshot::theme::textFont(qRound(20.0 + m_textSize),
                                                            QFont::DemiBold,
                                                            m_textFontFamily));
        }
    }
    updateAnnotationPropertyPanel();
    update();
    persistAnnotationState();
}
