#include "scroll/scroll_session_window_internal.h"

namespace markshot::scroll {

void ScrollSessionWindow::resizeEvent(QResizeEvent *event)
{
    QWidget::resizeEvent(event);
    const QRegion oldPaint = overlayPaintRegion();
    updatePreviewPanelVisibility();
    layoutOverlay();
    updateInputMask();
    syncPreviewWindowVisibility();
    const QRegion repaintRegion = oldPaint + overlayPaintRegion();
    if (!repaintRegion.isEmpty()) {
        update(repaintRegion);
    }
}

void ScrollSessionWindow::captureTick()
{
    if (m_paused || (m_panelOnlyWindow && m_axisDragging)) {
        return;
    }

    QImage frame;
    auto captureFrame = [&](const char *debugTag) {
        // Scroll capture needs non-interactive, repeatable frames. Request the
        // screencast path when possible and disable portal screenshot fallback
        // so a stalled stream fails instead of opening prompts during scrolling.
        CaptureRequest request;
        request.preferredOutputName = m_outputName;
        request.sourceGeometry = m_geometry;
        request.allOutputs = false;
        request.preferScreencast = true;
        request.allowInteractivePortal = false;
        request.allowPortalScreenshotFallback = false;

        const bool makePanelTransparentForCapture =
            !m_gnomeShellPreview
            && !m_layerShell
            && isVisible()
            && (m_previewPanelVisible || floatingDragHandleActive() || !framePaintRegion().isEmpty());
        if (makePanelTransparentForCapture) {
            // Plain Wayland fallback windows can overlap the capture rectangle.
            // Hide their controls for one compositor frame so the next backend
            // frame contains only the user-selected page area.
            const QRegion transparentRegion = overlayPaintRegion();
            m_panelTransparentForCapture = true;
            if (m_controlBar) {
                m_controlBar->hide();
            }
            updateInputMask();
            repaint(transparentRegion);
            QCoreApplication::processEvents(QEventLoop::ExcludeUserInputEvents);
            QThread::msleep(16);
            QCoreApplication::processEvents(QEventLoop::ExcludeUserInputEvents);
            request.minimumFrameTimeMs = QDateTime::currentMSecsSinceEpoch() + 16;
        }
        const CaptureResult result = captureScreenFrame(request);
        if (makePanelTransparentForCapture) {
            // Restore UI immediately after the backend has produced a frame.
            m_panelTransparentForCapture = false;
            if (m_controlBar) {
                layoutOverlay();
            }
            updateInputMask();
            repaint(overlayPaintRegion());
            QCoreApplication::processEvents(QEventLoop::ExcludeUserInputEvents);
        }
        if (result.image.isNull()) {
            m_paused = true;
            m_autoPausedForPreview = false;
            cancelScrollIdlePause();
            stopActiveScreencastCapture();
            m_statusText = MS_TR("Capture error");
            updatePreviewPanelVisibility();
            refreshControlLabels();
            updateGnomeShellPreview(true);
            logScrollDebug("%s-capture-error geom=%d,%d %dx%d output=%s error=%s",
                           debugTag,
                           request.sourceGeometry.x(), request.sourceGeometry.y(),
                           request.sourceGeometry.width(), request.sourceGeometry.height(),
                           request.preferredOutputName.toUtf8().constData(),
                           result.error.toUtf8().constData());
            update();
            return false;
        }

        Q_UNUSED(debugTag);
        frame = result.image.convertToFormat(QImage::Format_ARGB32_Premultiplied);
        const int scrubbedEdges = scrubCaptureFrameArtifacts(&frame);
        if (scrubbedEdges > 0) {
            logScrollDebug("%s-scrub-frame-artifacts edges=%d frame=%dx%d",
                           debugTag,
                           scrubbedEdges,
                           frame.width(),
                           frame.height());
        }
        return true;
    };

    if (frame.isNull() && !captureFrame("normal")) {
        return;
    }

    // Static duplicate frames usually mean the user has not scrolled yet. Skip
    // them so the stitcher only sees actual page movement.
    const QVector<std::uint8_t> signature = frameSignature(frame, kSignatureCols, kSignatureRows);
    if (!m_lastSignature.isEmpty() && isDuplicateSignature(m_lastSignature, signature)) {
        m_statusText = MS_TR("Waiting for scroll");
        logScrollDebug("skip-duplicate frame=%dx%d axis=%s",
                       frame.width(), frame.height(),
                       axisDebugName(m_stitcher.axis()));
        if (shouldHidePreviewWhileCapturing()) {
            scheduleScrollIdlePause();
        }
        updateGnomeShellPreview();
        update();
        return;
    }
    cancelScrollIdlePause();
    m_lastSignature = signature;
    dumpDebugFrame(frame, "candidate");

    // Stitcher owns geometric truth for the long image. The session window only
    // mirrors its result into status text, preview scrub position, and exports.
    const StitchResult outcome = m_stitcher.pushFrame(frame);
    const StitchStats stats = m_stitcher.stats();
    const int oldCapturePos = m_capturePos;
    const int oldCaptureLen = m_captureLen;

    switch (outcome.status) {
    case StitchStatus::FirstFrame:
        m_statusText = MS_TR("Capturing");
        break;
    case StitchStatus::Appended:
        m_lastAppend = outcome.added;
        m_statusText = MS_TR("Height %1 px").arg(stats.totalHeight);
        break;
    case StitchStatus::NoProgress:
        m_statusText = MS_TR("Waiting for scroll");
        break;
    case StitchStatus::NoMatch:
        m_statusText = MS_TR("No overlap match");
        break;
    }
    if (outcome.frameLength > 0) {
        m_capturePos = outcome.position;
        m_captureLen = outcome.frameLength;
    }
    if (outcome.status == StitchStatus::FirstFrame
        || outcome.status == StitchStatus::Appended
        || oldCapturePos != m_capturePos
        || oldCaptureLen != m_captureLen) {
        // GNOME shell preview is rendered out-of-process, so mark it dirty only
        // when the visible image or live viewport position actually changed.
        m_gnomePreviewImageDirty = true;
    }
    syncPreviewScroll(outcome);
    refreshControlLabels();
    updateGnomeShellPreview();
    logScrollDebug("tick status=%s edge=%s added=%d pos=%d frame_len=%d full_len=%d frames=%d "
                   "scrub=%d following=%d axis=%s",
                   statusDebugName(outcome.status), edgeDebugName(outcome.edge), outcome.added,
                   outcome.position, outcome.frameLength, stats.totalHeight, stats.frameCount,
                   m_scrubPos, m_following ? 1 : 0, axisDebugName(m_stitcher.axis()));
    update();
}

void ScrollSessionWindow::dumpDebugFrame(const QImage &frame, const char *tag)
{
    if (!markshot::debugEnabled() || frame.isNull() || m_debugFrameDumpCount >= 12) {
        return;
    }

    const QString safeTag = QString::fromLatin1(tag ? tag : "frame").remove(QLatin1Char('/'));
    const QString path = QDir::temp().filePath(
        QStringLiteral("mark-shot-scroll-%1-%2-%3.png")
            .arg(m_sessionId)
            .arg(m_debugFrameDumpCount, 2, 10, QLatin1Char('0'))
            .arg(safeTag));
    const bool saved = frame.save(path, "PNG");
    logScrollDebug("dump-frame index=%d saved=%d path=%s frame=%dx%d",
                   m_debugFrameDumpCount, saved ? 1 : 0,
                   path.toUtf8().constData(), frame.width(), frame.height());
    ++m_debugFrameDumpCount;
}

void ScrollSessionWindow::togglePause()
{
    if (m_autoPausedForPreview) {
        resumeAutoPausedCapture();
        return;
    }

    m_paused = !m_paused;
    if (!m_paused) {
        m_lastSignature.clear();
    } else {
        cancelScrollIdlePause();
    }
    m_autoPausedForPreview = false;
    updatePreviewPanelVisibility();
    refreshControlLabels();
    updateGnomeShellPreview(true);
    update();
}

void ScrollSessionWindow::toggleAxis()
{
    if (m_stitcher.axisLocked()) {
        return;
    }

    // Direction only switches before the capture has committed an orientation.
    const ScrollAxis next = m_stitcher.axis() == ScrollAxis::Horizontal
        ? ScrollAxis::Vertical
        : ScrollAxis::Horizontal;
    m_stitcher.setAxis(next);
    m_gnomePreviewImageDirty = true;
    updateInputMask();
    refreshControlLabels();
    updateGnomeShellPreview(true);
    update();
}

void ScrollSessionWindow::setPreviewPanelVisible(bool visible)
{
    if (m_previewPanelVisible == visible) {
        return;
    }

    const QRegion oldPaint = overlayPaintRegion();
    const bool hidingPreview = m_previewPanelVisible && !visible;
    m_previewPanelVisible = visible;
    if (hidingPreview && !m_panelOnlyWindow && !oldPaint.isEmpty()) {
        m_transientPaintMask += oldPaint;
        m_restoreMaskAfterPaint = true;
    }
    if (m_panelOnlyWindow) {
        updatePanelWindowGeometry();
    }
    layoutOverlay();
    updateInputMask();
    updateGnomeShellPreview(true);
    syncPreviewWindowVisibility();
    const QRegion repaintRegion = oldPaint + overlayPaintRegion();
    if (!repaintRegion.isEmpty()) {
        update(repaintRegion);
    } else {
        update();
    }
}

bool ScrollSessionWindow::shouldHidePreviewWhileCapturing() const
{
    return !m_paused
        && (m_uiConfig.hidePreviewDuringCapture
            || (shouldAvoidPreviewOverlapForCapture() && !previewPanelFitsAvailableSpace()));
}

void ScrollSessionWindow::updatePreviewPanelVisibility()
{
    setPreviewPanelVisible(!shouldHidePreviewWhileCapturing());
    syncPreviewWindowVisibility();
}

void ScrollSessionWindow::syncPreviewWindowVisibility()
{
    if (m_gnomeShellPreview || !m_layerShell) {
        return;
    }

    if (m_previewPanelVisible || floatingDragHandleActive() || !framePaintRegion().isEmpty()) {
        if (!isVisible()) {
            logScrollDebug("preview-window-show paused=%d auto_paused=%d",
                           m_paused ? 1 : 0,
                           m_autoPausedForPreview ? 1 : 0);
            show();
            raise();
        }
        return;
    }

    if (isVisible()) {
        logScrollDebug("preview-window-hide paused=%d auto_paused=%d",
                       m_paused ? 1 : 0,
                       m_autoPausedForPreview ? 1 : 0);
        hide();
    }
}

void ScrollSessionWindow::scheduleScrollIdlePause()
{
    if (!m_scrollIdleTimer || m_paused || !shouldHidePreviewWhileCapturing()) {
        return;
    }
    if (m_scrollIdleTimer->isActive()) {
        return;
    }
    logScrollDebug("idle-preview-timer-start timeout_ms=%d", kScrollIdlePauseMs);
    m_scrollIdleTimer->start(kScrollIdlePauseMs);
}

void ScrollSessionWindow::cancelScrollIdlePause()
{
    if (m_scrollIdleTimer) {
        m_scrollIdleTimer->stop();
    }
}

void ScrollSessionWindow::handleScrollIdleTimeout()
{
    if (m_paused || !shouldHidePreviewWhileCapturing()) {
        return;
    }

    m_paused = true;
    m_autoPausedForPreview = true;
    m_lastSignature.clear();
    m_statusText = MS_TR("Capture paused");
    logScrollDebug("idle-preview-timeout show_preview=1");
    updatePreviewPanelVisibility();
    refreshControlLabels();
    updateGnomeShellPreview(true);
    update();
}

void ScrollSessionWindow::resumeAutoPausedCapture()
{
    m_paused = false;
    m_autoPausedForPreview = false;
    m_lastSignature.clear();
    m_statusText = MS_TR("Capturing");
    cancelScrollIdlePause();
    updatePreviewPanelVisibility();
    refreshControlLabels();
    updateGnomeShellPreview(true);
    if (m_timer && !m_timer->isActive()) {
        m_timer->start();
    }
    update();
}

QRect ScrollSessionWindow::imageAreaRect() const
{
    const QRect panel = previewPanelRect();
    const int left = panel.left() + kPanelPadding;
    const int top = panel.top() + kPanelPadding + kStatusHeight + kPanelPadding;
    const int width = panel.width() - kPanelPadding * 2;
    // Below the image area sits the control bar, separated by a padding gap from
    // the image and from the panel edge.
    const int height = panel.height() - kStatusHeight - kControlBarHeight - kPanelPadding * 4;
    return QRect(left, top, width, std::max(0, height));
}

ScrollSessionWindow::PreviewLayout ScrollSessionWindow::computePreviewLayout() const
{
    return computePreviewLayout(imageAreaRect());
}

ScrollSessionWindow::PreviewLayout ScrollSessionWindow::computePreviewLayout(const QRect &area) const
{
    PreviewLayout layout;
    const QImage result = currentResult();
    if (result.isNull() || area.width() <= 0 || area.height() <= 0
        || result.width() <= 0 || result.height() <= 0) {
        return layout;
    }
    layout.valid = true;

    const bool horizontal = m_stitcher.axis() == ScrollAxis::Horizontal;
    constexpr int gap = 10;

    if (!horizontal) {
        // Vertical: the detail view is scaled to the column width; once the image
        // is taller than the area, the whole-image thumbnail appears to the right
        // (80/20 split).
        const qreal fullScale = static_cast<qreal>(area.width()) / result.width();
        layout.overflow =
            static_cast<int>(std::lround(result.height() * fullScale)) > area.height();
        if (!layout.overflow) {
            layout.detailRect = area;
        } else {
            const int leftW = (area.width() - gap) * 8 / 10;
            const int rightW = area.width() - gap - leftW;
            layout.detailRect = QRect(area.left(), area.top(), leftW, area.height());
            layout.globalRect = QRect(area.left() + leftW + gap, area.top(), rightW, area.height());
        }
        layout.detailScale = static_cast<qreal>(layout.detailRect.width()) / result.width();
        layout.longLen = result.height();
    } else {
        // Horizontal: the detail view is scaled to the column height; once the
        // image is wider than the area, the thumbnail appears below (80/20 split).
        const qreal fullScale = static_cast<qreal>(area.height()) / result.height();
        layout.overflow =
            static_cast<int>(std::lround(result.width() * fullScale)) > area.width();
        if (!layout.overflow) {
            layout.detailRect = area;
        } else {
            const int topH = (area.height() - gap) * 8 / 10;
            const int botH = area.height() - gap - topH;
            layout.detailRect = QRect(area.left(), area.top(), area.width(), topH);
            layout.globalRect = QRect(area.left(), area.top() + topH + gap, area.width(), botH);
        }
        layout.detailScale = static_cast<qreal>(layout.detailRect.height()) / result.height();
        layout.longLen = result.width();
    }

    const int detailLongPx = horizontal ? layout.detailRect.width() : layout.detailRect.height();
    layout.viewportLen = std::min(layout.longLen,
        std::max(1, static_cast<int>(std::lround(detailLongPx / layout.detailScale))));
    layout.maxScrub = std::max(0, layout.longLen - layout.viewportLen);
    return layout;
}

}  // namespace markshot::scroll
