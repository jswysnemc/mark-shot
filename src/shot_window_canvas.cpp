#include "shot_window_module.h"

#include "debug_log.h"
#include "selection_loupe.h"

namespace cfg = markshot::config;
namespace shortcuts = markshot::shortcut;
using namespace markshot::shot;

/// @brief 绘制截图、标注和选区辅助层
/// @param event 本次需要重绘的区域
/// @return 无返回值
void ShotWindow::paintEvent(QPaintEvent *event)
{
    QPainter painter(this);
    if (event) {
        painter.setClipRegion(event->region());
    }
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.fillRect(rect(), QColor(0, 0, 0));
    const qreal imageScale = m_frozenFrame.isNull()
        ? 1.0
        : m_frozenImageRect.width() / std::max<qreal>(1.0, m_frozenFrame.width());
    const QRectF visibleImageRect = m_frozenImageRect.intersected(QRectF(rect()));
    const qreal dpr = devicePixelRatioF();
    const QSize sharpTargetSize(qMax(1, qRound(visibleImageRect.width() * dpr)),
                                qMax(1, qRound(visibleImageRect.height() * dpr)));
    const qsizetype sharpPixels =
        static_cast<qsizetype>(sharpTargetSize.width()) * sharpTargetSize.height();
    if (imageNavigationAvailable() && imageScale > 1.01 && !visibleImageRect.isEmpty()
        && sharpPixels <= kMaxSharpViewportPixels) {
        const QRectF sourceRect((visibleImageRect.left() - m_frozenImageRect.left()) / imageScale,
                                (visibleImageRect.top() - m_frozenImageRect.top()) / imageScale,
                                visibleImageRect.width() / imageScale,
                                visibleImageRect.height() / imageScale);
        const bool cacheHit = !m_sharpViewportCache.isNull()
            && m_sharpViewportCacheSourceRect == sourceRect
            && m_sharpViewportCacheTargetSize == sharpTargetSize
            && qFuzzyCompare(m_sharpViewportCacheDpr, dpr);
        QImage rendered = cacheHit
            ? m_sharpViewportCache
            : renderSharpViewport(m_frozenFrame, sourceRect, sharpTargetSize);
        if (!rendered.isNull()) {
            rendered.setDevicePixelRatio(dpr);
            if (!cacheHit) {
                m_sharpViewportCache = rendered;
                m_sharpViewportCacheSourceRect = sourceRect;
                m_sharpViewportCacheTargetSize = sharpTargetSize;
                m_sharpViewportCacheDpr = dpr;
            }
            painter.drawImage(visibleImageRect, rendered);
        } else {
            m_sharpViewportCache = {};
            painter.drawImage(m_frozenImageRect, m_frozenFrame);
        }
    } else {
        m_sharpViewportCache = {};
        painter.drawImage(m_frozenImageRect, m_frozenFrame);
    }

    const QRectF selection = normalizedSelection();
    QPainterPath dimPath;
    dimPath.addRect(rect());
    if (hasUsableSelection()) {
        dimPath.addRect(imageRectToWidget(selection));
        painter.fillPath(dimPath, QColor(2, 6, 12, 128));
    } else {
        painter.fillRect(rect(), QColor(2, 6, 12, 88));
    }

    if (hasUsableSelection()) {
        const QRectF widgetSelection = imageRectToWidget(selection);
        painter.save();
        for (const Annotation &annotation : m_annotations) {
            if (m_editingTextAnnotationId.has_value() && annotation.id == *m_editingTextAnnotationId) {
                continue;
            }
            drawAnnotation(painter, annotation, true);
        }
        if (m_draft.has_value()) {
            drawAnnotation(painter, *m_draft, true);
        }
        const qint64 now = m_laserClock.isValid() ? m_laserClock.elapsed() : 0;
        for (const LaserStroke &stroke : m_laserStrokes) {
            const qreal opacity = std::clamp(static_cast<qreal>(stroke.expiresAt - now) / kLaserLifetimeMs, 0.0, 1.0);
            if (opacity > 0.0) {
                drawLaserStroke(painter, stroke, true, opacity);
            }
        }
        if (m_laserDraft.has_value()) {
            drawLaserStroke(painter, *m_laserDraft, true, 1.0);
        }
        drawSelectedAnnotationFrame(painter);
        if (m_annotationSelectionBoxActive) {
            const QRectF box = imageRectToWidget(m_annotationSelectionBox.normalized());
            painter.setPen(QPen(QColor(45, 212, 191), 1.5, Qt::DashLine));
            painter.setBrush(QColor(45, 212, 191, 34));
            painter.drawRoundedRect(box, 4.0, 4.0);
        }
        painter.restore();

        painter.setPen(QPen(QColor(94, 234, 212), 2.0));
        painter.setBrush(Qt::NoBrush);
        painter.drawRoundedRect(widgetSelection, 3.0, 3.0);

        if (m_tool == Tool::Move && !m_fullscreenAnnotation) {
            painter.setPen(Qt::NoPen);
            painter.setBrush(QColor(94, 234, 212));
            const QVector<QPointF> handles = {
                widgetSelection.topLeft(), QPointF(widgetSelection.center().x(), widgetSelection.top()), widgetSelection.topRight(),
                QPointF(widgetSelection.left(), widgetSelection.center().y()), QPointF(widgetSelection.right(), widgetSelection.center().y()),
                widgetSelection.bottomLeft(), QPointF(widgetSelection.center().x(), widgetSelection.bottom()), widgetSelection.bottomRight(),
            };
            for (const QPointF &handle : handles) {
                painter.drawRoundedRect(QRectF(handle.x() - 4.0, handle.y() - 4.0, 8.0, 8.0), 2.0, 2.0);
            }
        }

        const bool selectionInfoVisible = m_selectionDrag != SelectionDrag::None
            || (m_showSelectionInfo && m_selectionInfoTimer.isValid() && m_selectionInfoTimer.elapsed() <= 1000);
        if (selectionInfoVisible) {
            const QString sizeText = QStringLiteral("%1 x %2").arg(qRound(selection.width())).arg(qRound(selection.height()));
            painter.setFont(markshot::theme::uiFont(11, QFont::DemiBold));
            const QFontMetrics metrics(painter.font());
            const QRectF labelRect(widgetSelection.left() + 10.0,
                                   widgetSelection.top() + 10.0,
                                   metrics.horizontalAdvance(sizeText) + 22.0,
                                   metrics.height() + 12.0);
            painter.setPen(Qt::NoPen);
            painter.setBrush(QColor(8, 13, 19, 220));
            painter.drawRoundedRect(labelRect, 10.0, 10.0);
            painter.setPen(QColor(204, 251, 241, 238));
            painter.drawText(labelRect, Qt::AlignCenter, sizeText);
        } else if (m_showSelectionInfo) {
            m_showSelectionInfo = false;
        }
    }

    if (m_hoveredWindowRect.has_value()
        && m_mode == Mode::Selecting
        && (m_startupTool == StartupTool::None
            || m_startupTool == StartupTool::CodeScanner
            || recordingModeForStartupTool(m_startupTool).has_value())) {
        const QRectF hoverWidget = imageRectToWidget(QRectF(*m_hoveredWindowRect));
        painter.setPen(QPen(QColor(94, 234, 212), 2.0));
        painter.setBrush(QColor(94, 234, 212, 32));
        painter.drawRect(hoverWidget);
    }

    drawStartupToolOverlay(painter);
    drawSelectionAdjustmentOverlay(painter);
    drawStartupShortcutHint(painter);
    drawActiveRecordingStatus(painter);

    drawWheelPreview(painter);
}

void ShotWindow::resizeEvent(QResizeEvent *)
{
    updateFrozenImageRect();
    updateDisplayCapturePickerGeometry();
    if (m_colorPalette && m_colorPalette->isVisible()) {
        updateColorPaletteGeometry(m_colorPaletteAnchor);
    }
    if (m_shapeMarkerPopup && m_shapeMarkerPopup->isVisible()) {
        updateShapeMarkerPopupGeometry();
    }
    updateTextEditorGeometry();
    updateImageScrollBars();
    updateToolbarGeometry();
    updateActionToolbarGeometry();
    updateAnnotationPropertyPanelGeometry();
    updateOpenWithPanelGeometry();
    updateExtensionPanelGeometry();
}

void ShotWindow::showEvent(QShowEvent *event)
{
    QWidget::showEvent(event);
    markshot::windows::setExcludedFromCapture(this);
    if (auto *handle = windowHandle()) {
        disconnect(handle, &QWindow::screenChanged, this, nullptr);
        connect(handle, &QWindow::screenChanged, this, [this](QScreen *newScreen) {
            markshot::debugLog("capture-session",
                               "【截图会话】【屏幕切换】new=%s dpr=%.3f",
                               newScreen ? newScreen->name().toUtf8().constData() : "(none)",
                               newScreen ? newScreen->devicePixelRatio() : 0.0);
            updateFrozenImageRect();
            update();
        });
    }
}

/// @brief 按当前工具开始选区或标注操作
/// @param event 鼠标按下事件
/// @return 无返回值
void ShotWindow::mousePressEvent(QMouseEvent *event)
{
    clearWheelPreview();

    if (m_mode == Mode::Selecting
        && displayCapturePickerVisible()
        && !displayCapturePickerContains(event->pos())) {
        hideDisplayCapturePicker();
        update();
    }

    if (m_mode == Mode::Selecting
        && activeRecordingAvailable()
        && event->button() == Qt::LeftButton
        && activeRecordingStopButtonRect().contains(event->position())) {
        stopActiveRecordingFromOverlay();
        event->accept();
        return;
    }

    if (m_mode == Mode::Selecting
        && m_startupTool != StartupTool::None
        && event->button() == Qt::RightButton) {
        leaveStartupTool();
        event->accept();
        return;
    }

    const bool startupPointerTool = m_startupTool == StartupTool::ColorPicker
        || m_startupTool == StartupTool::Ruler;
    if (m_mode == Mode::Selecting && startupPointerTool) {
        if (event->button() != Qt::LeftButton) {
            event->accept();
            return;
        }
        if (!m_frozenImageRect.contains(event->position())) {
            event->accept();
            return;
        }

        const QPointF imagePoint = clampImagePoint(selectingPointerImagePoint(event->position()));
        m_startupHoverImagePoint = imagePoint;
        m_startupHoverValid = true;
        if (m_startupTool == StartupTool::ColorPicker) {
            showStartupColorDialog(sampledImageColor(imagePoint),
                                   imageToWidget(imagePoint).toPoint());
            update();
            event->accept();
            return;
        }
        if (m_startupTool == StartupTool::Ruler) {
            m_startupRulerDragging = true;
            m_startupRulerHasMeasure = true;
            m_startupRulerStart = imagePoint;
            m_startupRulerEnd = imagePoint;
            update();
            event->accept();
            return;
        }
    }

    if (event->button() != Qt::LeftButton) {
        if (m_mode == Mode::Selecting) {
            if (event->button() == Qt::RightButton) {
                emit sessionCancelRequested();
                close();
                event->accept();
                return;
            }
            if (event->button() == Qt::MiddleButton) {
                enterFullscreenAnnotation(true);
                event->accept();
                return;
            }
        }
        if (event->button() == Qt::MiddleButton && imageNavigationAvailable() && m_frozenImageRect.contains(event->position())) {
            commitTextEditor();
            m_dragging = false;
            m_annotationSelectionBoxActive = false;
            m_imagePanning = true;
            m_imagePanStartWidget = event->position();
            m_imagePanStartCenter = m_imageCenterInitialized
                ? m_imageCenter
                : QPointF(m_frozenFrame.width() / 2.0, m_frozenFrame.height() / 2.0);
            updateCursor();
            event->accept();
            return;
        }
        if (event->button() == Qt::RightButton && m_mode == Mode::Editing) {
            setTool(Tool::Select);
            event->accept();
            return;
        }
        return;
    }

    const QPointF imagePoint = (m_mode == Mode::Selecting || canAdjustSelection())
        ? selectingPointerImagePoint(event->position())
        : widgetToImage(event->position());
    if (m_openWithPanel && m_openWithPanel->isVisible()
        && !m_openWithPanel->geometry().contains(event->pos())
        && (!m_actionToolbar || !m_actionToolbar->geometry().contains(event->pos()))
        && (!m_toolbar || !m_toolbar->geometry().contains(event->pos()))) {
        m_openWithPanel->hide();
    }
    if (m_extensionPanel && m_extensionPanel->isVisible()
        && !m_extensionPanel->geometry().contains(event->pos())
        && (!m_actionToolbar || !m_actionToolbar->geometry().contains(event->pos()))
        && (!m_toolbar || !m_toolbar->geometry().contains(event->pos()))) {
        m_extensionPanel->hide();
    }
    if (m_colorPalette && m_colorPalette->isVisible()
        && !m_colorPalette->geometry().contains(event->pos())) {
        m_colorPalette->hide();
    }
    if (m_shapeMarkerPopup && m_shapeMarkerPopup->isVisible()) {
        const QPoint localPos = event->pos();
        const bool overPopup = m_shapeMarkerPopup->geometry().contains(localPos);
        bool overButton = false;
        if (m_shapeMarkerToolbarButton) {
            const QRect buttonRect(m_shapeMarkerToolbarButton->mapTo(this, QPoint(0, 0)),
                                   m_shapeMarkerToolbarButton->size());
            overButton = buttonRect.contains(localPos);
        }
        if (!overPopup && !overButton) {
            hideShapeMarkerPopup();
            update();
        }
    }
    if (m_propertyColorDialogPanel && m_propertyColorDialogPanel->isVisible()
        && !m_propertyColorDialogPanel->geometry().contains(event->pos())
        && (!m_annotationPropertyPanel || !m_annotationPropertyPanel->geometry().contains(event->pos()))
        && (!m_toolbar || !m_toolbar->geometry().contains(event->pos()))) {
        m_propertyColorDialogPanel->hide();
    }
    if (m_propertyFontPanel && m_propertyFontPanel->isVisible()
        && !m_propertyFontPanel->geometry().contains(event->pos())
        && (!m_annotationPropertyPanel || !m_annotationPropertyPanel->geometry().contains(event->pos()))
        && (!m_toolbar || !m_toolbar->geometry().contains(event->pos()))) {
        m_propertyFontPanel->hide();
    }
    if (m_textEditor && m_textEditor->isVisible() && !m_textEditor->geometry().contains(event->pos())) {
        commitTextEditor();
    }

    if (m_mode == Mode::Selecting) {
        if (m_colorPalette) {
            m_colorPalette->hide();
        }
        m_selectionClickStart = imageToWidget(imagePoint);
        beginSelection(imagePoint);
        return;
    }

    if (!m_frozenImageRect.contains(event->position())) {
        return;
    }

    if (m_tool == Tool::Move && !m_fullscreenAnnotation) {
        m_selectionDrag = selectionDragAt(imagePoint);
        if (m_selectionDrag == SelectionDrag::None) {
            updateCursor();
            return;
        }
        m_dragging = true;
        m_selectionKeyboardAdjusting = false;
        m_startupHoverImagePoint = clampImagePoint(imagePoint);
        m_startupHoverValid = true;
        m_dragStart = imagePoint;
        m_selectionBeforeDrag = normalizedSelection();
        revealSelectionInfo();
        updateCursor();
        update();
        return;
    }

    if (m_tool == Tool::Select) {
        if (selectedAnnotationDeleteButtonRect().contains(event->position())) {
            deleteSelectedAnnotation();
            return;
        }

        const QVector<int> selectedIds = selectedAnnotationIds();
        if (selectedIds.size() > 1) {
            const SelectionDrag drag = selectedAnnotationsDragAt(imagePoint);
            if (drag != SelectionDrag::None) {
                beginAnnotationDrag(selectedIds.first(), drag, imagePoint);
                return;
            }
        } else if (m_selectedAnnotationId.has_value()) {
            const SelectionDrag drag = annotationDragAt(imagePoint, *m_selectedAnnotationId);
            if (drag != SelectionDrag::None) {
                beginAnnotationDrag(*m_selectedAnnotationId, drag, imagePoint);
                return;
            }
        }

        const std::optional<int> hitAnnotationId = annotationAt(imagePoint);
        if (hitAnnotationId.has_value()) {
            const SelectionDrag drag = annotationDragAt(imagePoint, *hitAnnotationId);
            setSelectedAnnotations({*hitAnnotationId});
            beginAnnotationDrag(*hitAnnotationId, drag == SelectionDrag::None ? SelectionDrag::Move : drag, imagePoint);
            updateAnnotationPropertyPanel();
        } else if (m_imageNavigationEnabled) {
            beginAnnotationSelectionBox(imagePoint);
            m_imageSelected = true;
        } else {
            beginAnnotationSelectionBox(imagePoint);
        }
        return;
    }

    if (m_tool == Tool::Text) {
        commitTextEditor();
        beginTextAnnotation(imagePoint);
        return;
    }

    if (m_tool == Tool::Number) {
        Annotation annotation;
        annotation.tool = Tool::Number;
        annotation.points.append(clampImagePoint(imagePoint));
        annotation.points.append(clampImagePoint(imagePoint));
        annotation.number = m_nextNumber;
        annotation.numberStyle = m_numberStyle;
        annotation.color = m_currentColor;
        annotation.width = m_numberWidth;
        m_dragging = true;
        m_dragStart = annotation.points.last();
        m_draft = annotation;
        update();
        return;
    }

    if (m_tool == Tool::Magnifier) {
        const QPointF sourceCenter = clampImagePoint(imagePoint);
        Annotation annotation;
        annotation.tool = Tool::Magnifier;
        annotation.points.append(sourceCenter);
        annotation.points.append(sourceCenter);
        annotation.rect = QRectF(sourceCenter, sourceCenter);
        annotation.color = m_currentColor;
        annotation.width = currentToolWidth();
        annotation.magnifierScale = m_magnifierScale;
        annotation.magnifierShape = m_magnifierShape;
        m_dragging = true;
        m_dragStart = sourceCenter;
        m_draft = annotation;
        update();
        return;
    }

    if (m_tool == Tool::Laser) {
        beginLaserStroke(imagePoint);
        return;
    }

    m_dragging = true;
    m_dragStart = imagePoint;
    Annotation annotation;
    annotation.tool = m_tool;
    annotation.color = m_currentColor;
    annotation.width = currentToolWidth();
    annotation.filled = m_shapeFilled;
    annotation.cornerRadius = m_tool == Tool::Rectangle ? m_rectangleCornerRadius : 0.0;
    if (m_tool == Tool::Marker) {
        annotation.filled = m_markerFilled;
    }
    annotation.arrowStyle = m_arrowStyle;
    annotation.rectangleStyle = m_rectangleStyle;
    annotation.markerShape = m_markerShape;
    annotation.fontFamily = m_textFontFamily;
    annotation.rotationDegrees = 0.0;
    annotation.highlighterStyle = m_tool == Tool::Highlighter ? m_highlighterStyle : HighlighterStyle::Freehand;
    if (m_tool == Tool::Pen
        || (m_tool == Tool::Highlighter && annotation.highlighterStyle == HighlighterStyle::Freehand)) {
        annotation.points.append(imagePoint);
    } else if (m_tool == Tool::Mosaic) {
        annotation.width = m_mosaicBlockSize;
        annotation.rect = QRectF(imagePoint, imagePoint);
        annotation.points.append(imagePoint);
        annotation.points.append(imagePoint);
    } else {
        annotation.rect = QRectF(imagePoint, imagePoint);
        annotation.points.append(imagePoint);
        annotation.points.append(imagePoint);
    }
    m_draft = annotation;
    update();
}
