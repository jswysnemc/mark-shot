#include "shot_window_module.h"

#include "debug_log.h"

namespace cfg = markshot::config;
namespace shortcuts = markshot::shortcut;
using namespace markshot::shot;

/**
 * 生成启动阶段快捷键提示项。
 * @return 当前选区状态下需要显示的提示项列表。
 */
QVector<markshot::startup_hint::ShortcutHintItem> ShotWindow::startupShortcutHintItems() const
{
    using markshot::startup_hint::InputIcon;

    if (m_mode != Mode::Selecting || hasUsableSelection()) {
        return {};
    }

    if (m_startupTool == StartupTool::CodeScanner) {
        return {
            {MS_TR("Drag"), MS_TR("Select code region"), InputIcon::Mouse},
            {MS_TR("Right/Esc"), MS_TR("Return to selection"), InputIcon::Mouse},
        };
    }

    if (recordingModeForStartupTool(m_startupTool).has_value()) {
        return {
            {MS_TR("Drag"), MS_TR("Select recording region"), InputIcon::Mouse},
            {MS_TR("Right/Esc"), MS_TR("Return to selection"), InputIcon::Mouse},
        };
    }

    if (m_startupTool != StartupTool::None) {
        return {};
    }

    auto shortcutTextOr = [](const QKeySequence &sequence, const QString &fallback) {
        const QString text = sequence.toString(QKeySequence::NativeText);
        return text.isEmpty() ? fallback : text;
    };

    QVector<markshot::startup_hint::ShortcutHintItem> items = {
        {MS_TR("Drag"), MS_TR("Select screenshot region"), InputIcon::Mouse},
        {shortcutTextOr(m_startupColorPickerShortcut, QStringLiteral("C")), MS_TR("Pick color"), InputIcon::Keyboard},
        {shortcutTextOr(m_startupRulerShortcut, QStringLiteral("R")), MS_TR("Measure size"), InputIcon::Keyboard},
        {shortcutTextOr(m_startupCodeScannerShortcut, QStringLiteral("Q")), MS_TR("Scan QR or barcode"), InputIcon::Keyboard},
        {shortcutTextOr(m_startupDisplayCaptureShortcut, QStringLiteral("D")), MS_TR("Quick display capture"), InputIcon::Keyboard},
    };
    if (!activeRecordingAvailable()) {
        items.append({shortcutTextOr(m_startupGifRecorderShortcut, QStringLiteral("G")),
                      MS_TR("Record GIF"),
                      InputIcon::Keyboard});
        items.append({shortcutTextOr(m_startupVideoRecorderShortcut, QStringLiteral("V")),
                      MS_TR("Record video"),
                      InputIcon::Keyboard});
    }
    items.append({MS_TR("Middle"), MS_TR("Toggle fullscreen annotation"), InputIcon::Wheel});
    items.append({MS_TR("Right/Esc"), MS_TR("Cancel"), InputIcon::Mouse});
    return items;
}

/**
 * 根据鼠标位置更新启动提示面板停靠位置。
 * @param pointer 当前鼠标位置。
 * @return 停靠位置发生变化时返回 true。
 */
bool ShotWindow::updateStartupShortcutHintAnchor(QPointF pointer)
{
    const QVector<markshot::startup_hint::ShortcutHintItem> items = startupShortcutHintItems();
    if (items.isEmpty()) {
        m_startupHintAnchor = markshot::startup_hint::PanelAnchor::BottomLeft;
        return false;
    }

    const markshot::startup_hint::PanelAnchor nextAnchor =
        markshot::startup_hint::preferredAnchor(pointer, items, size());
    if (nextAnchor == m_startupHintAnchor) {
        return false;
    }

    m_startupHintAnchor = nextAnchor;
    return true;
}

/**
 * 绘制启动阶段快捷键提示面板。
 * @param painter 当前绘图对象。
 */
void ShotWindow::drawStartupShortcutHint(QPainter &painter) const
{
    const QVector<markshot::startup_hint::ShortcutHintItem> items = startupShortcutHintItems();
    if (items.isEmpty()) {
        return;
    }

    const markshot::startup_hint::PanelLayout layout =
        markshot::startup_hint::layoutPanel(items, size(), m_startupHintAnchor);
    markshot::startup_hint::drawPanel(painter, items, layout);
}

void ShotWindow::drawStartupRuler(QPainter &painter) const
{
    if (!m_startupHoverValid && !m_startupRulerHasMeasure && !m_startupRulerDragging) {
        return;
    }

    painter.save();
    painter.setFont(markshot::theme::uiFont(10, QFont::DemiBold));
    painter.setPen(QPen(QColor(45, 212, 191, 150), 1.0, Qt::DashLine));
    auto clampFloatingRect = [this](QRectF rect) {
        if (rect.right() > width() - 8.0) {
            rect.moveRight(width() - 8.0);
        }
        if (rect.left() < 8.0) {
            rect.moveLeft(8.0);
        }
        if (rect.bottom() > height() - 8.0) {
            rect.moveBottom(height() - 8.0);
        }
        if (rect.top() < 8.0) {
            rect.moveTop(8.0);
        }
        return rect;
    };

    std::optional<QRectF> hoverLabelRect;
    if (m_startupHoverValid) {
        const QPointF hover = imageToWidget(m_startupHoverImagePoint);
        painter.drawLine(QPointF(m_frozenImageRect.left(), hover.y()), QPointF(m_frozenImageRect.right(), hover.y()));
        painter.drawLine(QPointF(hover.x(), m_frozenImageRect.top()), QPointF(hover.x(), m_frozenImageRect.bottom()));

        const QColor color = sampledImageColor(m_startupHoverImagePoint);
        const QString hoverText = QStringLiteral("x %1  y %2  %3")
                                      .arg(qRound(m_startupHoverImagePoint.x()))
                                      .arg(qRound(m_startupHoverImagePoint.y()))
                                      .arg(colorHexRgb(color));
        const QFontMetrics metrics(painter.font());
        QRectF hoverLabel(hover.x() + 14.0,
                          hover.y() + 14.0,
                          metrics.horizontalAdvance(hoverText) + 18.0,
                          metrics.height() + 10.0);
        if (hoverLabel.right() > width() - 8.0) {
            hoverLabel.moveRight(hover.x() - 14.0);
        }
        if (hoverLabel.bottom() > height() - 8.0) {
            hoverLabel.moveBottom(hover.y() - 14.0);
        }
        hoverLabel = clampFloatingRect(hoverLabel);
        hoverLabelRect = hoverLabel;
        drawRoundedLabel(painter, hoverLabel, hoverText, QColor(8, 13, 19, 225));
    }

    if (!m_startupRulerHasMeasure && !m_startupRulerDragging) {
        painter.restore();
        return;
    }

    const QRectF imageRect = normalizedRect(m_startupRulerStart, m_startupRulerEnd);
    if (imageRect.width() < 1.0 && imageRect.height() < 1.0) {
        painter.restore();
        return;
    }

    const QRectF widgetRect = imageRectToWidget(imageRect);
    painter.setPen(QPen(QColor(45, 212, 191), 2.0));
    painter.setBrush(QColor(45, 212, 191, 26));
    painter.drawRoundedRect(widgetRect, 3.0, 3.0);

    const int widthPx = qRound(imageRect.width());
    const int heightPx = qRound(imageRect.height());
    const qreal diagonal = std::hypot(imageRect.width(), imageRect.height());
    const QString info = QStringLiteral("%1 x %2 px   diag %3 px   area %4 px")
                             .arg(widthPx)
                             .arg(heightPx)
                             .arg(qRound(diagonal))
                             .arg(widthPx * heightPx);
    const QFontMetrics metrics(painter.font());
    const QSizeF infoSize(metrics.horizontalAdvance(info) + 20.0, metrics.height() + 10.0);
    constexpr qreal kRulerFloatingGap = 8.0;
    constexpr qreal kRulerMajorTickLength = 13.0;
    constexpr qreal kRulerMinorTickLength = 6.0;
    constexpr qreal kRulerTopLabelOffset = 30.0;
    constexpr qreal kRulerTopLabelWidth = 68.0;
    constexpr qreal kRulerTickLabelHeight = 16.0;
    constexpr qreal kRulerLeftLabelOffset = 62.0;
    constexpr qreal kRulerLeftLabelWidth = 48.0;
    constexpr qreal kRulerTopScaleGap = kRulerTopLabelOffset + kRulerFloatingGap;
    constexpr qreal kRulerBottomScaleGap = kRulerMajorTickLength + kRulerFloatingGap;
    constexpr qreal kRulerRightScaleGap = kRulerMajorTickLength + kRulerFloatingGap;
    constexpr qreal kRulerLeftScaleGap = kRulerLeftLabelOffset + kRulerFloatingGap;
    const QVector<QRectF> infoCandidates = {
        QRectF(QPointF(widgetRect.left(), widgetRect.bottom() + kRulerBottomScaleGap), infoSize),
        QRectF(QPointF(widgetRect.left(), widgetRect.top() - infoSize.height() - kRulerTopScaleGap), infoSize),
        QRectF(QPointF(widgetRect.right() + kRulerRightScaleGap, widgetRect.top()), infoSize),
        QRectF(QPointF(widgetRect.left() - infoSize.width() - kRulerLeftScaleGap, widgetRect.top()), infoSize),
    };
    const QRectF rulerScaleArea = widgetRect.adjusted(-kRulerLeftScaleGap,
                                                      -kRulerTopScaleGap,
                                                      kRulerRightScaleGap,
                                                      kRulerBottomScaleGap);
    auto overlapsInfoObstacle = [&](const QRectF &rect) {
        if (rect.intersects(rulerScaleArea)) {
            return true;
        }
        if (!hoverLabelRect.has_value()) {
            return false;
        }
        return rect.intersects(hoverLabelRect->adjusted(-kRulerFloatingGap,
                                                        -kRulerFloatingGap,
                                                        kRulerFloatingGap,
                                                        kRulerFloatingGap));
    };

    QRectF infoRect = clampFloatingRect(infoCandidates.first());
    for (const QRectF &candidate : infoCandidates) {
        const QRectF clamped = clampFloatingRect(candidate);
        if (!overlapsInfoObstacle(clamped)) {
            infoRect = clamped;
            break;
        }
    }
    drawRoundedLabel(painter, infoRect, info);

    const int stepX = rulerTickStep(imageRect.width());
    const int stepY = rulerTickStep(imageRect.height());
    const int minorX = std::max(1, stepX / 5);
    const int minorY = std::max(1, stepY / 5);
    painter.setPen(QPen(QColor(204, 251, 241, 230), 1.0));
    auto drawXTick = [&](int tick, bool major) {
        if (imageRect.width() <= 0.0) {
            return;
        }
        const qreal x = widgetRect.left() + tick * widgetRect.width() / imageRect.width();
        const qreal length = major ? kRulerMajorTickLength : kRulerMinorTickLength;
        painter.drawLine(QPointF(x, widgetRect.top()), QPointF(x, widgetRect.top() - length));
        painter.drawLine(QPointF(x, widgetRect.bottom()), QPointF(x, widgetRect.bottom() + length));
        if (major) {
            const QString text = QString::number(tick);
            painter.drawText(QRectF(x - kRulerTopLabelWidth / 2.0,
                                    widgetRect.top() - kRulerTopLabelOffset,
                                    kRulerTopLabelWidth,
                                    kRulerTickLabelHeight),
                             Qt::AlignCenter,
                             text);
        }
    };
    auto drawYTick = [&](int tick, bool major) {
        if (imageRect.height() <= 0.0) {
            return;
        }
        const qreal y = widgetRect.top() + tick * widgetRect.height() / imageRect.height();
        const qreal length = major ? kRulerMajorTickLength : kRulerMinorTickLength;
        painter.drawLine(QPointF(widgetRect.left(), y), QPointF(widgetRect.left() - length, y));
        painter.drawLine(QPointF(widgetRect.right(), y), QPointF(widgetRect.right() + length, y));
        if (major) {
            const QString text = QString::number(tick);
            painter.drawText(QRectF(widgetRect.left() - kRulerLeftLabelOffset,
                                    y - kRulerTickLabelHeight / 2.0,
                                    kRulerLeftLabelWidth,
                                    kRulerTickLabelHeight),
                             Qt::AlignRight | Qt::AlignVCenter,
                             text);
        }
    };

    for (int x = 0; x <= widthPx; x += minorX) {
        drawXTick(x, x % stepX == 0);
    }
    for (int y = 0; y <= heightPx; y += minorY) {
        drawYTick(y, y % stepY == 0);
    }

    painter.restore();
}

void ShotWindow::drawStartupToolOverlay(QPainter &painter)
{
    if (m_startupTool == StartupTool::ColorPicker) {
        drawStartupColorLoupe(painter, m_startupHoverImagePoint);
    } else if (m_startupTool == StartupTool::Ruler) {
        drawStartupRuler(painter);
    }
}

void ShotWindow::paintEvent(QPaintEvent *)
{
    QPainter painter(this);
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

        const QPointF imagePoint = clampImagePoint(widgetToImage(event->position()));
        m_startupHoverImagePoint = imagePoint;
        m_startupHoverValid = true;
        if (m_startupTool == StartupTool::ColorPicker) {
            showStartupColorDialog(sampledImageColor(imagePoint), event->pos());
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

    const QPointF imagePoint = widgetToImage(event->position());
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
        update();
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
        m_selectionClickStart = event->position();
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
    annotation.arrowStyle = m_arrowStyle;
    annotation.rectangleStyle = m_rectangleStyle;
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
