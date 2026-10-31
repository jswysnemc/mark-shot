#include "shot_window_module.h"
#include "shot_window_ruler_tool.h"

#include <QCursor>

namespace cfg = markshot::config;
namespace shortcuts = markshot::shortcut;
using namespace markshot::shot;

QRectF ShotWindow::constrainedRect(QPointF start, QPointF end) const
{
    const qreal dx = end.x() - start.x();
    const qreal dy = end.y() - start.y();
    const qreal side = std::max(std::abs(dx), std::abs(dy));
    const QPointF constrainedEnd(start.x() + std::copysign(side, dx == 0.0 ? 1.0 : dx),
                                 start.y() + std::copysign(side, dy == 0.0 ? 1.0 : dy));
    return normalizedRect(start, clampImagePoint(constrainedEnd));
}

void ShotWindow::updateFrozenImageRect()
{
    if (m_frozenFrame.isNull()) {
        m_frozenImageRect = {};
        m_imageCenterInitialized = false;
        return;
    }

    QSizeF frameSize = m_frozenFrame.size();
    frameSize.scale(size(), Qt::KeepAspectRatio);
    if (!imageNavigationAvailable()) {
        const QPointF topLeft((width() - frameSize.width()) / 2.0, (height() - frameSize.height()) / 2.0);
        m_frozenImageRect = QRectF(topLeft, frameSize);
        return;
    }

    const qreal fitScale = frameSize.width() / std::max<qreal>(1.0, m_frozenFrame.width());
    const qreal scale = fitScale * m_imageZoom;
    frameSize = QSizeF(m_frozenFrame.width() * scale, m_frozenFrame.height() * scale);
    if (!m_imageCenterInitialized) {
        m_imageCenter = QPointF(m_frozenFrame.width() / 2.0, m_frozenFrame.height() / 2.0);
        m_imageCenterInitialized = true;
    }

    if (frameSize.width() <= width()) {
        m_imageCenter.setX(m_frozenFrame.width() / 2.0);
    } else {
        const qreal halfVisibleWidth = width() / (2.0 * scale);
        m_imageCenter.setX(std::clamp(m_imageCenter.x(), halfVisibleWidth, m_frozenFrame.width() - halfVisibleWidth));
    }
    if (frameSize.height() <= height()) {
        m_imageCenter.setY(m_frozenFrame.height() / 2.0);
    } else {
        const qreal halfVisibleHeight = height() / (2.0 * scale);
        m_imageCenter.setY(std::clamp(m_imageCenter.y(), halfVisibleHeight, m_frozenFrame.height() - halfVisibleHeight));
    }

    const QPointF widgetCenter(width() / 2.0, height() / 2.0);
    const QPointF topLeft(widgetCenter.x() - m_imageCenter.x() * scale,
                          widgetCenter.y() - m_imageCenter.y() * scale);
    m_frozenImageRect = QRectF(topLeft, frameSize);
}

void ShotWindow::zoomImageAt(qreal factor, QPointF widgetAnchor)
{
    if (!imageNavigationAvailable() || m_frozenFrame.isNull() || m_frozenImageRect.isEmpty() || factor <= 0.0) {
        return;
    }

    const QPointF anchorImage = m_frozenImageRect.contains(widgetAnchor)
        ? widgetToImage(widgetAnchor)
        : (m_imageCenterInitialized ? m_imageCenter : QPointF(m_frozenFrame.width() / 2.0, m_frozenFrame.height() / 2.0));
    m_imageZoom = std::clamp(m_imageZoom * factor, kMinImageZoom, kMaxImageZoom);

    QSizeF fitSize = m_frozenFrame.size();
    fitSize.scale(size(), Qt::KeepAspectRatio);
    const qreal scale = fitSize.width() / std::max<qreal>(1.0, m_frozenFrame.width()) * m_imageZoom;
    const QPointF widgetCenter(width() / 2.0, height() / 2.0);
    m_imageCenter = QPointF(anchorImage.x() - (widgetAnchor.x() - widgetCenter.x()) / scale,
                            anchorImage.y() - (widgetAnchor.y() - widgetCenter.y()) / scale);
    m_imageCenterInitialized = true;
    updateFrozenImageRect();
    refreshViewGeometry();
    update();
}

void ShotWindow::resetImageZoom()
{
    if (m_frozenFrame.isNull()) {
        return;
    }

    m_imageZoom = 1.0;
    m_imageCenter = QPointF(m_frozenFrame.width() / 2.0, m_frozenFrame.height() / 2.0);
    m_imageCenterInitialized = true;
    updateFrozenImageRect();
    refreshViewGeometry();
    update();
}

void ShotWindow::panImageTo(QPointF widgetPosition)
{
    if (!imageNavigationAvailable() || m_frozenFrame.isNull() || m_frozenImageRect.isEmpty()) {
        return;
    }

    const qreal scale = m_frozenImageRect.width() / std::max<qreal>(1.0, m_frozenFrame.width());
    if (scale <= 0.0) {
        return;
    }

    const QPointF delta = widgetPosition - m_imagePanStartWidget;
    m_imageCenter = m_imagePanStartCenter - delta / scale;
    m_imageCenterInitialized = true;
    updateFrozenImageRect();
    refreshViewGeometry();
    update();
}

void ShotWindow::updateImageScrollBars()
{
    if (!m_controls.m_horizontalImageScrollBar || !m_controls.m_verticalImageScrollBar) {
        return;
    }
    if (!imageNavigationAvailable() || m_frozenFrame.isNull() || m_frozenImageRect.isEmpty()) {
        m_controls.m_horizontalImageScrollBar->hide();
        m_controls.m_verticalImageScrollBar->hide();
        return;
    }

    const qreal scale = m_frozenImageRect.width() / std::max<qreal>(1.0, m_frozenFrame.width());
    if (scale <= 0.0) {
        m_controls.m_horizontalImageScrollBar->hide();
        m_controls.m_verticalImageScrollBar->hide();
        return;
    }

    const int scaledWidth = qRound(m_frozenFrame.width() * scale);
    const int scaledHeight = qRound(m_frozenFrame.height() * scale);
    const int maxX = std::max(0, scaledWidth - width());
    const int maxY = std::max(0, scaledHeight - height());
    const bool showHorizontal = maxX > 0;
    const bool showVertical = maxY > 0;

    const int horizontalWidth = std::max(0, width() - (showVertical ? kImageScrollBarExtent : 0));
    const int verticalHeight = std::max(0, height() - (showHorizontal ? kImageScrollBarExtent : 0));
    m_controls.m_horizontalImageScrollBar->setGeometry(0,
                                            height() - kImageScrollBarExtent,
                                            horizontalWidth,
                                            kImageScrollBarExtent);
    m_controls.m_verticalImageScrollBar->setGeometry(width() - kImageScrollBarExtent,
                                          0,
                                          kImageScrollBarExtent,
                                          verticalHeight);

    const QSignalBlocker blockHorizontal(m_controls.m_horizontalImageScrollBar);
    const QSignalBlocker blockVertical(m_controls.m_verticalImageScrollBar);
    m_syncingImageScrollBars = true;
    m_controls.m_horizontalImageScrollBar->setRange(0, maxX);
    m_controls.m_horizontalImageScrollBar->setPageStep(std::max(1, width()));
    m_controls.m_horizontalImageScrollBar->setSingleStep(std::max(1, width() / 12));
    m_controls.m_horizontalImageScrollBar->setValue(std::clamp(qRound(-m_frozenImageRect.left()), 0, maxX));
    m_controls.m_verticalImageScrollBar->setRange(0, maxY);
    m_controls.m_verticalImageScrollBar->setPageStep(std::max(1, height()));
    m_controls.m_verticalImageScrollBar->setSingleStep(std::max(1, height() / 12));
    m_controls.m_verticalImageScrollBar->setValue(std::clamp(qRound(-m_frozenImageRect.top()), 0, maxY));
    m_syncingImageScrollBars = false;

    // 1. 【画布滚动条】【按需显示】内容超出视区时才存在，并且只在指针靠近边缘或正在拖动时出现
    const QPoint pointer = mapFromGlobal(QCursor::pos());
    const bool pointerInside = rect().contains(pointer);
    const int revealDistance = kImageScrollBarExtent + 18;
    const bool nearHorizontal = pointerInside && pointer.y() >= height() - revealDistance;
    const bool nearVertical = pointerInside && pointer.x() >= width() - revealDistance;
    m_controls.m_horizontalImageScrollBar->setVisible(
        showHorizontal && (nearHorizontal || m_controls.m_horizontalImageScrollBar->isSliderDown()));
    m_controls.m_verticalImageScrollBar->setVisible(
        showVertical && (nearVertical || m_controls.m_verticalImageScrollBar->isSliderDown()));
    if (showHorizontal) {
        m_controls.m_horizontalImageScrollBar->raise();
    }
    if (showVertical) {
        m_controls.m_verticalImageScrollBar->raise();
    }
}

void ShotWindow::setImageCenterFromScrollBars()
{
    if (m_syncingImageScrollBars || !imageNavigationAvailable()
        || m_frozenFrame.isNull() || m_frozenImageRect.isEmpty()) {
        return;
    }

    const qreal scale = m_frozenImageRect.width() / std::max<qreal>(1.0, m_frozenFrame.width());
    if (scale <= 0.0) {
        return;
    }

    const bool hasHorizontal = m_controls.m_horizontalImageScrollBar && m_controls.m_horizontalImageScrollBar->maximum() > 0;
    const bool hasVertical = m_controls.m_verticalImageScrollBar && m_controls.m_verticalImageScrollBar->maximum() > 0;
    const qreal centerX = hasHorizontal
        ? (m_controls.m_horizontalImageScrollBar->value() + width() / 2.0) / scale
        : m_frozenFrame.width() / 2.0;
    const qreal centerY = hasVertical
        ? (m_controls.m_verticalImageScrollBar->value() + height() / 2.0) / scale
        : m_frozenFrame.height() / 2.0;

    m_imageCenter = QPointF(centerX, centerY);
    m_imageCenterInitialized = true;
    updateFrozenImageRect();
    refreshViewGeometry();
    update();
}

void ShotWindow::updateMinimumImageWindowSize()
{
    if (!m_imageNavigationEnabled || !m_controls.m_toolbar) {
        setMinimumSize(QSize(0, 0));
        return;
    }

    m_controls.m_toolbar->adjustSize();
    const int minWidth = m_controls.m_toolbar->sizeHint().width() + kImageWindowMinimumToolbarPadding;
    setMinimumWidth(minWidth);
    if (width() < minWidth) {
        resize(minWidth, height());
    }
}

void ShotWindow::refreshViewGeometry()
{
    updateTextEditorGeometry();
    updateImageScrollBars();
    updateToolbarGeometry();
    updateActionToolbarGeometry();
    updateAnnotationPropertyPanelGeometry();
    updateOpenWithPanelGeometry();
    updateExtensionPanelGeometry();
    // 【截图】【选区尺寸】窗口缩放或视图平移后 HUD 重新跟随选区左上角
    updateSelectionSizePanelGeometry();
}

void ShotWindow::updateAnnotationPropertyPanel()
{
    if (!m_controls.m_annotationPropertyPanel) {
        return;
    }

    const QVector<int> selectedIds = selectedAnnotationIds();
    const Annotation *annotation = selectedIds.size() == 1
        ? annotationById(selectedIds.first())
        : nullptr;
    const Annotation *firstSelectedAnnotation = !selectedIds.isEmpty()
        ? annotationById(selectedIds.first())
        : nullptr;
    const bool groupSelection = m_tool == Tool::Select && selectedIds.size() > 1;
    const bool editingAnnotation = m_tool == Tool::Select && !selectedIds.isEmpty();
    const bool editingTool = m_mode == Mode::Editing
        && m_tool != Tool::Move
        && m_tool != Tool::Select;
    if (!editingAnnotation && !editingTool) {
        m_controls.m_annotationPropertyPanel->hide();
        if (m_controls.m_propertyColorDialogPanel) {
            m_controls.m_propertyColorDialogPanel->hide();
        }
        if (m_controls.m_propertyFontPanel) {
            m_controls.m_propertyFontPanel->hide();
        }
        return;
    }

    const Tool panelTool = groupSelection ? Tool::Select : (annotation ? annotation->tool : m_tool);
    const QColor panelColor = firstSelectedAnnotation ? firstSelectedAnnotation->color : m_currentColor;
    const QColor panelTextBackgroundColor = annotation && annotation->tool == Tool::Text
        ? annotation->backgroundColor
        : m_textBackgroundColor;
    const qreal panelWidth = firstSelectedAnnotation ? firstSelectedAnnotation->width : currentToolWidth();
    const int panelOpacity = qRound(panelColor.alphaF() * 100.0);
    const bool panelFilled = annotation ? annotation->filled : m_shapeFilled;
    const qreal panelRadius = annotation ? annotation->cornerRadius : m_rectangleCornerRadius;
    const RectangleStyle panelRectangleStyle =
        annotation && annotation->tool == Tool::Rectangle
            ? annotation->rectangleStyle
            : m_rectangleStyle;
    const qreal panelMagnifierScale =
        annotation && annotation->tool == Tool::Magnifier
            ? annotation->magnifierScale
            : m_magnifierScale;
    const HighlighterStyle panelHighlighterStyle =
        annotation && annotation->tool == Tool::Highlighter
            ? annotation->highlighterStyle
            : m_highlighterStyle;
    const NumberStyle panelNumberStyle =
        annotation && annotation->tool == Tool::Number
            ? annotation->numberStyle
            : m_numberStyle;
    const QString panelFontFamily = annotation ? annotation->fontFamily : m_textFontFamily;
    const QFont::Weight panelFontWeight =
        annotation && annotation->tool == Tool::Text ? annotation->fontWeight : m_textWeight;
    const bool panelFontItalic =
        annotation && annotation->tool == Tool::Text ? annotation->textItalic : m_textItalic;

    // 【标注属性栏】【标题胶囊】工具名称已由工具栏选中态表达，只在多选时显示分组数量
    if (m_controls.m_annotationPropertyTitle) {
        m_controls.m_annotationPropertyTitle->setVisible(groupSelection);
        if (groupSelection) {
            m_controls.m_annotationPropertyTitle->setText(MS_TR("Group %1").arg(selectedIds.size()));
        }
    }
    if (m_controls.m_propertyEditTextButton) {
        m_controls.m_propertyEditTextButton->setVisible(!groupSelection && editingAnnotation && panelTool == Tool::Text);
    }
    if (m_controls.m_propertyFontButton) {
        m_controls.m_propertyFontButton->setVisible(!groupSelection && panelTool == Tool::Text);
        if (!groupSelection && panelTool == Tool::Text) {
            const QString family = panelFontFamily.isEmpty() ? markshot::theme::textFontFamily() : panelFontFamily;
            m_controls.m_propertyFontButton->setToolTip(family);
            if (m_controls.m_propertyFontList) {
                const auto matches = m_controls.m_propertyFontList->findItems(family, Qt::MatchExactly);
                if (!matches.isEmpty()) {
                    m_controls.m_propertyFontList->setCurrentItem(matches.first());
                    m_controls.m_propertyFontList->scrollToItem(matches.first(), QAbstractItemView::PositionAtCenter);
                }
            }
        } else if (m_controls.m_propertyFontPanel) {
            m_controls.m_propertyFontPanel->hide();
            m_controls.m_propertyFontButton->setToolTip(MS_TR("Text font"));
        }
    }
    if (m_controls.m_propertyFillButton) {
        // Highlight/Invert 风格不使用 filled 字段,隐藏填充开关避免歧义
        const bool supportsFill = !groupSelection
            && (panelTool == Tool::Ellipse
                || (panelTool == Tool::Rectangle && panelRectangleStyle == RectangleStyle::Stroke));
        m_controls.m_propertyFillButton->setVisible(supportsFill);
        const QSignalBlocker blocker(m_controls.m_propertyFillButton);
        m_controls.m_propertyFillButton->setChecked(panelFilled);
        m_controls.m_propertyFillButton->setIcon(markshot::ui::makeFillIcon(panelFilled, m_toolbarAppearance.palette.text));
    }
    // 圆角对描边与聚光灯两种矩形风格生效
    const bool supportsRadius = !groupSelection
        && panelTool == Tool::Rectangle
        && (panelRectangleStyle == RectangleStyle::Stroke || panelRectangleStyle == RectangleStyle::Spotlight);
    if (m_controls.m_propertyRadiusGlyphLabel) {
        m_controls.m_propertyRadiusGlyphLabel->setVisible(supportsRadius);
    }
    if (m_controls.m_propertyRadiusLabel) {
        m_controls.m_propertyRadiusLabel->setVisible(supportsRadius);
        m_controls.m_propertyRadiusLabel->setText(QString::number(qRound(panelRadius)));
    }
    if (m_controls.m_propertyRadiusSlider) {
        m_controls.m_propertyRadiusSlider->setVisible(supportsRadius);
        const QSignalBlocker blocker(m_controls.m_propertyRadiusSlider);
        m_controls.m_propertyRadiusSlider->setValue(qRound(panelRadius));
    }
    if (m_controls.m_propertyRectangleStyleCombo) {
        const bool supportsRectangleStyle = !groupSelection && panelTool == Tool::Rectangle;
        m_controls.m_propertyRectangleStyleCombo->setVisible(supportsRectangleStyle);
        if (supportsRectangleStyle) {
            const QSignalBlocker blocker(m_controls.m_propertyRectangleStyleCombo);
            const int styleValue = static_cast<int>(panelRectangleStyle);
            for (int i = 0; i < m_controls.m_propertyRectangleStyleCombo->count(); ++i) {
                if (m_controls.m_propertyRectangleStyleCombo->itemData(i).toInt() == styleValue) {
                    m_controls.m_propertyRectangleStyleCombo->setCurrentIndex(i);
                    break;
                }
            }
        }
    }
    if (m_controls.m_propertyMosaicStyleCombo) {
        const bool supportsMosaicStyle = !groupSelection && panelTool == Tool::Mosaic;
        m_controls.m_propertyMosaicStyleCombo->setVisible(supportsMosaicStyle);
        if (supportsMosaicStyle) {
            const MosaicStyle panelMosaicStyle =
                annotation && annotation->tool == Tool::Mosaic ? annotation->mosaicStyle : m_mosaicStyle;
            const QSignalBlocker blocker(m_controls.m_propertyMosaicStyleCombo);
            m_controls.m_propertyMosaicStyleCombo->setCurrentIndex(
                std::max(0, m_controls.m_propertyMosaicStyleCombo->findData(static_cast<int>(panelMosaicStyle))));
        }
    }
    updateArrowPropertyControls(annotation, !groupSelection && panelTool == Tool::Arrow);
    // 测量方式只决定新画的尺子，只在尺子工具下显示
    ShotWindowRulerTool(*this).syncPanel(!groupSelection && m_tool == Tool::Ruler);
    if (m_controls.m_propertyHighlighterStyleCombo) {
        const bool supportsHighlighterStyle = !groupSelection && panelTool == Tool::Highlighter;
        m_controls.m_propertyHighlighterStyleCombo->setVisible(supportsHighlighterStyle);
        if (supportsHighlighterStyle) {
            const QSignalBlocker blocker(m_controls.m_propertyHighlighterStyleCombo);
            m_controls.m_propertyHighlighterStyleCombo->setCurrentIndex(
                panelHighlighterStyle == HighlighterStyle::StraightLine ? 1 : 0);
        }
    }
    if (m_controls.m_propertyNumberStyleCombo) {
        const bool supportsNumberStyle = !groupSelection && panelTool == Tool::Number;
        m_controls.m_propertyNumberStyleCombo->setVisible(supportsNumberStyle);
        if (supportsNumberStyle) {
            const QSignalBlocker blocker(m_controls.m_propertyNumberStyleCombo);
            const int numberStyleValue = static_cast<int>(panelNumberStyle);
            for (int i = 0; i < m_controls.m_propertyNumberStyleCombo->count(); ++i) {
                if (m_controls.m_propertyNumberStyleCombo->itemData(i).toInt() == numberStyleValue) {
                    m_controls.m_propertyNumberStyleCombo->setCurrentIndex(i);
                    break;
                }
            }
        }
    }
    if (m_controls.m_propertyResetNumberButton) {
        m_controls.m_propertyResetNumberButton->setVisible(!groupSelection && panelTool == Tool::Number);
    }
    const bool supportsMagnifierScale = !groupSelection && panelTool == Tool::Magnifier;
    if (m_controls.m_propertyMagnifierScaleGlyphLabel) {
        m_controls.m_propertyMagnifierScaleGlyphLabel->setVisible(supportsMagnifierScale);
    }
    if (m_controls.m_propertyMagnifierScaleLabel) {
        m_controls.m_propertyMagnifierScaleLabel->setVisible(supportsMagnifierScale);
        m_controls.m_propertyMagnifierScaleLabel->setText(magnifierScaleText(panelMagnifierScale));
    }
    if (m_controls.m_propertyMagnifierScaleSlider) {
        m_controls.m_propertyMagnifierScaleSlider->setVisible(supportsMagnifierScale);
        const QSignalBlocker blocker(m_controls.m_propertyMagnifierScaleSlider);
        m_controls.m_propertyMagnifierScaleSlider->setValue(magnifierScaleSliderValue(panelMagnifierScale));
    }
    if (m_controls.m_propertyMagnifierShapeButton) {
        m_controls.m_propertyMagnifierShapeButton->setVisible(supportsMagnifierScale);
        const MagnifierShape panelShape = annotation && annotation->tool == Tool::Magnifier
            ? annotation->magnifierShape
            : m_magnifierShape;
        const QSignalBlocker blocker(m_controls.m_propertyMagnifierShapeButton);
        m_controls.m_propertyMagnifierShapeButton->setChecked(panelShape == MagnifierShape::Rectangle);
    }
    if (m_controls.m_propertyWidthLabel) {
        m_controls.m_propertyWidthLabel->setText(QString::number(qRound(panelWidth)));
    }
    if (m_controls.m_propertyWidthSlider) {
        const QSignalBlocker blocker(m_controls.m_propertyWidthSlider);
        if (panelTool == Tool::Mosaic) {
            m_controls.m_propertyWidthSlider->setRange(qRound(kMinMosaicBlockSize), qRound(kMaxMosaicBlockSize));
        } else if (panelTool == Tool::Number) {
            m_controls.m_propertyWidthSlider->setRange(qRound(kMinNumberWidth), qRound(kMaxNumberWidth));
        } else if (panelTool == Tool::Text) {
            m_controls.m_propertyWidthSlider->setRange(-11, 281);
        } else if (panelTool == Tool::Highlighter) {
            m_controls.m_propertyWidthSlider->setRange(qRound(kMinStrokeWidth), qRound(kMaxHighlighterWidth));
        } else {
            m_controls.m_propertyWidthSlider->setRange(qRound(kMinStrokeWidth), qRound(kMaxStrokeWidth));
        }
        m_controls.m_propertyWidthSlider->setValue(qRound(panelWidth));
    }
    const bool isTextPanel = !groupSelection && panelTool == Tool::Text;
    if (m_controls.m_propertyFontSizeSpin) {
        const QSignalBlocker blocker(m_controls.m_propertyFontSizeSpin);
        m_controls.m_propertyFontSizeSpin->setVisible(isTextPanel);
        m_controls.m_propertyFontSizeSpin->setValue(qRound(19.0 + panelWidth));
    }
    if (m_controls.m_propertyFontBoldButton) {
        const QSignalBlocker blocker(m_controls.m_propertyFontBoldButton);
        m_controls.m_propertyFontBoldButton->setVisible(isTextPanel);
        m_controls.m_propertyFontBoldButton->setChecked(panelFontWeight >= QFont::DemiBold);
    }
    if (m_controls.m_propertyFontItalicButton) {
        const QSignalBlocker blocker(m_controls.m_propertyFontItalicButton);
        m_controls.m_propertyFontItalicButton->setVisible(isTextPanel);
        m_controls.m_propertyFontItalicButton->setChecked(panelFontItalic);
    }
    if (m_controls.m_propertyOpacityLabel) {
        m_controls.m_propertyOpacityLabel->setText(QStringLiteral("%1%").arg(panelOpacity));
    }
    if (m_controls.m_propertyOpacitySlider) {
        const QSignalBlocker blocker(m_controls.m_propertyOpacitySlider);
        m_controls.m_propertyOpacitySlider->setValue(panelOpacity);
    }
    if (m_controls.m_propertyColorButton) {
        m_controls.m_propertyColorButton->setStyleSheet(markshot::theme::propertyColorButtonStyleSheet(panelColor));
        m_controls.m_propertyColorButton->setIcon(markshot::ui::makePropertyIcon(
            markshot::ui::PropertyIcon::Color, propertyIconInkForFill(panelColor, m_toolbarAppearance.palette.text)));
        m_controls.m_propertyColorButton->setVisible(panelTool != Tool::Mosaic);
        if (panelTool == Tool::Mosaic && m_controls.m_propertyColorDialogPanel) {
            m_controls.m_propertyColorDialogPanel->hide();
        }
    }
    if (m_controls.m_propertyTextBackgroundButton) {
        const bool supportsTextBackground = !groupSelection && panelTool == Tool::Text;
        m_controls.m_propertyTextBackgroundButton->setVisible(supportsTextBackground);
        m_controls.m_propertyTextBackgroundButton->setStyleSheet(markshot::theme::propertyColorButtonStyleSheet(panelTextBackgroundColor));
        m_controls.m_propertyTextBackgroundButton->setIcon(markshot::ui::makePropertyIcon(
            markshot::ui::PropertyIcon::TextBackground,
            propertyIconInkForFill(panelTextBackgroundColor, m_toolbarAppearance.palette.text)));
        if (!supportsTextBackground && m_controls.m_propertyColorDialogPanel && m_controls.m_propertyColorEditingTextBackground) {
            m_controls.m_propertyColorDialogPanel->hide();
        }
    }
    if (m_controls.m_propertyColorPicker && m_controls.m_propertyColorDialogPanel && m_controls.m_propertyColorDialogPanel->isVisible()) {
        const QSignalBlocker blocker(m_controls.m_propertyColorPicker);
        m_controls.m_propertyColorPicker->setColor(m_controls.m_propertyColorEditingTextBackground ? panelTextBackgroundColor : panelColor);
    }

    m_controls.m_annotationPropertyPanel->show();
    if (QLayout *panelLayout = m_controls.m_annotationPropertyPanel->layout()) {
        panelLayout->activate();
    }
    updateAnnotationPropertyPanelGeometry();
    m_controls.m_annotationPropertyPanel->raise();
    if (m_controls.m_propertyColorDialogPanel && m_controls.m_propertyColorDialogPanel->isVisible()) {
        updatePropertyColorDialogGeometry();
        m_controls.m_propertyColorDialogPanel->raise();
    }
}

void ShotWindow::updatePropertyColorDialogGeometry()
{
    if (!m_controls.m_propertyColorDialogPanel || !m_controls.m_annotationPropertyPanel) {
        return;
    }

    m_controls.m_propertyColorDialogPanel->adjustSize();
    QSize panelSize = m_controls.m_propertyColorDialogPanel->sizeHint();
    panelSize.setWidth(std::min(panelSize.width(), std::max(160, width() - 16)));
    panelSize.setHeight(std::min(panelSize.height(), std::max(180, height() - 16)));

    // Anchor on the color button's centre so the picker stays put when the
    // property panel resizes (e.g. fill/radius/font slots showing or hiding).
    // Falling back to the property panel keeps geometry valid when the
    // button is hidden (mosaic case).
    QPoint anchor;
    if (m_controls.m_propertyColorEditingTextBackground && m_controls.m_propertyTextBackgroundButton && m_controls.m_propertyTextBackgroundButton->isVisible()) {
        anchor = m_controls.m_propertyTextBackgroundButton->mapTo(this, m_controls.m_propertyTextBackgroundButton->rect().center());
    } else if (m_controls.m_propertyColorButton && m_controls.m_propertyColorButton->isVisible()) {
        anchor = m_controls.m_propertyColorButton->mapTo(this, m_controls.m_propertyColorButton->rect().center());
    } else {
        const QRect propertyRect = m_controls.m_annotationPropertyPanel->geometry();
        anchor = QPoint(propertyRect.center().x(), propertyRect.bottom());
    }

    int x = anchor.x() - panelSize.width() / 2;
    int y = anchor.y() + 14;

    const QRect propertyRect = m_controls.m_annotationPropertyPanel->geometry();
    if (y + panelSize.height() > height() - 8) {
        // Place above the property panel instead.
        y = propertyRect.top() - panelSize.height() - 8;
    }
    x = std::clamp(x, 8, std::max(8, width() - panelSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - panelSize.height() - 8));
    m_controls.m_propertyColorDialogPanel->setGeometry(x, y, panelSize.width(), panelSize.height());
}

/**
 * 【标注属性栏】【字体弹层】计算字体弹层尺寸与位置，列表高度扣除底部字号与 B/I 控件行，避免两者重叠。
 * @return 无返回值。
 */
void ShotWindow::updatePropertyFontPanelGeometry()
{
    if (!m_controls.m_propertyFontPanel || !m_controls.m_annotationPropertyPanel || !m_controls.m_propertyFontButton) {
        return;
    }

    // 1. 底部控件行高度取字号输入框与 B/I 按钮的较大值，列表高度需扣除该行避免重叠
    namespace popup = markshot::shot_popup;
    int controlsHeight = 0;
    for (QWidget *control : {static_cast<QWidget *>(m_controls.m_propertyFontSizeSpin),
                             static_cast<QWidget *>(m_controls.m_propertyFontBoldButton),
                             static_cast<QWidget *>(m_controls.m_propertyFontItalicButton)}) {
        if (control) {
            controlsHeight = std::max(controlsHeight, control->sizeHint().expandedTo(control->minimumSize()).height());
        }
    }
    const int rowCount = m_controls.m_propertyFontList ? m_controls.m_propertyFontList->count() : 1;
    const int rowHeight = m_controls.m_propertyFontList ? std::max(24, m_controls.m_propertyFontList->sizeHintForRow(0)) : 28;
    const int searchHeight = m_controls.m_propertyFontSearch ? m_controls.m_propertyFontSearch->height() : 0;
    const popup::FontPanelLayout layout = popup::fontPanelLayout(360, rowHeight, rowCount, controlsHeight, size(),
                                                                 6, 6, searchHeight);

    // 2. 弹层以字体按钮为横向中心，纵向位于属性栏下方并保持 6-8px 间距
    const QPoint anchor = m_controls.m_propertyFontButton->mapTo(this, QPoint(m_controls.m_propertyFontButton->width() / 2, 0));
    const QRect placed = popup::popupBelowAnchor(m_controls.m_annotationPropertyPanel->geometry(),
                                                 anchor.x() - layout.panelSize.width() / 2, layout.panelSize, rect());
    if (m_controls.m_propertyFontList) {
        m_controls.m_propertyFontList->setFixedHeight(layout.listRect.height());
    }
    m_controls.m_propertyFontPanel->setFixedSize(layout.panelSize);
    m_controls.m_propertyFontPanel->setGeometry(placed);
}
