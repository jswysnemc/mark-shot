#include "shot_window_module.h"
#include "shot_window_ruler_painter.h"
#include "shot_window_ruler_tool.h"

#include "selection_aspect/selection_aspect_ratio.h"

#include "debug_log.h"
#include "ui/transparency_background.h"
#include "selection_loupe.h"
#include "text_editor_box_chrome.h"

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
    // 1. 图片外区域使用透明棋盘格，仅影响编辑视图，不写入导出图片
    markshot::ui::paintTransparencyBackground(painter, event ? event->rect() : rect());
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
        // 1. 【画布】【视口内存】失效缓存不参与失败回退，先释放以免新旧整幅视口同时驻留
        if (!cacheHit) m_sharpViewportCache = {};
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
        // 聚光灯遮罩在所有标注之下，保留区域外的标注不会被压暗
        drawSpotlightOverlay(painter, widgetSelection, true);
        for (const Annotation &annotation : m_annotations) {
            if (m_editingTextAnnotationId.has_value() && annotation.id == *m_editingTextAnnotationId) {
                continue;
            }
            drawAnnotation(painter, annotation, true);
        }
        if (m_draft.has_value()) {
            drawAnnotation(painter, *m_draft, true);
        }
        ShotWindowRulerPainter(*this).drawLabels(painter, true);
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
        // 文字输入框的背景和虚线边框画在编辑器下方，编辑器本身透明；选择工具已画出选中框时只画背景
        const bool selectionFrameShown = m_tool != Tool::Text && !selectedAnnotationIds().isEmpty();
        paintTextEditorBox(painter, m_textEditor, annotationSizeScale(true), !selectionFrameShown);
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
            // 【截图选区】【控制点】8 个 6px 白色方块 + 青色描边，命中范围沿用 selectionDragAt
            painter.save();
            painter.setRenderHint(QPainter::Antialiasing, true);
            painter.setPen(QPen(QColor(45, 212, 191), 1.0));
            painter.setBrush(QColor(255, 255, 255));
            for (const QPointF &handle : selectionHandlePoints(widgetSelection)) {
                painter.drawRect(markshot::shot_popup::selectionHandleRect(handle));
            }
            painter.restore();
        }

        // 左上角尺寸 HUD 显示时不再重复绘制临时尺寸标签
        const bool sizeHudVisible = m_selectionSizePanel && m_selectionSizePanel->isVisible();
        const bool selectionInfoVisible = !sizeHudVisible
            && (m_selectionDrag != SelectionDrag::None
                || (m_showSelectionInfo && m_selectionInfoTimer.isValid() && m_selectionInfoTimer.elapsed() <= 1000));
        if (selectionInfoVisible) {
            QString sizeText = QStringLiteral("%1 x %2").arg(qRound(selection.width())).arg(qRound(selection.height()));
            // 锁定比例时在尺寸后标出比例，提醒拖动会保持比例
            for (const auto &preset : markshot::selection_aspect::aspectRatioPresets()) {
                if (m_selectionAspectRatio > 0.0 && std::abs(preset.ratio - m_selectionAspectRatio) < 1e-6) {
                    sizeText += QStringLiteral("  ") + preset.label;
                    break;
                }
            }
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
    ShotWindowRulerPainter(*this).drawHover(painter);
    drawSelectionAdjustmentOverlay(painter);
    drawStartupShortcutHint(painter);
    drawActiveRecordingStatus(painter);

    drawWheelPreview(painter);
    if (selectionPointerVisible()) {
        drawSelectionPointer(painter, imageToWidget(m_startupHoverImagePoint));
    }
    paintEditorWindowBorderIfEnabled(painter);
}

void ShotWindow::resizeEvent(QResizeEvent *)
{
    updateFrozenImageRect();
    updateDisplayCapturePickerGeometry();
    if (m_controls.m_colorPalette && m_controls.m_colorPalette->isVisible()) {
        updateColorPaletteGeometry(m_controls.m_colorPaletteAnchor);
    }
    if (m_controls.m_shapeMarkerPopup && m_controls.m_shapeMarkerPopup->isVisible()) {
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
            updateCursor();
            update();
        });
    }
    updateCursor();
    // 1. 【画布】【按需识别】文件编辑入口稍后切换标注模式，不为其预先分析整图选区
    if (m_mode == Mode::Selecting && !m_imageNavigationEnabled) initializeRegionDetection();
}

/// @brief 按当前工具开始选区或标注操作
/// @param event 鼠标按下事件
/// @return 无返回值
void ShotWindow::mousePressEvent(QMouseEvent *event)
{
    clearWheelPreview();

    // 1. 【标注】【图像平移】中键抓取期间只维护平移，其他按键不创建并行草稿
    if (m_imagePanning) {
        event->accept();
        return;
    }

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

    if (m_mode == Mode::Selecting && m_startupTool == StartupTool::ColorPicker) {
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
        showStartupColorDialog(sampledImageColor(imagePoint),
                               imageToWidget(imagePoint).toPoint());
        update();
        event->accept();
        return;
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
            cancelPointerInteraction();
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
    if (m_controls.m_openWithPanel && m_controls.m_openWithPanel->isVisible()
        && !m_controls.m_openWithPanel->geometry().contains(event->pos())
        && (!m_controls.m_actionToolbar || !m_controls.m_actionToolbar->geometry().contains(event->pos()))
        && (!m_controls.m_toolbar || !m_controls.m_toolbar->geometry().contains(event->pos()))) {
        m_controls.m_openWithPanel->hide();
    }
    if (m_controls.m_extensionPanel && m_controls.m_extensionPanel->isVisible()
        && !m_controls.m_extensionPanel->geometry().contains(event->pos())
        && (!m_controls.m_actionToolbar || !m_controls.m_actionToolbar->geometry().contains(event->pos()))
        && (!m_controls.m_toolbar || !m_controls.m_toolbar->geometry().contains(event->pos()))) {
        m_controls.m_extensionPanel->hide();
    }
    if (m_controls.m_colorPalette && m_controls.m_colorPalette->isVisible()
        && !m_controls.m_colorPalette->geometry().contains(event->pos())) {
        m_controls.m_colorPalette->hide();
    }
    if (m_controls.m_shapeMarkerPopup && m_controls.m_shapeMarkerPopup->isVisible()) {
        const QPoint localPos = event->pos();
        const bool overPopup = m_controls.m_shapeMarkerPopup->geometry().contains(localPos);
        bool overButton = false;
        if (m_controls.m_shapeMarkerToolbarButton) {
            const QRect buttonRect(m_controls.m_shapeMarkerToolbarButton->mapTo(this, QPoint(0, 0)),
                                   m_controls.m_shapeMarkerToolbarButton->size());
            overButton = buttonRect.contains(localPos);
        }
        if (!overPopup && !overButton) {
            hideShapeMarkerPopup();
            update();
        }
    }
    if (m_controls.m_propertyColorDialogPanel && m_controls.m_propertyColorDialogPanel->isVisible()
        && !m_controls.m_propertyColorDialogPanel->geometry().contains(event->pos())
        && (!m_controls.m_annotationPropertyPanel || !m_controls.m_annotationPropertyPanel->geometry().contains(event->pos()))
        && (!m_controls.m_toolbar || !m_controls.m_toolbar->geometry().contains(event->pos()))) {
        m_controls.m_propertyColorDialogPanel->hide();
    }
    if (m_controls.m_propertyFontPanel && m_controls.m_propertyFontPanel->isVisible()
        && !m_controls.m_propertyFontPanel->geometry().contains(event->pos())
        && (!m_controls.m_annotationPropertyPanel || !m_controls.m_annotationPropertyPanel->geometry().contains(event->pos()))
        && (!m_controls.m_toolbar || !m_controls.m_toolbar->geometry().contains(event->pos()))) {
        m_controls.m_propertyFontPanel->hide();
    }
    const bool committedTextEditor = m_textEditor && m_textEditor->isVisible()
        && !m_textEditor->geometry().contains(event->pos());
    if (committedTextEditor) {
        commitTextEditor();
    }

    if (m_mode == Mode::Selecting) {
        if (m_controls.m_colorPalette) {
            m_controls.m_colorPalette->hide();
        }
        m_selectionClickStart = imageToWidget(imagePoint);
        beginSelection(imagePoint);
        return;
    }

    if (m_tool == Tool::Select && selectedAnnotationDeleteButtonRect().contains(event->position())) {
        deleteSelectedAnnotation();
        return;
    }
    if (ShotWindowRulerTool(*this).press(event->position(), imagePoint)) {
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
        handleTextToolPress(imagePoint, committedTextEditor);
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
    annotation.arrowLineStyle = m_arrowLineStyle;
    annotation.rectangleStyle = m_rectangleStyle;
    annotation.mosaicStyle = m_mosaicStyle;
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

namespace {

constexpr bool FEATURE_EDITOR_WINDOW_BORDER_ENABLED = true;

}  // namespace

void ShotWindow::paintEditorWindowBorderIfEnabled(QPainter &painter) const
{
    if (!FEATURE_EDITOR_WINDOW_BORDER_ENABLED || !m_imageNavigationEnabled || width() < 2 || height() < 2) {
        return;
    }

    // 1. 【图片编辑】【窗口边框】1px 描边盖在画布最上层，截图遮罩不走这条路径
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, false);
    painter.setPen(Qt::NoPen);
    painter.setBrush(QColor(148, 163, 184));
    painter.fillRect(QRect(0, 0, width(), 1), painter.brush());
    painter.fillRect(QRect(0, height() - 1, width(), 1), painter.brush());
    painter.fillRect(QRect(0, 0, 1, height()), painter.brush());
    painter.fillRect(QRect(width() - 1, 0, 1, height()), painter.brush());
    painter.restore();
}
