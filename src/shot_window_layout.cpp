#include "shot_window_module.h"

namespace cfg = markshot::config;
namespace shortcuts = markshot::shortcut;
using namespace markshot::shot;

QRectF ShotWindow::textContentRect(const Annotation &annotation, bool widgetCoordinates) const
{
    const qreal scale = annotationSizeScale(widgetCoordinates);
    const QRectF baseRect = annotation.rect.isEmpty()
        ? QRectF(annotation.points.value(0), QSizeF(360.0, 140.0))
        : annotation.rect.normalized();
    const QPointF topLeft = widgetCoordinates ? imageToWidget(baseRect.topLeft()) : baseRect.topLeft();
    const qreal wrapWidth = std::max<qreal>(16.0, baseRect.width() * scale - kTextBackgroundPaddingX * 2.0 * scale);

    QFont font = markshot::theme::textFont(qRound((19.0 + annotation.width) * scale),
                                           QFont::DemiBold,
                                           annotation.fontFamily);
    QTextOption option;
    option.setWrapMode(QTextOption::WrapAtWordBoundaryOrAnywhere);
    option.setAlignment(Qt::AlignLeft | Qt::AlignTop);

    QTextDocument document;
    document.setDocumentMargin(0.0);
    document.setDefaultFont(font);
    document.setDefaultTextOption(option);
    document.setPlainText(annotation.text);
    document.setTextWidth(wrapWidth);

    const QSizeF documentSize = document.size();
    qreal textWidth = 0.0;
    qreal textHeight = 0.0;
    for (QTextBlock block = document.begin(); block.isValid(); block = block.next()) {
        const QTextLayout *layout = block.layout();
        if (!layout) {
            continue;
        }
        for (int i = 0; i < layout->lineCount(); ++i) {
            const QTextLine line = layout->lineAt(i);
            textWidth = std::max(textWidth, line.naturalTextWidth());
            textHeight = std::max(textHeight, layout->position().y() + line.y() + line.height());
        }
    }
    if (textWidth <= 0.0 || textHeight <= 0.0) {
        textWidth = documentSize.width();
        textHeight = documentSize.height();
    }

    const qreal rectWidth = std::max<qreal>(1.0, std::ceil(textWidth + kTextBackgroundPaddingX * 2.0 * scale) + 5.0);
    const qreal rectHeight = std::max<qreal>(1.0, std::ceil(textHeight + kTextBackgroundPaddingY * 2.0 * scale));
    return QRectF(topLeft, QSizeF(rectWidth, rectHeight));
}

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
    if (!m_horizontalImageScrollBar || !m_verticalImageScrollBar) {
        return;
    }
    if (!imageNavigationAvailable() || m_frozenFrame.isNull() || m_frozenImageRect.isEmpty()) {
        m_horizontalImageScrollBar->hide();
        m_verticalImageScrollBar->hide();
        return;
    }

    const qreal scale = m_frozenImageRect.width() / std::max<qreal>(1.0, m_frozenFrame.width());
    if (scale <= 0.0) {
        m_horizontalImageScrollBar->hide();
        m_verticalImageScrollBar->hide();
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
    m_horizontalImageScrollBar->setGeometry(0,
                                            height() - kImageScrollBarExtent,
                                            horizontalWidth,
                                            kImageScrollBarExtent);
    m_verticalImageScrollBar->setGeometry(width() - kImageScrollBarExtent,
                                          0,
                                          kImageScrollBarExtent,
                                          verticalHeight);

    const QSignalBlocker blockHorizontal(m_horizontalImageScrollBar);
    const QSignalBlocker blockVertical(m_verticalImageScrollBar);
    m_syncingImageScrollBars = true;
    m_horizontalImageScrollBar->setRange(0, maxX);
    m_horizontalImageScrollBar->setPageStep(std::max(1, width()));
    m_horizontalImageScrollBar->setSingleStep(std::max(1, width() / 12));
    m_horizontalImageScrollBar->setValue(std::clamp(qRound(-m_frozenImageRect.left()), 0, maxX));
    m_verticalImageScrollBar->setRange(0, maxY);
    m_verticalImageScrollBar->setPageStep(std::max(1, height()));
    m_verticalImageScrollBar->setSingleStep(std::max(1, height() / 12));
    m_verticalImageScrollBar->setValue(std::clamp(qRound(-m_frozenImageRect.top()), 0, maxY));
    m_syncingImageScrollBars = false;

    m_horizontalImageScrollBar->setVisible(showHorizontal);
    m_verticalImageScrollBar->setVisible(showVertical);
    if (showHorizontal) {
        m_horizontalImageScrollBar->raise();
    }
    if (showVertical) {
        m_verticalImageScrollBar->raise();
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

    const bool hasHorizontal = m_horizontalImageScrollBar && m_horizontalImageScrollBar->maximum() > 0;
    const bool hasVertical = m_verticalImageScrollBar && m_verticalImageScrollBar->maximum() > 0;
    const qreal centerX = hasHorizontal
        ? (m_horizontalImageScrollBar->value() + width() / 2.0) / scale
        : m_frozenFrame.width() / 2.0;
    const qreal centerY = hasVertical
        ? (m_verticalImageScrollBar->value() + height() / 2.0) / scale
        : m_frozenFrame.height() / 2.0;

    m_imageCenter = QPointF(centerX, centerY);
    m_imageCenterInitialized = true;
    updateFrozenImageRect();
    refreshViewGeometry();
    update();
}

void ShotWindow::updateMinimumImageWindowSize()
{
    if (!m_imageNavigationEnabled || !m_toolbar) {
        setMinimumSize(QSize(0, 0));
        return;
    }

    m_toolbar->adjustSize();
    const int minWidth = m_toolbar->sizeHint().width() + kImageWindowMinimumToolbarPadding;
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
}

QRect ShotWindow::clampedToolbarGeometry(QRect toolbarGeometry) const
{
    toolbarGeometry.moveLeft(std::clamp(toolbarGeometry.left(), 8, std::max(8, width() - toolbarGeometry.width() - 8)));
    toolbarGeometry.moveTop(std::clamp(toolbarGeometry.top(), 8, std::max(8, height() - toolbarGeometry.height() - 8)));
    return toolbarGeometry;
}

void ShotWindow::updateToolbarGeometry()
{
    if (!m_toolbar || !hasUsableSelection()) {
        return;
    }

    m_toolbar->adjustSize();
    if (m_toolbarUserPlaced) {
        const QSize toolbarSize = m_toolbar->sizeHint();
        QRect toolbarGeometry = m_toolbar->geometry();
        toolbarGeometry.setSize(toolbarSize);
        m_toolbar->setGeometry(clampedToolbarGeometry(toolbarGeometry));
        updateAnnotationPropertyPanelGeometry();
        return;
    }
    if (m_imageNavigationEnabled && m_fullscreenAnnotation) {
        const QSize toolbarSize = m_toolbar->sizeHint();
        const QRect toolbarGeometry(QPoint(qRound((width() - toolbarSize.width()) / 2.0), 12), toolbarSize);
        m_toolbar->setGeometry(clampedToolbarGeometry(toolbarGeometry));
        updateAnnotationPropertyPanelGeometry();
        return;
    }

    const QRectF selection = imageRectToWidget(normalizedSelection());
    const QSize toolbarSize = m_toolbar->sizeHint();
    int x = qRound(selection.center().x() - toolbarSize.width() / 2.0);
    int y = qRound(selection.bottom() + kToolbarMargin);

    x = std::clamp(x, 8, std::max(8, width() - toolbarSize.width() - 8));
    if (y + toolbarSize.height() > height() - 8) {
        y = qRound(selection.top() - toolbarSize.height() - kToolbarMargin);
    }
    y = std::clamp(y, 8, std::max(8, height() - toolbarSize.height() - 8));
    m_toolbar->setGeometry(x, y, toolbarSize.width(), toolbarSize.height());
    updateAnnotationPropertyPanelGeometry();
}

void ShotWindow::updateActionToolbarGeometry()
{
    if (!m_actionToolbar || !hasUsableSelection() || m_fullscreenAnnotation) {
        return;
    }

    m_actionToolbar->adjustSize();
    const QRectF selection = imageRectToWidget(normalizedSelection());
    const QSize toolbarSize = m_actionToolbar->sizeHint();
    const QRect selectionRect = selection.toAlignedRect();
    const QRect toolbarRect = m_toolbar && m_toolbar->isVisible() ? m_toolbar->geometry() : QRect();
    const QRect propertyRect = m_annotationPropertyPanel && m_annotationPropertyPanel->isVisible()
        ? m_annotationPropertyPanel->geometry()
        : QRect();

    auto clamped = [this, toolbarSize](QPoint topLeft) {
        const int x = std::clamp(topLeft.x(), 8, std::max(8, width() - toolbarSize.width() - 8));
        const int y = std::clamp(topLeft.y(), 8, std::max(8, height() - toolbarSize.height() - 8));
        return QRect(QPoint(x, y), toolbarSize);
    };
    auto clearOfPanels = [toolbarRect, propertyRect, selectionRect](const QRect &candidate) {
        const QRect padded = candidate.adjusted(-4, -4, 4, 4);
        return (toolbarRect.isNull() || !padded.intersects(toolbarRect))
            && (propertyRect.isNull() || !padded.intersects(propertyRect))
            && !padded.intersects(selectionRect);
    };

    const int selectionCenterY = qRound(selection.center().y() - toolbarSize.height() / 2.0);
    QVector<QRect> candidates = {
        clamped(QPoint(qRound(selection.right() + kToolbarMargin), selectionCenterY)),
        clamped(QPoint(qRound(selection.left() - toolbarSize.width() - kToolbarMargin), selectionCenterY)),
    };
    if (!toolbarRect.isNull()) {
        candidates.append(clamped(QPoint(toolbarRect.right() + kToolbarMargin, toolbarRect.top())));
        candidates.append(clamped(QPoint(toolbarRect.left() - toolbarSize.width() - kToolbarMargin, toolbarRect.top())));
        candidates.append(clamped(QPoint(toolbarRect.right() - toolbarSize.width(), toolbarRect.bottom() + kToolbarMargin)));
        candidates.append(clamped(QPoint(toolbarRect.right() - toolbarSize.width(), toolbarRect.top() - toolbarSize.height() - kToolbarMargin)));
    }

    for (const QRect &candidate : candidates) {
        if (clearOfPanels(candidate)) {
            m_actionToolbar->setGeometry(candidate);
            return;
        }
    }
    m_actionToolbar->setGeometry(candidates.first());
}

void ShotWindow::updateAnnotationPropertyPanel()
{
    if (!m_annotationPropertyPanel) {
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
        m_annotationPropertyPanel->hide();
        if (m_propertyColorDialogPanel) {
            m_propertyColorDialogPanel->hide();
        }
        if (m_propertyFontPanel) {
            m_propertyFontPanel->hide();
        }
        return;
    }

    QString title = QStringLiteral("Object");
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

    switch (panelTool) {
    case Tool::Move:
    case Tool::Select:
        title = QStringLiteral("Object");
        break;
    case Tool::Pen:
        title = QStringLiteral("Pen");
        break;
    case Tool::Highlighter:
        title = QStringLiteral("Highlighter");
        break;
    case Tool::Line:
        title = QStringLiteral("Line");
        break;
    case Tool::Rectangle:
        title = QStringLiteral("Rect");
        break;
    case Tool::Ellipse:
        title = QStringLiteral("Ellipse");
        break;
    case Tool::Arrow:
        title = QStringLiteral("Arrow");
        break;
    case Tool::Text:
        title = QStringLiteral("Text");
        break;
    case Tool::Number:
        title = QStringLiteral("Number");
        break;
    case Tool::Mosaic:
        title = QStringLiteral("Mosaic");
        break;
    case Tool::Magnifier:
        title = QStringLiteral("Magnifier");
        break;
    case Tool::Laser:
        title = QStringLiteral("Laser");
        break;
    }

    if (m_annotationPropertyTitle) {
        m_annotationPropertyTitle->setText(groupSelection
                                               ? MS_TR("Group %1").arg(selectedIds.size())
                                               : markshot::i18n::translate(title));
    }
    if (m_propertyEditTextButton) {
        m_propertyEditTextButton->setVisible(!groupSelection && editingAnnotation && panelTool == Tool::Text);
    }
    if (m_propertyFontButton) {
        m_propertyFontButton->setVisible(!groupSelection && panelTool == Tool::Text);
        if (!groupSelection && panelTool == Tool::Text) {
            const QString family = panelFontFamily.isEmpty() ? markshot::theme::textFontFamily() : panelFontFamily;
            m_propertyFontButton->setToolTip(family);
            if (m_propertyFontList) {
                const auto matches = m_propertyFontList->findItems(family, Qt::MatchExactly);
                if (!matches.isEmpty()) {
                    m_propertyFontList->setCurrentItem(matches.first());
                    m_propertyFontList->scrollToItem(matches.first(), QAbstractItemView::PositionAtCenter);
                }
            }
        } else if (m_propertyFontPanel) {
            m_propertyFontPanel->hide();
            m_propertyFontButton->setToolTip(MS_TR("Text font"));
        }
    }
    if (m_propertyFillButton) {
        // Highlight/Invert 风格不使用 filled 字段,隐藏填充开关避免歧义
        const bool supportsFill = !groupSelection
            && (panelTool == Tool::Ellipse
                || (panelTool == Tool::Rectangle && panelRectangleStyle == RectangleStyle::Stroke));
        m_propertyFillButton->setVisible(supportsFill);
        const QSignalBlocker blocker(m_propertyFillButton);
        m_propertyFillButton->setChecked(panelFilled);
        m_propertyFillButton->setIcon(markshot::ui::makeFillIcon(panelFilled));
    }
    if (m_propertyRadiusGlyphLabel) {
        m_propertyRadiusGlyphLabel->setVisible(!groupSelection
                                               && panelTool == Tool::Rectangle
                                               && panelRectangleStyle == RectangleStyle::Stroke);
    }
    if (m_propertyRadiusLabel) {
        m_propertyRadiusLabel->setVisible(!groupSelection
                                          && panelTool == Tool::Rectangle
                                          && panelRectangleStyle == RectangleStyle::Stroke);
        m_propertyRadiusLabel->setText(QString::number(qRound(panelRadius)));
    }
    if (m_propertyRadiusSlider) {
        m_propertyRadiusSlider->setVisible(!groupSelection
                                           && panelTool == Tool::Rectangle
                                           && panelRectangleStyle == RectangleStyle::Stroke);
        const QSignalBlocker blocker(m_propertyRadiusSlider);
        m_propertyRadiusSlider->setValue(qRound(panelRadius));
    }
    if (m_propertyRectangleStyleCombo) {
        const bool supportsRectangleStyle = !groupSelection && panelTool == Tool::Rectangle;
        m_propertyRectangleStyleCombo->setVisible(supportsRectangleStyle);
        if (supportsRectangleStyle) {
            const QSignalBlocker blocker(m_propertyRectangleStyleCombo);
            const int styleValue = static_cast<int>(panelRectangleStyle);
            for (int i = 0; i < m_propertyRectangleStyleCombo->count(); ++i) {
                if (m_propertyRectangleStyleCombo->itemData(i).toInt() == styleValue) {
                    m_propertyRectangleStyleCombo->setCurrentIndex(i);
                    break;
                }
            }
        }
    }
    if (m_propertyArrowStyleCombo) {
        const bool supportsArrowStyle = !groupSelection && panelTool == Tool::Arrow;
        m_propertyArrowStyleCombo->setVisible(supportsArrowStyle);
        if (supportsArrowStyle) {
            const ArrowStyle panelArrowStyle = annotation ? annotation->arrowStyle : m_arrowStyle;
            const QSignalBlocker blocker(m_propertyArrowStyleCombo);
            m_propertyArrowStyleCombo->setCurrentIndex(std::max(0, m_propertyArrowStyleCombo->findData(static_cast<int>(panelArrowStyle))));
        }
    }
    if (m_propertyHighlighterStyleCombo) {
        const bool supportsHighlighterStyle = !groupSelection && panelTool == Tool::Highlighter;
        m_propertyHighlighterStyleCombo->setVisible(supportsHighlighterStyle);
        if (supportsHighlighterStyle) {
            const QSignalBlocker blocker(m_propertyHighlighterStyleCombo);
            m_propertyHighlighterStyleCombo->setCurrentIndex(
                panelHighlighterStyle == HighlighterStyle::StraightLine ? 1 : 0);
        }
    }
    if (m_propertyNumberStyleCombo) {
        const bool supportsNumberStyle = !groupSelection && panelTool == Tool::Number;
        m_propertyNumberStyleCombo->setVisible(supportsNumberStyle);
        if (supportsNumberStyle) {
            const QSignalBlocker blocker(m_propertyNumberStyleCombo);
            const int numberStyleValue = static_cast<int>(panelNumberStyle);
            for (int i = 0; i < m_propertyNumberStyleCombo->count(); ++i) {
                if (m_propertyNumberStyleCombo->itemData(i).toInt() == numberStyleValue) {
                    m_propertyNumberStyleCombo->setCurrentIndex(i);
                    break;
                }
            }
        }
    }
    if (m_propertyResetNumberButton) {
        m_propertyResetNumberButton->setVisible(!groupSelection && panelTool == Tool::Number);
    }
    const bool supportsMagnifierScale = !groupSelection && panelTool == Tool::Magnifier;
    if (m_propertyMagnifierScaleGlyphLabel) {
        m_propertyMagnifierScaleGlyphLabel->setVisible(supportsMagnifierScale);
    }
    if (m_propertyMagnifierScaleLabel) {
        m_propertyMagnifierScaleLabel->setVisible(supportsMagnifierScale);
        m_propertyMagnifierScaleLabel->setText(magnifierScaleText(panelMagnifierScale));
    }
    if (m_propertyMagnifierScaleSlider) {
        m_propertyMagnifierScaleSlider->setVisible(supportsMagnifierScale);
        const QSignalBlocker blocker(m_propertyMagnifierScaleSlider);
        m_propertyMagnifierScaleSlider->setValue(magnifierScaleSliderValue(panelMagnifierScale));
    }
    if (m_propertyMagnifierShapeButton) {
        m_propertyMagnifierShapeButton->setVisible(supportsMagnifierScale);
        const MagnifierShape panelShape = annotation && annotation->tool == Tool::Magnifier
            ? annotation->magnifierShape
            : m_magnifierShape;
        const QSignalBlocker blocker(m_propertyMagnifierShapeButton);
        m_propertyMagnifierShapeButton->setChecked(panelShape == MagnifierShape::Rectangle);
    }
    if (m_propertyWidthLabel) {
        m_propertyWidthLabel->setText(QString::number(qRound(panelWidth)));
    }
    if (m_propertyWidthSlider) {
        const QSignalBlocker blocker(m_propertyWidthSlider);
        if (panelTool == Tool::Mosaic) {
            m_propertyWidthSlider->setRange(qRound(kMinMosaicBlockSize), qRound(kMaxMosaicBlockSize));
        } else if (panelTool == Tool::Number) {
            m_propertyWidthSlider->setRange(qRound(kMinNumberWidth), qRound(kMaxNumberWidth));
        } else if (panelTool == Tool::Text) {
            m_propertyWidthSlider->setRange(1, 1000);
        } else if (panelTool == Tool::Highlighter) {
            m_propertyWidthSlider->setRange(qRound(kMinStrokeWidth), qRound(kMaxHighlighterWidth));
        } else {
            m_propertyWidthSlider->setRange(qRound(kMinStrokeWidth), qRound(kMaxStrokeWidth));
        }
        m_propertyWidthSlider->setValue(qRound(panelWidth));
    }
    if (m_propertyOpacityLabel) {
        m_propertyOpacityLabel->setText(QStringLiteral("%1%").arg(panelOpacity));
    }
    if (m_propertyOpacitySlider) {
        const QSignalBlocker blocker(m_propertyOpacitySlider);
        m_propertyOpacitySlider->setValue(panelOpacity);
    }
    if (m_propertyColorButton) {
        m_propertyColorButton->setStyleSheet(markshot::theme::propertyColorButtonStyleSheet(panelColor));
        m_propertyColorButton->setIcon(markshot::ui::makePropertyIcon(
            markshot::ui::PropertyIcon::Color, propertyIconInkForFill(panelColor)));
        m_propertyColorButton->setVisible(panelTool != Tool::Mosaic);
        if (panelTool == Tool::Mosaic && m_propertyColorDialogPanel) {
            m_propertyColorDialogPanel->hide();
        }
    }
    if (m_propertyTextBackgroundButton) {
        const bool supportsTextBackground = !groupSelection && panelTool == Tool::Text;
        m_propertyTextBackgroundButton->setVisible(supportsTextBackground);
        m_propertyTextBackgroundButton->setStyleSheet(markshot::theme::propertyColorButtonStyleSheet(panelTextBackgroundColor));
        m_propertyTextBackgroundButton->setIcon(markshot::ui::makePropertyIcon(
            markshot::ui::PropertyIcon::TextBackground, propertyIconInkForFill(panelTextBackgroundColor)));
        if (!supportsTextBackground && m_propertyColorDialogPanel && m_propertyColorEditingTextBackground) {
            m_propertyColorDialogPanel->hide();
        }
    }
    if (m_propertyColorPicker && m_propertyColorDialogPanel && m_propertyColorDialogPanel->isVisible()) {
        const QSignalBlocker blocker(m_propertyColorPicker);
        m_propertyColorPicker->setColor(m_propertyColorEditingTextBackground ? panelTextBackgroundColor : panelColor);
    }

    m_annotationPropertyPanel->show();
    if (QLayout *panelLayout = m_annotationPropertyPanel->layout()) {
        panelLayout->activate();
    }
    updateAnnotationPropertyPanelGeometry();
    m_annotationPropertyPanel->raise();
    if (m_propertyColorDialogPanel && m_propertyColorDialogPanel->isVisible()) {
        updatePropertyColorDialogGeometry();
        m_propertyColorDialogPanel->raise();
    }
}

void ShotWindow::updateAnnotationPropertyPanelGeometry()
{
    if (!m_annotationPropertyPanel) {
        return;
    }

    m_annotationPropertyPanel->adjustSize();
    const QSize panelSize = m_annotationPropertyPanel->sizeHint();
    const QRect toolbarRect = m_toolbar && m_toolbar->isVisible()
        ? m_toolbar->geometry()
        : QRect(8, 8, 0, 0);
    int x = toolbarRect.left();
    int y = toolbarRect.bottom() + kToolbarMargin;
    if (y + panelSize.height() > height() - 8) {
        y = toolbarRect.top() - panelSize.height() - kToolbarMargin;
    }
    if (x + panelSize.width() > width() - 8) {
        x = toolbarRect.right() - panelSize.width();
    }
    x = std::clamp(x, 8, std::max(8, width() - panelSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - panelSize.height() - 8));
    m_annotationPropertyPanel->setGeometry(x, y, panelSize.width(), panelSize.height());
    updatePropertyColorDialogGeometry();
    updatePropertyFontPanelGeometry();
}

void ShotWindow::updatePropertyColorDialogGeometry()
{
    if (!m_propertyColorDialogPanel || !m_annotationPropertyPanel) {
        return;
    }

    m_propertyColorDialogPanel->adjustSize();
    QSize panelSize = m_propertyColorDialogPanel->sizeHint();
    panelSize.setWidth(std::min(panelSize.width(), std::max(160, width() - 16)));
    panelSize.setHeight(std::min(panelSize.height(), std::max(180, height() - 16)));

    // Anchor on the color button's centre so the picker stays put when the
    // property panel resizes (e.g. fill/radius/font slots showing or hiding).
    // Falling back to the property panel keeps geometry valid when the
    // button is hidden (mosaic case).
    QPoint anchor;
    if (m_propertyColorEditingTextBackground && m_propertyTextBackgroundButton && m_propertyTextBackgroundButton->isVisible()) {
        anchor = m_propertyTextBackgroundButton->mapTo(this, m_propertyTextBackgroundButton->rect().center());
    } else if (m_propertyColorButton && m_propertyColorButton->isVisible()) {
        anchor = m_propertyColorButton->mapTo(this, m_propertyColorButton->rect().center());
    } else {
        const QRect propertyRect = m_annotationPropertyPanel->geometry();
        anchor = QPoint(propertyRect.center().x(), propertyRect.bottom());
    }

    int x = anchor.x() - panelSize.width() / 2;
    int y = anchor.y() + 14;

    const QRect propertyRect = m_annotationPropertyPanel->geometry();
    if (y + panelSize.height() > height() - 8) {
        // Place above the property panel instead.
        y = propertyRect.top() - panelSize.height() - 8;
    }
    x = std::clamp(x, 8, std::max(8, width() - panelSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - panelSize.height() - 8));
    m_propertyColorDialogPanel->setGeometry(x, y, panelSize.width(), panelSize.height());
}

void ShotWindow::updatePropertyFontPanelGeometry()
{
    if (!m_propertyFontPanel || !m_annotationPropertyPanel || !m_propertyFontButton) {
        return;
    }

    const int visibleRows = std::min(10, m_propertyFontList ? std::max(1, m_propertyFontList->count()) : 1);
    const int rowHeight = m_propertyFontList ? std::max(24, m_propertyFontList->sizeHintForRow(0)) : 28;
    QSize panelSize(260, std::min(280, visibleRows * rowHeight + 18));
    panelSize.setWidth(std::min(panelSize.width(), std::max(180, width() - 16)));
    panelSize.setHeight(std::min(panelSize.height(), std::max(120, height() - 16)));

    QPoint anchor = m_propertyFontButton->mapTo(this, QPoint(m_propertyFontButton->width() / 2,
                                                            m_propertyFontButton->height()));
    int x = anchor.x() - panelSize.width() / 2;
    int y = anchor.y() + 10;
    const QRect propertyRect = m_annotationPropertyPanel->geometry();
    if (y + panelSize.height() > height() - 8) {
        y = propertyRect.top() - panelSize.height() - 8;
    }
    x = std::clamp(x, 8, std::max(8, width() - panelSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - panelSize.height() - 8));
    if (m_propertyFontList) {
        m_propertyFontList->setFixedHeight(std::max(80, panelSize.height() - 16));
    }
    m_propertyFontPanel->setFixedSize(panelSize);
    m_propertyFontPanel->setGeometry(x, y, panelSize.width(), panelSize.height());
}
